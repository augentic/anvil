use crate::context::Context;
use crate::finding::{Check, Finding};

// Predicate modules (`adapter`, `brief`, `skill_frontmatter`, …) land as sibling
// files under `src/check/` in RFC-5 Changes 3–14.

/// Run every registered check predicate sequentially.
pub fn run(ctx: &Context) -> Vec<Finding> {
    let checks: [&dyn Check; 0] = [];
    let mut findings = Vec::new();

    for check in checks {
        findings.extend(check.run(ctx));
    }

    let _ = ctx;
    findings
}
