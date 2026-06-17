//! Hardcoded rule registry — the table of representative rules keyed
//! by brief id, plus the cross-brief rules. Semantic rules declare
//! `check: None` and are materialised as `Deferred` by construction.

use crate::validate::Rule;

mod contract_rules;
mod cross;
mod design;
mod proposal;
mod specs;
mod tasks;

pub use cross::cross_rules;

/// Return the registered rules for `brief_id`. Unknown ids return `&[]`.
#[must_use]
pub fn rules_for(brief_id: &str) -> &'static [Rule] {
    match brief_id {
        "proposal" => proposal::PROPOSAL_RULES,
        "specs" => specs::SPECS_RULES,
        "design" => design::DESIGN_RULES,
        "tasks" => tasks::TASKS_RULES,
        "contracts" => contract_rules::CONTRACTS_RULES,
        _ => &[],
    }
}
