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

use assert_cmd::Command;
use serde_json::Value;

mod common;
use common::{
    GIT_ENV, Project, assert_golden_at, copy_dir, hold_plan_lock, omnia_schema_dir, parse_stdout,
    repo_root, run_git, specify_cmd,
};
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Paths + setup helpers
// ---------------------------------------------------------------------------

fn e2e_fixtures() -> PathBuf {
    repo_root().join("tests/fixtures/e2e")
}

fn goldens_dir() -> PathBuf {
    e2e_fixtures().join("goldens")
}

fn specify_with_git_identity() -> Command {
    let mut cmd = specify_cmd();
    cmd.envs(GIT_ENV);
    cmd
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
// 3. merge — two-spec slice
// ---------------------------------------------------------------------------

#[test]
fn merge_two_spec_slice_produces_baselines() {
    let project = Project::init().with_schemas();
    project.stage_slice("merge-two-spec-slice");
    project.seed_plan(
        "\
name: merge-e2e
slices:
  - name: my-slice
    project: default
    status: in-progress
",
    );
    let _lock = project.hold_plan_lock();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "merge", "run", "my-slice"])
        .assert()
        .success();

    // Baselines landed under .specify/specs/.
    let login_baseline = project.root().join(".specify/specs/login/spec.md");
    let oauth_baseline = project.root().join(".specify/specs/oauth/spec.md");
    assert!(login_baseline.is_file(), "login baseline must exist");
    assert!(oauth_baseline.is_file(), "oauth baseline must exist");

    // Slice dir moved under archive/<YYYY-MM-DD>-my-slice/.
    let archive_root = project.root().join(".specify/archive");
    let archived: Vec<_> =
        fs::read_dir(&archive_root).expect("read archive dir").filter_map(Result::ok).collect();
    assert_eq!(archived.len(), 1, "expected one archived slice");
    let archived_name = archived[0].file_name().to_string_lossy().into_owned();
    assert!(
        archived_name.ends_with("-my-slice"),
        "archive dir name must end with -my-slice, got {archived_name}"
    );
    assert!(
        !project.root().join(".specify/slices/my-slice").exists(),
        "original slice dir should be gone"
    );

    let plan_yaml = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
    assert!(
        plan_yaml.contains("status: done"),
        "merge must stamp plan entry done, got:\n{plan_yaml}"
    );

    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_golden("merge-two-spec.json", actual);
}

#[test]
fn workspace_merge_stamps_plan_via_plan_dir() {
    // Workspace routing: merge runs inside the slot (which carries no
    // plan.yaml by design) with `--plan-dir` pointing at the initiating
    // workspace root, so `slice merge` — the sole writer of per-entry
    // `done` — stamps the workspace plan from the slot.
    let tmp = tempdir().expect("tempdir");
    let workspace_root = tmp.path();
    fs::write(
        workspace_root.join("plan.yaml"),
        "\
name: merge-e2e
slices:
  - name: my-slice
    project: orders
    status: in-progress
",
    )
    .expect("write workspace plan.yaml");

    let project_root = workspace_root.join("workspace/orders");
    fs::create_dir_all(&project_root).expect("mkdir workspace project");
    specify_cmd()
        .current_dir(&project_root)
        .args(["init"])
        .arg(omnia_schema_dir())
        .args(["--name", "orders"])
        .assert()
        .success();
    copy_dir(&omnia_schema_dir(), &project_root.join("adapters").join("targets").join("omnia"));
    copy_dir(
        &e2e_fixtures().join("merge-two-spec-slice"),
        &project_root.join(".specify/slices/my-slice"),
    );

    // The driver lock lives at the *workspace* root (`--plan-dir`), not
    // the slot — the probe must resolve through `Layout::plan_dir()`.
    let _lock = hold_plan_lock(workspace_root);

    specify_cmd()
        .current_dir(&project_root)
        .args(["--format", "json", "--plan-dir"])
        .arg(workspace_root)
        .args(["slice", "merge", "run", "my-slice"])
        .assert()
        .success();

    assert!(!project_root.join("plan.yaml").exists(), "the slot must not grow its own plan.yaml");
    let plan_yaml = fs::read_to_string(workspace_root.join("plan.yaml")).expect("read plan.yaml");
    assert!(
        plan_yaml.contains("status: done"),
        "merge must stamp the workspace plan entry done, got:\n{plan_yaml}"
    );
}

