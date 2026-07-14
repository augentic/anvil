//! Proposal-brief rules.

use crate::validate::{BriefContext, Rule, RuleOutcome, primitives};

fn why_has_content(ctx: &BriefContext<'_>) -> RuleOutcome {
    if primitives::has_content_after_heading(ctx.content, "## Why") {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: "`## Why` section missing or has no prose".to_string(),
        }
    }
}

fn domains_listed(ctx: &BriefContext<'_>) -> RuleOutcome {
    if primitives::has_content_after_heading(ctx.content, "## Domains") {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: "`## Domains` section missing or has no content".to_string(),
        }
    }
}

pub(super) const PROPOSAL_RULES: &[Rule] = &[
    Rule {
        id: "proposal.why-has-content",
        description: "Has a Why section with at least one sentence",
        check: why_has_content,
    },
    Rule {
        id: "proposal.domains-listed",
        description: "Has a Domains section listing at least one entry",
        check: domains_listed,
    },
];
