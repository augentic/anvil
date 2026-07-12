//! Credential-free trial-loop coverage with a scripted fake judge.
//!
//! Exercises the full grading pass — hard assertions through the
//! shared registry plus semantic rubrics through the [`Judge`] seam —
//! and the verdict-validation edge cases the live judge never
//! reproduces deterministically.

use std::path::Path;

use scenario::bundle::Bundle;
use scenario::evaluate::semantic::{Judge, Rubrics};
use scenario::grade::{Evaluators, Execution, StepResult, Verdict};
use scenario::{AssertionId, Grading, Outcome, catalog};
use tempfile::tempdir;

/// A judge that returns a canned raw verdict (or a spawn failure).
struct FakeJudge {
    verdict: Result<String, String>,
}

impl FakeJudge {
    fn returning(raw: &str) -> Self {
        Self {
            verdict: Ok(raw.to_owned()),
        }
    }

    fn failing(detail: &str) -> Self {
        Self {
            verdict: Err(detail.to_owned()),
        }
    }
}

impl Judge for FakeJudge {
    async fn judge(&self, _prompt: String, _workspace: &Path) -> Result<String, String> {
        self.verdict.clone()
    }
}

fn rubrics() -> Rubrics {
    Rubrics::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../quality/rubrics/semantic.yaml"),
    )
    .expect("shared rubric catalog loads")
}

fn passing_execution(root: &Path) -> Execution {
    std::fs::create_dir_all(root.join(".specify")).expect("mkdir");
    std::fs::write(
        root.join(".specify/journal.jsonl"),
        "{\"event\":\"slice.merge.succeeded\"}\n{\"event\":\"slice.archive.created\"}\n",
    )
    .expect("journal fixture");
    // The scenario's declared expected-outputs, graded by the trial loop.
    std::fs::write(root.join("plan.yaml"), "plan: demo\n").expect("plan fixture");
    std::fs::write(root.join("discovery.md"), "# Lead inventory\n").expect("discovery fixture");
    Execution::new(
        root,
        [(
            "execute".to_owned(),
            StepResult {
                exit_code: 0,
                stdout: r#"{"status":"drained"}"#.to_owned(),
                stderr: String::new(),
            },
        )],
    )
}

fn evaluators() -> Evaluators {
    Evaluators::default()
        .with(AssertionId::GuestJournalCadence, scenario::evaluate::guest::journal_cadence)
        .with(AssertionId::GuestGeneratedCrateVerifies, |_| Verdict::pass("stubbed"))
}

async fn grade_with(judge: &impl Judge) -> (scenario::TrialResult, Bundle, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let scenario = catalog::load("guest-execute-loop").expect("canonical scenario");
    let profile = scenario
        .profiles
        .iter()
        .find(|profile| profile.id == "wasm-live")
        .expect("wasm-live profile");
    assert_eq!(profile.grading, Grading::Semantic);

    let bundle = Bundle::new(dir.path().join("run"));
    bundle.create_trial(1).expect("trial dir");
    let workspace = bundle.workspace(1);
    std::fs::create_dir_all(&workspace).expect("workspace");
    let execution = passing_execution(&workspace);

    let setting = quality::trial::Setting {
        scenario: &scenario,
        profile,
        evaluators: &evaluators(),
        rubrics: &rubrics(),
        bundle: &bundle,
    };
    let result =
        quality::trial::grade(&setting, &execution, judge, 1, 42).await.expect("trial grades");
    (result, bundle, dir)
}

