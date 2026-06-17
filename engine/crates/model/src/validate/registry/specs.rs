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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        specs_ids_match_pattern, specs_requirements_have_ids, specs_requirements_have_scenarios,
    };
    use crate::spec::{ParsedSpec, parse_baseline};
    use crate::validate::{BriefContext, RuleOutcome};

    fn ctx(spec: Option<&ParsedSpec>) -> BriefContext<'_> {
        BriefContext {
            id: "specs",
            content: "",
            parsed_spec: spec,
            tasks: None,
            slice_dir: Path::new("."),
            specs_dir: Path::new("."),
        }
    }

    /// Each structural rule fails with the shared "not parsed" detail
    /// when no spec was handed in — a rule-layer branch the primitives
    /// never see.
    #[test]
    fn rules_fail_when_spec_not_parsed() {
        for rule in [
            specs_requirements_have_scenarios,
            specs_requirements_have_ids,
            specs_ids_match_pattern,
        ] {
            assert!(matches!(
                rule(&ctx(None)),
                RuleOutcome::Fail { detail } if detail.contains("not parsed")
            ));
        }
    }

    mod scenarios {
        use super::{ctx, parse_baseline, specs_requirements_have_scenarios};
        use crate::validate::RuleOutcome;

        #[test]
        fn passes_when_present() {
            let spec = parse_baseline(
                "### Requirement: Thing\n\nID: REQ-001\n\n#### Scenario: Happy\n- WHEN a\n- THEN b\n",
            );
            assert_eq!(specs_requirements_have_scenarios(&ctx(Some(&spec))), RuleOutcome::Pass);
        }

        #[test]
        fn fails_when_absent() {
            let spec = parse_baseline("### Requirement: Thing\n\nID: REQ-001\n\nno scenario\n");
            assert!(matches!(
                specs_requirements_have_scenarios(&ctx(Some(&spec))),
                RuleOutcome::Fail { .. }
            ));
        }
    }

    mod ids {
        use super::{ctx, parse_baseline, specs_requirements_have_ids};
        use crate::validate::RuleOutcome;

        #[test]
        fn passes_when_present() {
            let spec =
                parse_baseline("### Requirement: Thing\n\nID: REQ-001\n\n#### Scenario: H\n");
            assert_eq!(specs_requirements_have_ids(&ctx(Some(&spec))), RuleOutcome::Pass);
        }

        #[test]
        fn fails_when_missing() {
            let spec = parse_baseline("### Requirement: Thing\n\n#### Scenario: H\n");
            assert!(matches!(
                specs_requirements_have_ids(&ctx(Some(&spec))),
                RuleOutcome::Fail { .. }
            ));
        }
    }

    mod id_pattern {
        use super::{ctx, parse_baseline, specs_ids_match_pattern};
        use crate::validate::RuleOutcome;

        #[test]
        fn passes_on_canonical_ids() {
            let spec =
                parse_baseline("### Requirement: Thing\n\nID: REQ-001\n\n#### Scenario: H\n");
            assert_eq!(specs_ids_match_pattern(&ctx(Some(&spec))), RuleOutcome::Pass);
        }

        #[test]
        fn fails_on_short_id() {
            let spec = parse_baseline("### Requirement: Thing\n\nID: REQ-1\n\n#### Scenario: H\n");
            assert!(matches!(specs_ids_match_pattern(&ctx(Some(&spec))), RuleOutcome::Fail { .. }));
        }
    }
}
