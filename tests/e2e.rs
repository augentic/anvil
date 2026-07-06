//! End-to-end integration tests for the `specify` CLI.
//!
//! Each test stands up a fresh `.specify/` project in a `tempfile::TempDir`,
//! drives the built binary via `assert_cmd`, and compares stdout against a
//! checked-in golden JSON file (under `tests/fixtures/e2e/goldens/`).
//!
//! ## Regenerating goldens
//!
//! Goldens hold the canonical stdout shape after [`strip_substitutions`] has
//! replaced tempdir paths and today's date with deterministic placeholders.
//! When a subcommand's output shape intentionally changes, rerun this file
//! with `REGENERATE_GOLDENS=1` and commit the diff — see
//! [testing.md §"Golden files"](../docs/standards/testing.md).

use std::fs;
use std::path::PathBuf;

use common::{Project, assert_golden_at, parse_stdout, repo_root, specify_cmd};
use serde_json::Value;

use crate::common;

// ---------------------------------------------------------------------------
// Paths + setup helpers
// ---------------------------------------------------------------------------

fn e2e_fixtures() -> PathBuf {
    repo_root().join("tests/fixtures/e2e")
}

fn goldens_dir() -> PathBuf {
    e2e_fixtures().join("goldens")
}

// ---------------------------------------------------------------------------
// Substitution / golden comparison
// ---------------------------------------------------------------------------

fn assert_golden(name: &str, actual: Value) {
    assert_golden_at(&goldens_dir(), name, actual);
}

// ---------------------------------------------------------------------------
// 1. validate — good fixture
// ---------------------------------------------------------------------------

#[test]
fn validate_good_slice_passes() {
    let project = Project::init().with_schemas();
    project.stage_slice("good-slice");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .success();
    assert_eq!(assert.get_output().status.code(), Some(0));

    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    // The validate surface now emits a `DiagnosticReport`. A clean slice
    // carries no blocking (critical/important) diagnostics; exit 0 is the
    // pass signal.
    assert_eq!(actual["summary"]["critical"], 0);
    assert_eq!(actual["summary"]["important"], 0);
    assert_golden("validate-good.json", actual);
}

// ---------------------------------------------------------------------------
// 2. validate — bad fixture
// ---------------------------------------------------------------------------

#[test]
fn validate_bad_slice_fails_with_exit_two() {
    let project = Project::init().with_schemas();
    project.stage_slice("bad-slice");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2), "validate on bad fixture must exit 2");

    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    // The validate surface now emits a `DiagnosticReport`; the failing
    // slice carries at least one blocking `important` violation and the
    // command exits 2.
    assert!(
        actual["summary"]["important"].as_u64().unwrap_or(0) > 0,
        "bad fixture must surface important violations: {actual}"
    );
    assert_golden("validate-bad.json", actual);
}

// ---------------------------------------------------------------------------
// 4. task progress
// ---------------------------------------------------------------------------

#[test]
fn task_progress_reports_counts_and_items() {
    let project = Project::init().with_schemas();
    project.stage_slice("good-slice");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "task", "progress", "my-slice"])
        .assert()
        .success();

    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["total"], 5);
    assert_eq!(actual["complete"], 2);
    assert_eq!(actual["pending"], 3);
    assert_golden("task-progress.json", actual);
}

// ---------------------------------------------------------------------------
// 5. task mark — idempotent
// ---------------------------------------------------------------------------

#[test]
fn task_mark_is_idempotent() {
    let project = Project::init().with_schemas();
    project.stage_slice("good-slice");
    let tasks_path = project.root().join(".specify/slices/my-slice/tasks.md");

    let before = fs::read_to_string(&tasks_path).expect("read tasks before");
    assert!(before.contains("- [ ] 1.1"), "fixture must start with task 1.1 incomplete");

    // First mark: flips - [ ] -> - [x] and reports idempotent: false.
    let first = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "task", "mark", "my-slice", "1.1"])
        .assert()
        .success();
    let first_value = parse_stdout(&first.get_output().stdout, project.root());
    assert_eq!(first_value["marked"], "1.1");
    assert_eq!(first_value["idempotent"], false);

    let after_first = fs::read_to_string(&tasks_path).expect("read tasks after 1st mark");
    assert!(after_first.contains("- [x] 1.1"), "tasks.md should now show 1.1 complete");
    assert!(
        !after_first.contains("- [ ] 1.1"),
        "tasks.md should no longer have the incomplete form of 1.1"
    );

    // Second mark: no-op, idempotent: true, file unchanged.
    let second = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "task", "mark", "my-slice", "1.1"])
        .assert()
        .success();
    let second_value = parse_stdout(&second.get_output().stdout, project.root());
    assert_eq!(second_value["idempotent"], true);

    let after_second = fs::read_to_string(&tasks_path).expect("read tasks after 2nd mark");
    assert_eq!(after_first, after_second, "second mark must leave tasks.md byte-identical");

    assert_golden("task-mark.json", second_value);
}
