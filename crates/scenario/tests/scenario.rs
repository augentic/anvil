//! Public-boundary tests for canonical scenarios and reports.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use scenario::{
    AssertionId, AssertionKind, AssertionResult, Outcome, RubricResult, RunMetadata, Scenario,
    ScenarioReport, ScenarioReportVersion, TrialMetrics, TrialResult, assertion_registry,
};
use tempfile::tempdir;

const VALID: &str = r#"
version: 1
id: intent-only
owner: scenarios
gate-tier: release-blocker
isolation: fresh-project
setup:
  commands: ["specify init omnia@1.0.0"]
  environment: {}
workflow:
  - id: plan
    kind: prompt
    run: /spec:plan fix-typo
    profile: default
    fixtures: [brief]
profiles:
  - id: default
    runtime: native
    model: live
    grading: semantic
    trials: 2
    environment: {}
fixtures:
  - id: brief
    source: fixtures/brief.md
    destination: brief.md
hard-assertions:
  - id: plan-exists
    after: plan
    probe:
      kind: path-exists
      path: plan.yaml
semantic-rubrics:
  - id: slices-match-expected-shape
    after: plan
    criterion: Slice scope reflects the operator brief.
    evidence: [plan.yaml]
expected-outputs:
  - path: plan.yaml
    kind: file
"#;

#[test]
fn canonical_loads() {
    let scenario = Scenario::from_yaml(VALID).expect("canonical scenario parses and validates");
    assert_eq!(scenario.id, "intent-only");
    assert_eq!(scenario.profiles[0].trials, 2);
    assert_eq!(scenario.hard_assertions[0].id, AssertionId::PlanExists);
}

#[test]
fn file_loads() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("scenario.yaml");
    fs::write(&path, VALID).expect("write fixture");

    let scenario = Scenario::load(&path).expect("scenario file loads");
    assert_eq!(scenario.owner, "scenarios");
}

#[test]
fn unknown_field_errors() {
    let malformed = VALID.replace("owner: scenarios", "owner: scenarios\nsurprise: true");
    let error = Scenario::from_yaml(&malformed).expect_err("unknown top-level field fails");
    assert_eq!(error.variant_str(), "scenario-schema");
}

#[test]
fn bad_reference_errors() {
    let malformed = VALID.replace("profile: default", "profile: missing");
    let error = Scenario::from_yaml(&malformed).expect_err("unknown profile fails");
    assert_eq!(error.variant_str(), "scenario-contract");
    assert!(error.to_string().contains("unknown profile `missing`"));
}

#[test]
fn assertion_kind_errors() {
    let malformed =
        VALID.replace("id: plan-exists\n    after", "id: guest-spec-sensible\n    after");
    let error = Scenario::from_yaml(&malformed).expect_err("semantic id cannot be hard");
    assert_eq!(error.variant_str(), "scenario-contract");
}

#[test]
fn registry_is_closed() {
    let registry = assertion_registry();
    let ids = registry.iter().map(|metadata| metadata.id).collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), registry.len());
    assert_eq!(AssertionId::PlanExists.metadata().kind, AssertionKind::Hard);
    assert_eq!(AssertionId::GuestSpecSensible.metadata().kind, AssertionKind::Semantic);
}

#[test]
fn report_round_trips() {
    let started_at = "2026-07-12T01:00:00Z".parse().expect("timestamp");
    let completed_at = "2026-07-12T01:01:00Z".parse().expect("timestamp");
    let report = ScenarioReport {
        version: ScenarioReportVersion,
        scenario: "intent-only".into(),
        outcome: Outcome::Pass,
        run: RunMetadata {
            id: "run-001".into(),
            runner: "native-harness@0.27.2".into(),
            revisions: [("specify".into(), "abc123".into())].into(),
            model: Some("cursor".into()),
            judge_model: None,
            prompt_digest: Some("sha256:abc".into()),
            component_digests: BTreeMap::default(),
            started_at,
            completed_at,
        },
        trials: vec![TrialResult {
            trial: 1,
            profile: "default".into(),
            outcome: Outcome::Pass,
            hard_assertions: vec![AssertionResult {
                id: AssertionId::PlanExists,
                outcome: Outcome::Pass,
                evidence: Some("plan.yaml".into()),
                detail: None,
            }],
            semantic_rubrics: vec![RubricResult {
                id: AssertionId::SlicesMatchExpectedShape,
                outcome: Outcome::Pass,
                score: Some(100),
                evidence: "plan.yaml".into(),
                detail: None,
            }],
            missing_outputs: vec![],
            metrics: TrialMetrics {
                usage_available: true,
                input_tokens: 10,
                output_tokens: 20,
                reasoning_tokens: 5,
                duration_ms: 250,
            },
            outputs: vec!["plan.yaml".into()],
        }],
    };

    let yaml = serde_saphyr::to_string(&report).expect("serialise report");
    let decoded: ScenarioReport = serde_saphyr::from_str(&yaml).expect("deserialize report");
    assert_eq!(decoded, report);
}

#[test]
fn hard_assertions_grade() {
    let scenario = Scenario::from_yaml(VALID).expect("scenario");
    let temp = tempdir().expect("tempdir");
    fs::write(temp.path().join("plan.yaml"), "lifecycle: pending").expect("plan fixture");
    let execution = scenario::grade::Execution::new(
        temp.path(),
        BTreeMap::from([(
            "plan".into(),
            scenario::grade::StepResult {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        )]),
    );

    let results = scenario::grade::hard(&scenario, &execution);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].outcome, Outcome::Pass);
}

#[test]
fn live_requires_semantic_grading() {
    let malformed = VALID.replace("grading: semantic", "grading: hard");
    let error = Scenario::from_yaml(&malformed).expect_err("live hard-only profile fails");
    assert_eq!(error.variant_str(), "scenario-contract");
}
