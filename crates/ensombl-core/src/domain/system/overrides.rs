//! The user's corrections to the detected system, stored in the repo as
//! `.crm/architecture.json`: names, descriptions, regrouping, merges, hidden
//! components and disabled rules.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::model::{Interaction, Layer, Role, SystemModel};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ComponentOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub feature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub layer: Option<Layer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub role: Option<Role>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
}

impl ComponentOverride {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Several detected components shown as one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Merge {
    pub id: String,
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ArchitectureFile {
    pub components: BTreeMap<String, ComponentOverride>,
    pub merges: Vec<Merge>,
    pub disabled_rules: Vec<String>,
}

impl ArchitectureFile {
    /// Merges `ids` into one component, flattening existing merges among them.
    pub fn merge(&mut self, ids: &[String], name: &str) -> String {
        let mut members = vec![];
        for id in ids {
            match self.merges.iter().position(|m| &m.id == id) {
                Some(i) => members.extend(self.merges.remove(i).members),
                None => members.push(id.clone()),
            }
        }
        members.dedup();
        let slug: String = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let mut id = format!("merge:{slug}");
        let mut n = 2;
        while self.merges.iter().any(|m| m.id == id) {
            id = format!("merge:{slug}-{n}");
            n += 1;
        }
        self.merges.push(Merge {
            id: id.clone(),
            name: name.into(),
            members,
        });
        id
    }

    pub fn split(&mut self, id: &str) {
        self.merges.retain(|m| m.id != id);
        self.components.remove(id);
    }

    pub fn set_override(&mut self, id: &str, value: ComponentOverride) {
        if value.is_empty() {
            self.components.remove(id);
        } else {
            self.components.insert(id.into(), value);
        }
    }
}

/// The detected model with the user's corrections applied.
pub fn apply(model: &SystemModel, file: &ArchitectureFile) -> SystemModel {
    let mut out = model.clone();
    let mut alias: HashMap<String, String> = HashMap::new();

    for merge in &file.merges {
        let members: Vec<_> = out
            .nodes
            .iter()
            .filter(|n| merge.members.contains(&n.id))
            .cloned()
            .collect();
        let Some(first) = members.first() else {
            continue;
        };
        let mut node = first.clone();
        node.id = merge.id.clone();
        node.name = merge.name.clone();
        node.symbol_ids = members.iter().flat_map(|m| m.symbol_ids.clone()).collect();
        node.merged = members.iter().map(|m| m.id.clone()).collect();
        node.description = None;
        for m in &members {
            alias.insert(m.id.clone(), merge.id.clone());
        }
        out.nodes.retain(|n| !merge.members.contains(&n.id));
        out.nodes.push(node);
    }

    if !alias.is_empty() {
        let rename = |id: &String| alias.get(id).cloned().unwrap_or_else(|| id.clone());
        let mut grouped: BTreeMap<String, Interaction> = BTreeMap::new();
        for mut i in std::mem::take(&mut out.interactions) {
            i.from = rename(&i.from);
            i.to = rename(&i.to);
            if i.from == i.to {
                continue;
            }
            i.id = format!("{}->{}:{:?}", i.from, i.to, i.kind);
            match grouped.get_mut(&i.id) {
                Some(existing) => {
                    existing.count += i.count;
                    for m in i.members {
                        if !existing.members.contains(&m) {
                            existing.members.push(m);
                        }
                    }
                    existing.sites.extend(i.sites);
                    existing.sites.truncate(8);
                }
                None => {
                    grouped.insert(i.id.clone(), i);
                }
            }
        }
        out.interactions = grouped.into_values().collect();
        for e in &mut out.entry_points {
            e.node = rename(&e.node);
        }
        for c in &mut out.calls {
            c.to_node = rename(&c.to_node);
        }
    }

    for node in &mut out.nodes {
        let Some(o) = file.components.get(&node.id) else {
            continue;
        };
        if let Some(name) = &o.name {
            node.name = name.clone();
        }
        if o.description.is_some() {
            node.description = o.description.clone();
        }
        if let Some(feature) = &o.feature {
            node.feature = feature.clone();
        }
        if let Some(role) = o.role {
            node.role = role;
        }
        if let Some(layer) = o.layer {
            node.layer = layer;
        }
        node.hidden = o.hidden;
    }
    out.nodes.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

#[cfg(test)]
mod tests {
    use super::super::build::tests::{NEST, interaction, node, system};
    use super::*;

    #[test]
    fn overrides_rename_describe_and_hide() {
        let m = system(NEST);
        let service = node(&m, "#ClaimsService").id.clone();
        let mut file = ArchitectureFile::default();
        file.set_override(
            &service,
            ComponentOverride {
                name: Some("Claims".into()),
                description: Some("Owns the claim lifecycle".into()),
                layer: Some(Layer::Domain),
                hidden: true,
                ..Default::default()
            },
        );
        let applied = apply(&m, &file);
        let n = node(&applied, "#ClaimsService");
        assert_eq!(n.name, "Claims");
        assert_eq!(n.description.as_deref(), Some("Owns the claim lifecycle"));
        assert_eq!(n.layer, Layer::Domain);
        assert!(n.hidden);

        file.set_override(&service, ComponentOverride::default());
        assert!(file.components.is_empty());
    }

    #[test]
    fn merging_combines_nodes_and_their_interactions() {
        let m = system(NEST);
        let service = node(&m, "#ClaimsService").id.clone();
        let repo = node(&m, "#ClaimsRepository").id.clone();
        let mut file = ArchitectureFile::default();
        let id = file.merge(&[service.clone(), repo.clone()], "Claims core");
        assert_eq!(id, "merge:claims-core");

        let applied = apply(&m, &file);
        let merged = node(&applied, "merge:claims-core");
        assert_eq!(merged.merged, [repo.clone(), service.clone()]);
        assert!(!applied.nodes.iter().any(|n| n.id == service));
        // The service→repository call is now internal; the rest points at the merge.
        assert!(!applied.interactions.iter().any(|i| i.from == i.to));
        assert_eq!(
            interaction(&applied, "#ClaimsController", "merge:claims-core").members,
            ["find", "submit"]
        );
        assert!(
            interaction(&applied, "merge:claims-core", "ext:Database")
                .members
                .contains(&"Claim.save".to_string())
        );

        // Merging a merge flattens it; splitting restores the parts.
        let again = file.merge(
            &[id.clone(), node(&m, "#NotifierListener").id.clone()],
            "Claims core",
        );
        assert_eq!(file.merges.len(), 1);
        assert_eq!(file.merges[0].members.len(), 3);
        file.split(&again);
        assert!(file.merges.is_empty());
    }

    #[test]
    fn file_round_trips_compactly() {
        let mut file = ArchitectureFile::default();
        file.set_override(
            "a",
            ComponentOverride {
                description: Some("x".into()),
                ..Default::default()
            },
        );
        let json = serde_json::to_string(&file).unwrap();
        assert_eq!(
            json,
            r#"{"components":{"a":{"description":"x"}},"merges":[],"disabledRules":[]}"#
        );
        assert_eq!(
            serde_json::from_str::<ArchitectureFile>(&json).unwrap(),
            file
        );
        assert_eq!(
            serde_json::from_str::<ArchitectureFile>("{}").unwrap(),
            ArchitectureFile::default()
        );
    }
}
