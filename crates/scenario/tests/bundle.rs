//! Round-trip and completeness tests for the run-bundle contract.

use jiff::Timestamp;
use scenario::bundle::{Bundle, validate};
use scenario::{
    AssertionId, AssertionResult, Outcome, RubricResult, RunMetadata, ScenarioReport,
    ScenarioReportVersion, TrialMetrics, TrialResult, catalog,
};
use tempfile::tempdir;

fn assertion(id: AssertionId, outcome: Outcome) -> AssertionResult {
    AssertionResult {
        id,
        outcome,
        evidence: Some("evidence".to_owned()),
        detail: None,
    }
}

fn rubric(id: AssertionId, outcome: Outcome) -> RubricResult {
    RubricResult {
        id,
        outcome,
        score: Some(90),
        evidence: format!("rubric-{id}.json"),
        detail: Some("graded".to_owned()),
    }
}

fn trial(profile: &str) -> TrialResult {
    TrialResult {
        trial: 1,
        profile: profile.to_owned(),
        outcome: Outcome::Pass,
        hard_assertions: vec![
            assertion(AssertionId::GuestLoopDrained, Outcome::Pass),
            assertion(AssertionId::GuestJournalCadence, Outcome::Pass),
            assertion(AssertionId::GuestGeneratedCrateVerifies, Outcome::Pass),
            assertion(AssertionId::GuestMarkerReleased, Outcome::Pass),
        ],
        semantic_rubrics: vec![rubric(AssertionId::GuestSpecSensible, Outcome::Pass)],
        missing_outputs: vec![],
        metrics: TrialMetrics::default(),
        outputs: vec!["driver.log".into()],
    }
}

fn report(trials: Vec<TrialResult>, outcome: Outcome) -> ScenarioReport {
    ScenarioReport {
        version: ScenarioReportVersion,
        scenario: "guest-execute-loop".to_owned(),
        outcome,
        run: RunMetadata {
            id: "guest-execute-loop-wasm-live-test".to_owned(),
            runner: "quality wasm-live".to_owned(),
            revisions: [("specify".to_owned(), "deadbeef".to_owned())].into(),
            model: Some("cursor-default".to_owned()),
            judge_model: Some("cursor-default".to_owned()),
            prompt_digest: Some("sha256:0".to_owned()),
            component_digests: [].into(),
            started_at: Timestamp::UNIX_EPOCH,
            completed_at: Timestamp::UNIX_EPOCH,
        },
        trials,
    }
}

#[test]
fn complete_report_validates_and_round_trips() {
    let scenario = catalog::load("guest-execute-loop").expect("canonical scenario");
    let report = report(vec![trial("wasm-live")], Outcome::Pass);
    validate(&scenario, &report).expect("complete report validates");

    let dir = tempdir().expect("tempdir");
    let bundle = Bundle::new(dir.path().join("run"));
    bundle.create_trial(1).expect("trial dir");
    bundle.write_trial_result(&report.trials[0]).expect("trial result");
    let path = bundle.write_report(&report).expect("report");

    let raw = std::fs::read_to_string(path).expect("read back");
    let parsed: ScenarioReport = serde_json::from_str(&raw).expect("report parses");
    assert_eq!(parsed, report);
    assert!(bundle.trial_result(1).is_file());
    assert_eq!(
        bundle.rubric_verdict(1, AssertionId::GuestSpecSensible).file_name().unwrap(),
        "rubric-guest-spec-sensible.json"
    );
}

#[test]
fn missing_assertion_is_incomplete() {
    let scenario = catalog::load("guest-execute-loop").expect("canonical scenario");
    let mut incomplete = trial("wasm-live");
    incomplete.hard_assertions.pop();
    let error = validate(&scenario, &report(vec![incomplete], Outcome::Pass))
        .expect_err("missing assertion rejected");
    assert!(error.to_string().contains("guest-marker-released"), "{error}");
}

#[test]
fn duplicate_rubric_is_incomplete() {
    let scenario = catalog::load("guest-execute-loop").expect("canonical scenario");
    let mut duplicated = trial("wasm-live");
    duplicated.semantic_rubrics.push(rubric(AssertionId::GuestSpecSensible, Outcome::Pass));
    let error = validate(&scenario, &report(vec![duplicated], Outcome::Pass))
        .expect_err("duplicate rubric rejected");
    assert!(error.to_string().contains("2 times"), "{error}");
}

#[test]
fn hard_profile_skips_rubric_completeness() {
    let scenario = catalog::load("guest-execute-loop").expect("canonical scenario");
    let mut hard_only = trial("native-scripted");
    hard_only.semantic_rubrics.clear();
    validate(&scenario, &report(vec![hard_only], Outcome::Pass))
        .expect("hard-graded profile needs no rubric results");
}

#[test]
fn passed_report_with_failing_trial_is_rejected() {
    let scenario = catalog::load("guest-execute-loop").expect("canonical scenario");
    let mut failing = trial("wasm-live");
    failing.hard_assertions[0].outcome = Outcome::Fail;
    let error = validate(&scenario, &report(vec![failing], Outcome::Pass))
        .expect_err("pass verdict with failing trial rejected");
    assert!(error.to_string().contains("non-passing"), "{error}");
}

#[test]
fn empty_report_is_rejected() {
    let scenario = catalog::load("guest-execute-loop").expect("canonical scenario");
    let error =
        validate(&scenario, &report(vec![], Outcome::Fail)).expect_err("empty report rejected");
    assert!(error.to_string().contains("no trials"), "{error}");
}
