//! Specs-brief rules.

use crate::validate::{BriefContext, Classification, Rule, RuleOutcome, primitives};

fn specs_requirements_have_scenarios(ctx: &BriefContext<'_>) -> RuleOutcome {
    let Some(spec) = ctx.parsed_spec else {
        return RuleOutcome::Fail {
            detail: "spec was not parsed".to_string(),
        };
    };
    if primitives::all_requirements_have_scenarios(spec) {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: "one or more requirements have no scenarios".to_string(),
        }
    }
}

fn specs_requirements_have_ids(ctx: &BriefContext<'_>) -> RuleOutcome {
    let Some(spec) = ctx.parsed_spec else {
        return RuleOutcome::Fail {
            detail: "spec was not parsed".to_string(),
        };
    };
    if primitives::all_requirements_have_ids(spec) {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: "one or more requirements are missing an ID".to_string(),
        }
    }
}

fn specs_ids_match_pattern(ctx: &BriefContext<'_>) -> RuleOutcome {
    let Some(spec) = ctx.parsed_spec else {
        return RuleOutcome::Fail {
            detail: "spec was not parsed".to_string(),
        };
    };
    if primitives::ids_match_pattern(spec, crate::spec::REQ_ID_PATTERN) {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: format!(
                "one or more requirement IDs do not match `{}`",
                crate::spec::REQ_ID_PATTERN
            ),
        }
    }
}

pub(super) const SPECS_RULES: &[Rule] = &[
    Rule {
        id: "specs.requirements-have-scenarios",
        description: "Every requirement has at least one scenario",
        classification: Classification::Structural,
        check: Some(specs_requirements_have_scenarios),
    },
    Rule {
        id: "specs.requirements-have-ids",
        description: "Every requirement has an `ID:` line",
        classification: Classification::Structural,
        check: Some(specs_requirements_have_ids),
    },
    Rule {
        id: "specs.ids-match-pattern",
        description: "IDs use the `REQ-[0-9]{3}` format",
        classification: Classification::Structural,
        check: Some(specs_ids_match_pattern),
    },
    Rule {
        id: "specs.uses-normative-language",
        description: "Uses SHALL/MUST language for normative requirements",
        classification: Classification::Semantic,
        check: None,
    },
];
