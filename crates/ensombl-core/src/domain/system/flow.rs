//! Traces what happens from an entry point: the member-level call chain across
//! components, through events and HTTP, like a sequence diagram.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use ts_rs::TS;

use super::model::{InteractionKind, MemberCall, Site, SystemModel};

const MAX_STEPS: usize = 400;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FlowStep {
    /// Nesting depth, 0 for calls made by the entry point itself.
    pub depth: u32,
    pub from: String,
    pub to: String,
    pub label: String,
    pub kind: InteractionKind,
    pub is_async: bool,
    pub site: Site,
}

type Key = (String, Option<String>);

/// Steps in call order, depth-first, stopping at `max_depth` and at cycles.
pub fn trace(model: &SystemModel, entry_id: &str, max_depth: u32) -> Vec<FlowStep> {
    let Some(entry) = model.entry_points.iter().find(|e| e.id == entry_id) else {
        return vec![];
    };
    let owner: HashMap<&str, &str> = model
        .nodes
        .iter()
        .flat_map(|n| {
            n.symbol_ids
                .iter()
                .map(move |s| (s.as_str(), n.id.as_str()))
        })
        .collect();
    let mut by_caller: HashMap<Key, Vec<&MemberCall>> = HashMap::new();
    for call in &model.calls {
        if call.kind == InteractionKind::Inject {
            continue;
        }
        by_caller
            .entry((call.from_symbol.clone(), call.from_member.clone()))
            .or_default()
            .push(call);
    }
    for calls in by_caller.values_mut() {
        calls.sort_by_key(|c| c.site.line);
    }

    let mut steps = vec![];
    let mut visited = HashSet::new();
    let start = (entry.symbol_id.clone(), entry.member.clone());
    visited.insert(start.clone());
    walk(
        &by_caller,
        &owner,
        &start,
        &entry.node,
        0,
        max_depth,
        &mut visited,
        &mut steps,
    );
    steps
}

#[allow(clippy::too_many_arguments)]
fn walk(
    by_caller: &HashMap<Key, Vec<&MemberCall>>,
    owner: &HashMap<&str, &str>,
    at: &Key,
    at_node: &str,
    depth: u32,
    max_depth: u32,
    visited: &mut HashSet<Key>,
    steps: &mut Vec<FlowStep>,
) {
    if depth >= max_depth {
        return;
    }
    let Some(calls) = by_caller.get(at) else {
        return;
    };
    for call in calls {
        if steps.len() >= MAX_STEPS {
            return;
        }
        steps.push(FlowStep {
            depth,
            from: at_node.to_string(),
            to: call.to_node.clone(),
            label: call.label.clone(),
            kind: call.kind,
            is_async: matches!(call.kind, InteractionKind::Emits | InteractionKind::Handles),
            site: call.site.clone(),
        });
        let Some(symbol) = &call.to_symbol else {
            continue;
        };
        let next = (symbol.clone(), call.to_member.clone());
        if visited.insert(next.clone()) {
            let node = owner.get(symbol.as_str()).copied().unwrap_or(&call.to_node);
            walk(
                by_caller,
                owner,
                &next,
                node,
                depth + 1,
                max_depth,
                visited,
                steps,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::build::tests::{NEST, system};
    use super::*;

    #[test]
    fn traces_a_route_through_services_events_and_handlers() {
        let m = system(NEST);
        let entry = m
            .entry_points
            .iter()
            .find(|e| e.label == "POST /claims")
            .unwrap();
        let short = |id: &str| id.rsplit(['#', ':']).next().unwrap().to_string();
        let steps: Vec<String> = trace(&m, &entry.id, 6)
            .iter()
            .map(|s| {
                format!(
                    "{} {} -> {}: {}",
                    s.depth,
                    short(&s.from),
                    short(&s.to),
                    s.label
                )
            })
            .collect();
        assert_eq!(
            steps,
            [
                "0 ClaimsController -> ClaimsService: submit",
                "1 ClaimsService -> ClaimsService: validate",
                "1 ClaimsService -> ClaimsRepository: save",
                "2 ClaimsRepository -> Database: Claim.save",
                "1 ClaimsService -> claim.submitted: claim.submitted",
                "2 claim.submitted -> NotifierListener: notify",
                "1 ClaimsService -> api.carrier.com: POST https://api.carrier.com/v1/claims",
            ]
        );
    }

    #[test]
    fn unknown_entries_and_depth_limits() {
        let m = system(NEST);
        assert!(trace(&m, "nope", 6).is_empty());
        let entry = &m
            .entry_points
            .iter()
            .find(|e| e.label == "POST /claims")
            .unwrap()
            .id;
        assert_eq!(trace(&m, entry, 1).len(), 1);
    }
}
