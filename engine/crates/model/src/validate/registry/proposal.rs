//! Proposal-brief rules.

use crate::validate::{BriefContext, Classification, Rule, RuleOutcome, primitives};

fn proposal_why_has_content(ctx: &BriefContext<'_>) -> RuleOutcome {
    if primitives::has_content_after_heading(ctx.content, "## Why") {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: "`## Why` section missing or has no prose".to_string(),
        }
    }
}

fn proposal_domains_listed(ctx: &BriefContext<'_>) -> RuleOutcome {
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
        classification: Classification::Structural,
        check: Some(proposal_why_has_content),
    },
    Rule {
        id: "proposal.domains-listed",
        description: "Has a Domains section listing at least one entry",
        classification: Classification::Structural,
        check: Some(proposal_domains_listed),
    },
    Rule {
        id: "proposal.uses-imperative-language",
        description: "Uses imperative language for motivation",
        classification: Classification::Semantic,
        check: None,
    },
];

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{proposal_domains_listed, proposal_why_has_content};
    use crate::validate::{BriefContext, RuleOutcome};

    fn ctx(content: &str) -> BriefContext<'_> {
        BriefContext {
            id: "proposal",
            content,
            parsed_spec: None,
            tasks: None,
            slice_dir: Path::new("."),
            specs_dir: Path::new("."),
        }
    }

    #[test]
    fn why_passes_prose_fails_empty() {
        assert_eq!(
            proposal_why_has_content(&ctx("## Why\n\nbecause it matters\n")),
            RuleOutcome::Pass
        );
        assert!(matches!(
            proposal_why_has_content(&ctx("## Why\n\n## Domains\n- a\n")),
            RuleOutcome::Fail { .. }
        ));
    }

    #[test]
    fn domains_passes_entries_fails_absent() {
        assert_eq!(proposal_domains_listed(&ctx("## Domains\n\n- login\n")), RuleOutcome::Pass);
        assert!(matches!(
            proposal_domains_listed(&ctx("## Why\n\nbecause\n")),
            RuleOutcome::Fail { .. }
        ));
    }
}
