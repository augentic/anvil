pub mod agent_teams;

use crate::context::Context;
use crate::finding::{Check, Finding};

pub use agent_teams::AgentTeamsCheck;

// Predicate modules (`adapter`, `brief`, `skill_frontmatter`, …) land as sibling
// files under `src/check/` in RFC-5 Changes 3–14.

/// Run every registered check predicate sequentially.
pub fn run(ctx: &Context) -> Vec<Finding> {
    let checks: [&dyn Check; 1] = [&AgentTeamsCheck];
    let mut findings = Vec::new();

    for check in checks {
        findings.extend(check.run(ctx));
    }

    findings
}
