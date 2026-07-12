//! Public-boundary tests for the profile-specific evaluators.

use std::fs;
use std::path::Path;

use scenario::evaluate::{guest, semantic};
use scenario::{AssertionId, AssertionResult, Outcome, catalog};
use tempfile::tempdir;

fn unresolved(id: AssertionId) -> AssertionResult {
    AssertionResult {
        id,
        outcome: Outcome::Fail,
        evidence: None,
        detail: Some("requires a profile-specific evaluator".into()),
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

        let mut results = vec![unresolved(AssertionId::GuestJournalCadence)];
        guest::guest(&mut results, workspace.path());
        assert_eq!(results[0].outcome, Outcome::Pass, "{results:?}");
        assert_eq!(results[0].evidence.as_deref(), Some(".specify/journal.jsonl"));
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

        let mut results = vec![unresolved(AssertionId::GuestJournalCadence)];
        guest::guest(&mut results, workspace.path());
        assert_eq!(results[0].outcome, Outcome::Fail);
        let detail = results[0].detail.as_deref().expect("failure detail");
        assert!(detail.contains("slice.archive.created"), "{detail}");
    }

    #[test]
    fn fails_without_journal() {
        let workspace = tempdir().expect("tempdir");
        let mut results = vec![unresolved(AssertionId::GuestJournalCadence)];
        guest::guest(&mut results, workspace.path());
        assert_eq!(results[0].outcome, Outcome::Fail);
    }
}

mod generated_crates {
    use super::*;

    fn write_crate(root: &Path, name: &str, lib: &str) {
        let crate_root = root.join("crates").join(name);
        fs::create_dir_all(crate_root.join("src")).expect("mkdir");
        fs::write(
            crate_root.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n"
            ),
        )
        .expect("manifest fixture");
        fs::write(crate_root.join("src/lib.rs"), lib).expect("lib fixture");
    }

    #[test]
    fn passes_when_crate_checks() {
        let workspace = tempdir().expect("tempdir");
        write_crate(workspace.path(), "demo", "pub fn answer() -> u8 { 42 }\n");

        let mut results = vec![unresolved(AssertionId::GuestGeneratedCrateVerifies)];
        guest::guest(&mut results, workspace.path());
        assert_eq!(results[0].outcome, Outcome::Pass, "{results:?}");
    }

    #[test]
    fn fails_when_crate_does_not_compile() {
        let workspace = tempdir().expect("tempdir");
        write_crate(workspace.path(), "broken", "pub fn answer() -> u8 { \"not a u8\" }\n");

        let mut results = vec![unresolved(AssertionId::GuestGeneratedCrateVerifies)];
        guest::guest(&mut results, workspace.path());
        assert_eq!(results[0].outcome, Outcome::Fail);
        let detail = results[0].detail.as_deref().expect("failure detail");
        assert!(detail.contains("cargo check failed"), "{detail}");
    }

    #[test]
    fn fails_without_generated_crates() {
        let workspace = tempdir().expect("tempdir");
        let mut results = vec![unresolved(AssertionId::GuestGeneratedCrateVerifies)];
        guest::guest(&mut results, workspace.path());
        assert_eq!(results[0].outcome, Outcome::Fail);
        assert_eq!(results[0].detail.as_deref(), Some("no generated crates/ directory"));
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