#[tokio::test]
async fn full_trial_passes_with_passing_judge() {
    let judge = FakeJudge::returning(r#"{"score":92,"outcome":"pass","detail":"faithful spec"}"#);
    let (result, bundle, _dir) = grade_with(&judge).await;
    assert_eq!(result.outcome, Outcome::Pass, "{result:?}");
    assert!(result.hard_assertions.iter().all(|a| a.outcome == Outcome::Pass), "{result:?}");
    let rubric = &result.semantic_rubrics[0];
    assert_eq!(rubric.id, AssertionId::GuestSpecSensible);
    assert_eq!(rubric.score, Some(92));
    assert_eq!(rubric.evidence, "rubric-guest-spec-sensible.json");
    assert!(bundle.rubric_verdict(1, AssertionId::GuestSpecSensible).is_file());
    assert!(bundle.trial_result(1).is_file());
    assert_eq!(result.metrics.duration_ms, 42);
    assert!(!result.metrics.usage_available);
}

#[tokio::test]
async fn missing_expected_output_fails_the_trial() {
    let judge = FakeJudge::returning(r#"{"score":92,"outcome":"pass","detail":"faithful spec"}"#);
    let dir = tempdir().expect("tempdir");
    let scenario = catalog::load("guest-execute-loop").expect("canonical scenario");
    let profile = scenario.profiles.iter().find(|p| p.id == "wasm-live").expect("profile");
    let bundle = Bundle::new(dir.path().join("run"));
    bundle.create_trial(1).expect("trial dir");
    let workspace = bundle.workspace(1);
    std::fs::create_dir_all(&workspace).expect("workspace");
    let execution = passing_execution(&workspace);
    std::fs::remove_file(workspace.join("discovery.md")).expect("withholding an output");

    let setting = quality::trial::Setting {
        scenario: &scenario,
        profile,
        evaluators: &evaluators(),
        rubrics: &rubrics(),
        bundle: &bundle,
    };
    let result =
        quality::trial::grade(&setting, &execution, &judge, 1, 42).await.expect("trial grades");
    assert_eq!(result.outcome, Outcome::Fail, "{result:?}");
    assert_eq!(result.missing_outputs, vec!["expected file `discovery.md` is absent".to_owned()]);
}

#[tokio::test]
async fn failing_score_fails_the_trial() {
    let judge = FakeJudge::returning(r#"{"score":40,"outcome":"fail","detail":"spec drifts"}"#);
    let (result, _bundle, _dir) = grade_with(&judge).await;
    assert_eq!(result.outcome, Outcome::Fail);
    assert_eq!(result.semantic_rubrics[0].outcome, Outcome::Fail);
}

#[tokio::test]
async fn malformed_verdict_is_an_error() {
    let judge = FakeJudge::returning("not json at all");
    let (result, _bundle, _dir) = grade_with(&judge).await;
    assert_eq!(result.outcome, Outcome::Fail);
    let rubric = &result.semantic_rubrics[0];
    assert_eq!(rubric.outcome, Outcome::Error);
    assert!(rubric.detail.as_deref().unwrap().contains("valid JSON"), "{rubric:?}");
}

#[tokio::test]
async fn missing_score_is_an_error() {
    let judge = FakeJudge::returning(r#"{"outcome":"pass","detail":"no score"}"#);
    let (result, _bundle, _dir) = grade_with(&judge).await;
    assert_eq!(result.semantic_rubrics[0].outcome, Outcome::Error);
}

#[tokio::test]
async fn out_of_range_score_is_an_error() {
    let judge = FakeJudge::returning(r#"{"score":150,"outcome":"pass","detail":"too high"}"#);
    let (result, _bundle, _dir) = grade_with(&judge).await;
    assert_eq!(result.semantic_rubrics[0].outcome, Outcome::Error);
}

#[tokio::test]
async fn score_overrides_disagreeing_outcome() {
    let judge = FakeJudge::returning(r#"{"score":95,"outcome":"fail","detail":"contradiction"}"#);
    let (result, _bundle, _dir) = grade_with(&judge).await;
    let rubric = &result.semantic_rubrics[0];
    assert_eq!(rubric.outcome, Outcome::Pass, "the score decides");
    assert!(rubric.detail.as_deref().unwrap().contains("score decides"), "{rubric:?}");
}

#[tokio::test]
async fn judge_spawn_failure_is_an_error() {
    let judge = FakeJudge::failing("no credentials");
    let (result, _bundle, _dir) = grade_with(&judge).await;
    let rubric = &result.semantic_rubrics[0];
    assert_eq!(rubric.outcome, Outcome::Error);
    assert!(rubric.detail.as_deref().unwrap().contains("could not run"), "{rubric:?}");
}