#[test]
fn workspace_merge_excludes_generated() {
    let tmp = tempdir().expect("tempdir");
    // The slot lives at `<platform>/workspace/<peer>/`; `is_slot` keys on
    // the platform root carrying `.specify/project.yaml`, so seed it.
    fs::create_dir_all(tmp.path().join(".specify")).expect("mkdir platform .specify");
    fs::write(tmp.path().join(".specify/project.yaml"), "workspace: true\n")
        .expect("write platform project.yaml");
    let project_root = tmp.path().join("workspace/orders");
    fs::create_dir_all(&project_root).expect("mkdir workspace project");

    specify_cmd()
        .current_dir(&project_root)
        .args(["init"])
        .arg(omnia_schema_dir())
        .args(["--name", "orders"])
        .assert()
        .success();
    copy_dir(&omnia_schema_dir(), &project_root.join("adapters").join("targets").join("omnia"));

    run_git(&project_root, &["init"]);
    run_git(&project_root, &["add", "."]);
    run_git(&project_root, &["commit", "-m", "initial project"]);

    let slice_dir = project_root.join(".specify/slices/my-slice");
    copy_dir(&e2e_fixtures().join("merge-two-spec-slice"), &slice_dir);
    fs::create_dir_all(slice_dir.join("contracts/schemas")).expect("mkdir slice contracts");
    fs::write(slice_dir.join("contracts/schemas/generated.yaml"), "openapi: 3.1\n")
        .expect("write generated contract");

    let generated_crate = project_root.join("crates/generated/src/lib.rs");
    fs::create_dir_all(generated_crate.parent().expect("crate parent")).expect("mkdir crate");
    fs::write(&generated_crate, "pub fn generated() {}\n").expect("write generated crate");
    run_git(&project_root, &["add", "crates/generated/src/lib.rs"]);

    specify_with_git_identity()
        .current_dir(&project_root)
        .args(["--format", "json", "slice", "merge", "run", "my-slice"])
        .assert()
        .success();

    let subject = run_git(&project_root, &["log", "-1", "--pretty=%s"]);
    assert_eq!(subject.trim(), "specify: merge my-slice");

    let committed_paths =
        run_git(&project_root, &["show", "--name-only", "--pretty=format:", "HEAD"]);
    let committed_paths: Vec<&str> =
        committed_paths.lines().filter(|line| !line.is_empty()).collect();
    assert!(
        committed_paths.iter().any(|path| path.starts_with(".specify/specs/")),
        "merge commit must include spec baselines, got {committed_paths:?}"
    );
    assert!(
        committed_paths.iter().any(|path| path.starts_with(".specify/archive/")),
        "merge commit must include archived slice, got {committed_paths:?}"
    );
    assert!(
        committed_paths.iter().all(
            |path| path.starts_with(".specify/specs/") || path.starts_with(".specify/archive/")
        ),
        "merge commit must not include generated residue, got {committed_paths:?}"
    );

    let status = run_git(&project_root, &["status", "--porcelain"]);
    assert!(
        status.contains("A  crates/generated/src/lib.rs"),
        "pre-staged generated crate must remain staged for execute residue commit, got:\n{status}"
    );
    assert!(
        status.contains("?? contracts/"),
        "opaque generated contracts must remain uncommitted, got:\n{status}"
    );
    assert!(
        !status
            .lines()
            .any(|line| { line.contains(".specify/specs/") || line.contains(".specify/archive/") }),
        "baseline-owned paths should be clean after merge auto-commit, got:\n{status}"
    );
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
