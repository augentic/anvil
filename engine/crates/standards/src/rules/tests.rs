use jsonschema::Validator;
use serde_json::Value as JsonValue;
use specify_schema::{RESOLVED_RULES_JSON_SCHEMA, RULE_JSON_SCHEMA};

use super::{
    Applicability, Deprecated, HintKind, LintMode, Origin, PathRoot, Reference, ResolvedRule,
    ResolvedRules, Rule, RuleHint, Severity,
};

fn validator(schema_source: &str) -> Validator {
    let schema: JsonValue = serde_json::from_str(schema_source).expect("schema parses");
    jsonschema::validator_for(&schema).expect("schema compiles")
}

fn assert_validates(schema_source: &str, instance: &JsonValue) {
    let v = validator(schema_source);
    let errors: Vec<String> = v.iter_errors(instance).map(|e| e.to_string()).collect();
    assert!(errors.is_empty(), "instance must validate; errors: {errors:?}");
}

/// `ResolvedRules` export contract: severity comparator order is
/// `critical < important < suggestion < optional`.
#[test]
fn severity_ordering_matches_contract() {
    assert!(Severity::Critical < Severity::Important);
    assert!(Severity::Important < Severity::Suggestion);
    assert!(Severity::Suggestion < Severity::Optional);
    let mut shuffled =
        vec![Severity::Optional, Severity::Critical, Severity::Suggestion, Severity::Important];
    shuffled.sort();
    assert_eq!(
        shuffled,
        vec![Severity::Critical, Severity::Important, Severity::Suggestion, Severity::Optional,]
    );
}

/// `ResolvedRules` export contract §"Ordering": origin comparator
/// order is `target, source, shared, core, unknown`.
#[test]
fn origin_ordering_matches_contract() {
    assert!(Origin::Target < Origin::Source);
    assert!(Origin::Source < Origin::Shared);
    assert!(Origin::Shared < Origin::Core);
    assert!(Origin::Core < Origin::Unknown);
    let mut shuffled =
        vec![Origin::Unknown, Origin::Core, Origin::Shared, Origin::Target, Origin::Source];
    shuffled.sort();
    assert_eq!(
        shuffled,
        vec![Origin::Target, Origin::Source, Origin::Shared, Origin::Core, Origin::Unknown]
    );
}

/// `Rule` round-trips its own JSON shape, exercising the
/// snake-to-kebab field renames (`lint-mode`,
/// `rule-hints`).
#[test]
fn codex_rule_round_trips() {
    let rule = Rule {
            id: "UNI-014".into(),
            title: "Hardcoded Configuration".into(),
            severity: Severity::Important,
            trigger: "Generated code embeds environment-specific configuration instead of routing it through declared configuration.".into(),
            lint_mode: Some(LintMode::Hybrid),
            applicability: Some(Applicability {
                adapters: Some(vec!["omnia".into()]),
                languages: Some(vec!["rust".into()]),
                artifacts: Some(vec!["code".into()]),
                paths: None,
            }),
            rule_hints: Some(vec![RuleHint {
                kind: HintKind::Regex,
                value: "https?://".into(),
                description: Some("Literal URL in generated code.".into()),
                config: None,
            }]),
            references: Some(vec![Reference {
                label: "Omnia guardrails".into(),
                url: None,
                path: Some("adapters/targets/omnia/references/guardrails.md".into()),
            }]),
            deprecated: None,
            body: "## Rule\n\nConfiguration values that vary between deployments must not be hardcoded in generated code.\n".into(),
        };
    let value = serde_json::to_value(&rule).expect("serialise");
    assert_eq!(value.get("lint-mode").and_then(JsonValue::as_str), Some("hybrid"));
    assert!(value.get("rule-hints").is_some());
    let parsed: Rule = serde_json::from_value(value).expect("round-trip");
    assert_eq!(rule, parsed);
}

/// UNI-014 example builds from typed structs, validates
/// against `resolved.schema.json`, and round-trips back to the
/// same struct.
#[test]
fn resolved_codex_round_trips() {
    let resolved = ResolvedRules {
            version: 1,
            target_adapter: "omnia".into(),
            source_adapters: vec!["typescript".into()],
            rules: vec![ResolvedRule {
                rule_id: "UNI-014".into(),
                title: "Hardcoded Configuration".into(),
                severity: Severity::Important,
                trigger: "Generated code embeds environment-specific configuration instead of routing it through declared configuration.".into(),
                lint_mode: Some(LintMode::Hybrid),
                applicability: Some(Applicability {
                    adapters: Some(vec!["omnia".into()]),
                    languages: Some(vec!["rust".into()]),
                    artifacts: Some(vec!["code".into()]),
                    paths: None,
                }),
                rule_hints: Some(vec![RuleHint {
                    kind: HintKind::Regex,
                    value: "https?://".into(),
                    description: Some("Literal URL in generated code.".into()),
                    config: None,
                }]),
                references: Some(vec![Reference {
                    label: "Omnia guardrails".into(),
                    url: None,
                    path: Some("adapters/targets/omnia/references/guardrails.md".into()),
                }]),
                origin: Origin::Shared,
                path_root: PathRoot::RulesRoot,
                path: "adapters/shared/rules/universal/hardcoded-configuration.md".into(),
                body: "## Rule\n\nConfiguration values that vary between deployments must not be hardcoded in generated code.\n".into(),
                deprecated: None,
            }],
        };
    let value = serde_json::to_value(&resolved).expect("serialise");
    assert_validates(RESOLVED_RULES_JSON_SCHEMA, &value);
    let parsed: ResolvedRules = serde_json::from_value(value).expect("round-trip");
    assert_eq!(resolved, parsed);
}

/// `Deprecated.replaced_by` MUST serialise to the kebab-case wire
/// key `replaced-by` per `ResolvedRules` export contract. Test
/// covers the explicitly-called-out rename.
#[test]
fn deprecated_replaced_by_kebab() {
    let deprecated = Deprecated {
        reason: "superseded by SEC-001".into(),
        replaced_by: Some("SEC-001".into()),
    };
    let value = serde_json::to_value(&deprecated).expect("serialise");
    assert_eq!(value.get("replaced-by").and_then(JsonValue::as_str), Some("SEC-001"));
    assert!(value.get("replaced_by").is_none(), "snake_case wire key must not appear");

    let body = serde_json::to_string(&deprecated).expect("serialise");
    assert!(body.contains("\"replaced-by\""), "body must carry replaced-by; got {body}");
    assert!(!body.contains("replaced_by"), "snake_case must not leak; got {body}");

    let parsed: Deprecated = serde_json::from_value(value).expect("round-trip");
    assert_eq!(deprecated, parsed);

    // Sanity: the standalone `Rule` schema also reads the
    // post-lift kebab-case shape since CH-10 owns the wire-side
    // structs (the snake-cased authoring schema is exercised by
    // the parallel test in `schema.rs`).
    validator(RULE_JSON_SCHEMA);
}
