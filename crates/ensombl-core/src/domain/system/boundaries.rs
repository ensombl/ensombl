//! Architecture rules: dependencies should point inward (interface →
//! application → domain), data access goes through the application layer, and
//! features don't reach into each other's data.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::model::{InteractionKind, Layer, Role, SystemModel, SystemNode};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Violation {
    pub interaction: String,
    pub rule: String,
    pub message: String,
}

pub const RULES: &[(&str, &str)] = &[
    ("outer-layer", "Depends on an outer layer"),
    (
        "skip-application",
        "Talks to data directly, skipping the application layer",
    ),
    ("domain-data", "Domain code depends on data access"),
    (
        "cross-feature-data",
        "Reaches into another feature's data access",
    ),
];

fn is_entry_role(role: Role) -> bool {
    matches!(
        role,
        Role::Controller | Role::Gateway | Role::Job | Role::Router | Role::Middleware
    )
}

fn violated(from: &SystemNode, to: &SystemNode) -> Option<&'static str> {
    // Unclassified code has a guessed layer; don't accuse it.
    if from.role == Role::Other || to.role == Role::Other {
        return None;
    }
    let data_like =
        to.layer == Layer::Data || (to.layer == Layer::External && to.name == "Database");
    if is_entry_role(from.role) && data_like {
        return Some("skip-application");
    }
    if from.layer == Layer::Domain && data_like {
        return Some("domain-data");
    }
    if to.layer < from.layer && to.layer != Layer::External && from.layer != Layer::External {
        return Some("outer-layer");
    }
    if to.role == Role::Repository
        && from.feature != to.feature
        && from.package == to.package
        && from.role != Role::Repository
    {
        return Some("cross-feature-data");
    }
    None
}

/// Rule violations among calls and injections, minus `disabled` rule ids.
pub fn check(model: &SystemModel, disabled: &[String]) -> Vec<Violation> {
    let nodes: HashMap<&str, &SystemNode> =
        model.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    model
        .interactions
        .iter()
        .filter(|i| matches!(i.kind, InteractionKind::Call | InteractionKind::Inject))
        .filter_map(|i| {
            let rule = violated(nodes.get(i.from.as_str())?, nodes.get(i.to.as_str())?)?;
            if disabled.iter().any(|d| d == rule) {
                return None;
            }
            let message = RULES.iter().find(|r| r.0 == rule).map(|r| r.1)?;
            Some(Violation {
                interaction: i.id.clone(),
                rule: rule.into(),
                message: message.into(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::build::tests::{NEST, system};
    use super::*;

    #[test]
    fn layered_code_passes() {
        assert!(check(&system(NEST), &[]).is_empty());
    }

    #[test]
    fn controllers_reaching_data_and_cross_feature_access_are_flagged() {
        let m = system(&[
            (
                "src/claims/claims.controller.ts",
                "import { ClaimsRepository } from './claims.repository';\nimport { BillingRepository } from '../billing/billing.repository';\n@Controller('c')\nexport class ClaimsController { constructor(private repo: ClaimsRepository) {} @Get() a() { return this.repo.all(); } }\n@Injectable()\nexport class ClaimsService { constructor(private billing: BillingRepository) {} run() { this.billing.charge(); } }",
            ),
            (
                "src/claims/claims.repository.ts",
                "@Injectable()\nexport class ClaimsRepository { all() {} }",
            ),
            (
                "src/billing/billing.repository.ts",
                "@Injectable()\nexport class BillingRepository { charge() {} }",
            ),
        ]);
        let mut rules: Vec<String> = check(&m, &[]).into_iter().map(|v| v.rule).collect();
        rules.sort();
        assert_eq!(rules, ["cross-feature-data", "skip-application"]);
        assert_eq!(check(&m, &["skip-application".into()]).len(), 1);
    }
}
