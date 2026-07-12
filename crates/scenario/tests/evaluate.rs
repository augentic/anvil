//! Public-boundary tests for the grading registry and the pure
//! profile-specific evaluators.

use std::fs;
use std::path::Path;

use scenario::evaluate::{composed, guest, semantic};
use scenario::grade::{Evaluators, Execution, StepResult, Verdict};
use scenario::{AssertionId, Outcome, catalog};
use tempfile::tempdir;

fn execution(root: &Path) -> Execution {
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

mod registry {
    use super::*;

    #[test]
    fn unregistered_probe_fails_with_detail() {
        let workspace = tempdir().expect("tempdir");
        let scenario = catalog::load("guest-execute-loop").expect("canonical scenario");
        let results = scenario::grade::hard(&scenario, &execution(workspace.path()));
        let cadence = results
            .iter()
            .find(|result| result.id == AssertionId::GuestJournalCadence)
            .expect("cadence result");
        assert_eq!(cadence.outcome, Outcome::Fail);
        let detail = cadence.detail.as_deref().expect("failure detail");
        assert!(detail.contains("requires a profile-specific evaluator"), "{detail}");
    }

    #[test]
    fn registered_evaluator_settles_probe() {
        let workspace = tempdir().expect("tempdir");
        let scenario = catalog::load("guest-execute-loop").expect("canonical scenario");
        let evaluators = Evaluators::default()
            .with(AssertionId::GuestJournalCadence, |_| Verdict::pass("stubbed"))
            .with(AssertionId::GuestGeneratedCrateVerifies, |_| {
                Verdict::fail("crates/", "stub failure")
            });
        let results =
            scenario::grade::hard_with(&scenario, &execution(workspace.path()), &evaluators);
        let by_id =
            |id: AssertionId| results.iter().find(|result| result.id == id).expect("graded");
        assert_eq!(by_id(AssertionId::GuestJournalCadence).outcome, Outcome::Pass);
        assert_eq!(by_id(AssertionId::GuestJournalCadence).evidence.as_deref(), Some("stubbed"));
        let failed = by_id(AssertionId::GuestGeneratedCrateVerifies);
        assert_eq!(failed.outcome, Outcome::Fail);
        assert_eq!(failed.detail.as_deref(), Some("stub failure"));
    }

    #[test]
    fn steps_keep_execution_order() {
        let steps = [
            (
                "z-first".to_owned(),
                StepResult {
                    exit_code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                },
            ),
            (
                "a-second".to_owned(),
                StepResult {
                    exit_code: 1,
                    stdout: String::new(),
                    stderr: String::new(),
                },
            ),
        ];
        let execution = Execution::new("/tmp", steps);
        let order: Vec<&str> = execution.steps().map(|(id, _)| id).collect();
        assert_eq!(order, ["z-first", "a-second"]);
        assert_eq!(execution.step("a-second").expect("lookup").exit_code, 1);
    }
}

mod journal_cadence {
    use super::*;

    #[test]
    fn passes_on_full_cadence() {
        let workspace = tempdir().expect("tempdir");
        fs::create_dir_all(workspace.path().join(".specify")).expect("mkdir");
        fs::write(
            workspace.path().join(".specify/journal.jsonl"),
            "{\"event\":\"slice.merge.succeeded\"}\n{\"event\":\"slice.archive.created\"}\n",
        )
        .expect("journal fixture");

        let verdict = guest::journal_cadence(&execution(workspace.path()));
        assert!(verdict.passed, "{verdict:?}");
        assert_eq!(verdict.evidence, ".specify/journal.jsonl");
    }

    #[test]
    fn fails_on_missing_event() {
        let workspace = tempdir().expect("tempdir");
        fs::create_dir_all(workspace.path().join(".specify")).expect("mkdir");
        fs::write(
            workspace.path().join(".specify/journal.jsonl"),
            "{\"event\":\"slice.merge.succeeded\"}\n",
        )
        .expect("journal fixture");

        let verdict = guest::journal_cadence(&execution(workspace.path()));
        assert!(!verdict.passed);
        let detail = verdict.detail.as_deref().expect("failure detail");
        assert!(detail.contains("slice.archive.created"), "{detail}");
    }

    #[test]
    fn fails_without_journal() {
        let workspace = tempdir().expect("tempdir");
        let verdict = guest::journal_cadence(&execution(workspace.path()));
        assert!(!verdict.passed);
    }
}

mod composed_evaluators {
    use super::*;

    #[test]
    fn drained_plan_passes() {
        let workspace = tempdir().expect("tempdir");
        fs::write(workspace.path().join("plan.yaml"), "entries:\n- status: done\n")
            .expect("plan fixture");
        assert!(composed::plan_drained(&execution(workspace.path())).passed);
    }

    #[test]
    fn pending_plan_fails() {
        let workspace = tempdir().expect("tempdir");
        fs::write(
            workspace.path().join("plan.yaml"),
            "entries:\n- status: done\n- status: pending\n",
        )
        .expect("plan fixture");
        assert!(!composed::plan_drained(&execution(workspace.path())).passed);
    }

    #[test]
    fn baseline_checks_track_the_merged_spec() {
        let workspace = tempdir().expect("tempdir");
        assert!(!composed::baseline_merge_visible(&execution(workspace.path())).passed);
        assert!(!composed::artifacts_complete(&execution(workspace.path())).passed);

        fs::create_dir_all(workspace.path().join(".specify/specs/echo")).expect("mkdir");
        fs::write(
            workspace.path().join(".specify/specs/echo/spec.md"),
            "ID: REQ-001\nSources: echo\n",
        )
        .expect("spec fixture");
        assert!(composed::baseline_merge_visible(&execution(workspace.path())).passed);
        assert!(composed::artifacts_complete(&execution(workspace.path())).passed);
    }
}

mod rubric_catalog {
    use super::*;

    #[test]
    fn shared_catalog_loads() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../quality/rubrics/semantic.yaml");
        let rubrics = semantic::Rubrics::load(&path).expect("shared rubric catalog loads");
        assert_eq!(rubrics.scale.pass, 80);
        assert!(
            rubrics.criteria.contains_key("guest-spec-sensible"),
            "guest-spec-sensible is catalogued"
        );
    }

    #[test]
    fn embedded_catalog_matches_disk() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../quality/rubrics/semantic.yaml");
        let disk = fs::read_to_string(&path).expect("shared rubric catalog reads");
        assert_eq!(semantic::CATALOG_YAML, disk, "embedded catalog drifted from disk");
        let rubrics = semantic::Rubrics::embedded().expect("embedded catalog parses");
        assert_eq!(rubrics.scale.pass, 80);
    }
}

mod step_argv {
    use super::*;

    #[test]
    fn canonical_steps_split() {
        let scenario = catalog::load("guest-execute-loop").expect("canonical scenario");
        let author = scenario.workflow.iter().find(|step| step.id == "author").expect("author");
        assert_eq!(
            author.argv().expect("author splits"),
            ["specify", "plan", "author", "demo", "--intent", "Fix the greeting."]
        );
        let execute = scenario.workflow.iter().find(|step| step.id == "execute").expect("execute");
        assert_eq!(
            execute.argv().expect("execute splits"),
            ["specify", "plan", "execute", "--format", "json"]
        );
    }
}
