//! `specify plan status` CLI tests — the read-only next-action
//! projection. The projection matrix is unit-tested in
//! `crates/workflow/src/change/plan/core/status/tests.rs`; this suite
//! pins the verb's wire shapes and its read-only contract.

use crate::support::*;

const APPROVED_IN_PROGRESS: &str = "\
name: demo
lifecycle: approved
slices:
  - name: a
    project: default
    status: in-progress
";

const APPROVED_ALL_DONE: &str = "\
name: demo
lifecycle: approved
slices:
  - name: a
    project: default
    status: done
";

/// Seed `<slice>/metadata.yaml` with the given lifecycle status.
fn seed_slice(project: &Project, name: &str, status: &str) {
    let slice_dir = project.slices_dir().join(name);
    fs::create_dir_all(&slice_dir).expect("mkdir slice");
    fs::write(slice_dir.join("metadata.yaml"), format!("target: omnia@1.0.0\nstatus: {status}\n"))
        .expect("write metadata.yaml");
}

/// Append raw journal lines (the projection only reads; tests own the
/// fixture file).
fn seed_journal(project: &Project, lines: &[&str]) {
    let path = project.root().join(".specify").join("journal.jsonl");
    let mut body = lines.join("\n");
    body.push('\n');
    fs::write(path, body).expect("write journal.jsonl");
}

#[test]
fn status_pending_plan_stops() {
    let project = Project::init();
    project.seed_plan(A_IN_PROGRESS);

    let assert =
        specify_cmd().current_dir(project.root()).args(["plan", "status"]).assert().success();
    let stdout = std::str::from_utf8(&assert.get_output().stdout).expect("utf8");
    assert!(
        stdout.contains("stop: plan-not-approved"),
        "unapproved plan must stop, got: {stdout:?}"
    );
    assert!(stdout.contains("hint:"), "stop must carry a hint line, got: {stdout:?}");
}

#[test]
fn status_active_refine_json() {
    let project = Project::init();
    project.seed_plan(APPROVED_IN_PROGRESS);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "status"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["next-action"], "refine a");
    assert_eq!(actual["action"], "refine");
    assert_eq!(actual["active"], "a");
    assert_golden("status-refine.json", actual);
}

#[test]
fn status_build_failure_stops() {
    let project = Project::init();
    project.seed_plan(APPROVED_IN_PROGRESS);
    seed_slice(&project, "a", "refined");
    seed_journal(
        &project,
        &[
            r#"{"timestamp":"2026-01-01T00:00:00Z","event":"plan.entry.advanced","payload":{"plan-name":"demo","slice-name":"a"}}"#,
            r#"{"timestamp":"2026-01-01T00:01:00Z","event":"slice.build.failed","payload":{"slice-name":"a","reason":"exhausted repair budget"}}"#,
        ],
    );

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "status"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["next-action"], "stop build-failed");
    assert_eq!(actual["stop"]["reason"], "build-failed");
    assert_eq!(actual["stop"]["detail"], "exhausted repair budget");
    assert_eq!(actual["resume"], "/spec:build a", "RM-15 re-entry point");
    assert_golden("status-build-failed.json", actual);

    let text =
        specify_cmd().current_dir(project.root()).args(["plan", "status"]).assert().success();
    let stdout = std::str::from_utf8(&text.get_output().stdout).expect("utf8");
    assert!(stdout.contains("stop: build-failed"), "got: {stdout:?}");
    assert!(stdout.contains("  slice: a"), "stop block must name the slice, got: {stdout:?}");
    assert!(stdout.contains("  detail: exhausted repair budget"), "got: {stdout:?}");
    assert!(stdout.contains("resume: /spec:build a"), "got: {stdout:?}");
}

#[test]
fn status_built_slice_dispatches_merge() {
    let project = Project::init();
    project.seed_plan(APPROVED_IN_PROGRESS);
    seed_slice(&project, "a", "built");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "status"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["next-action"], "merge a");
}

#[test]
fn status_drained_renders_finalize_hint() {
    let project = Project::init();
    project.seed_plan(APPROVED_ALL_DONE);

    let assert =
        specify_cmd().current_dir(project.root()).args(["plan", "status"]).assert().success();
    let stdout = std::str::from_utf8(&assert.get_output().stdout).expect("utf8");
    assert!(
        stdout.contains("drained — run /spec:finalize demo"),
        "drained must render the literal stop-conditions string, got: {stdout:?}"
    );
}

#[test]
fn status_is_read_only() {
    // The projection must not advance the plan, write the journal, or
    // touch slice state — `plan next` stays the only in-progress writer.
    let project = Project::init();
    project.seed_plan(A_DONE_B_PENDING);
    let plan_before = fs::read_to_string(project.plan_path()).expect("read plan");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "status"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(
        actual["next-action"], "stop plan-not-approved",
        "seeded plan has no lifecycle stamp"
    );

    let plan_after = fs::read_to_string(project.plan_path()).expect("read plan");
    assert_eq!(plan_before, plan_after, "plan status must not write plan.yaml");
    assert!(
        !project.root().join(".specify").join("journal.jsonl").exists(),
        "plan status must not journal"
    );
}
