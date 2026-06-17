//! `specify plan transition` CLI tests: per-entry edges, undo, the
//! plan-level Gate-1 stamp, and the retired-state rejections.

use crate::support::*;

const SINGLE_IN_PROGRESS: &str = "\
name: demo
slices:
  - name: foo
    project: default
    status: in-progress
";

const SINGLE_DONE: &str = "\
name: demo
slices:
  - name: foo
    project: default
    status: done
";

#[test]
fn plan_transition_legal_edge_json() {
    // Post-2.0 the only legal per-entry transition is
    // `InProgress -> Done`.
    let project = Project::init();
    project.seed_plan(SINGLE_IN_PROGRESS);
    let _lock = project.hold_plan_lock();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "transition", "foo", "done"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());

    assert_eq!(actual["name"], "foo");
    assert_eq!(actual["current"], "done");
    assert_eq!(actual["previous"], "in-progress");
    assert_eq!(actual["kind"], "entry");

    assert_golden("transition-in-progress-to-done.json", actual);
}

#[test]
fn plan_transition_rejects_illegal_edge() {
    let project = Project::init();
    project.seed_plan(SINGLE_DONE);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["plan", "transition", "foo", "pending"])
        .assert()
        .failure();
    let code = assert.get_output().status.code();
    assert!(
        code == Some(1) || code == Some(2),
        "illegal transition should be rejected (exit 1 or 2), got: {code:?}"
    );
    let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8");
    assert!(
        stderr.to_lowercase().contains("transition")
            || stderr.contains("plan add")
            || stderr.contains("plan next")
            || stderr.contains("argument"),
        "stderr should mention the rejected transition, got: {stderr:?}"
    );
}

#[test]
fn transition_undo_done_to_in_progress() {
    let project = Project::init();
    project.seed_plan(SINGLE_DONE);
    let _lock = project.hold_plan_lock();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "transition", "foo", "--undo"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["kind"], "undo");
    assert_eq!(actual["name"], "foo");
    assert_eq!(actual["previous"], "done");
    assert_eq!(actual["current"], "in-progress");
    assert_eq!(actual["undo"]["from"], "done");
    assert_eq!(actual["undo"]["to"], "in-progress");

    let plan_after = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
    assert!(plan_after.contains("status: in-progress"), "plan.yaml: {plan_after}");

    let journal = fs::read_to_string(project.root().join(".specify").join("journal.jsonl"))
        .expect("read journal.jsonl");
    let last = journal.lines().rfind(|l| !l.is_empty()).expect("journal line");
    assert!(
        last.contains(r#""event":"plan.transition.undone""#),
        "undo must emit plan.transition.undone, got:\n{last}"
    );
    assert!(last.contains(r#""from":"done""#), "from in payload: {last}");
    assert!(last.contains(r#""to":"in-progress""#), "to in payload: {last}");
}

#[test]
fn undo_in_progress_to_pending_refuses() {
    let project = Project::init();
    project.seed_plan(SINGLE_IN_PROGRESS);
    let _lock = project.hold_plan_lock();

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "transition", "foo", "--undo"])
        .assert()
        .success();

    let plan_mid = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
    assert!(plan_mid.contains("status: pending"), "plan.yaml after first undo: {plan_mid}");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["plan", "transition", "foo", "--undo"])
        .assert()
        .failure();
    let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8");
    assert!(
        stderr.contains("pending"),
        "undo-from-pending stderr should mention `pending`, got: {stderr:?}"
    );
}

#[test]
fn transition_plan_level_approved() {
    // workflow §The Plan: `specify plan transition <plan-name>
    // approved` is the operator-stamped Gate 1 transition. The plan
    // name on the wire matches `plan.yaml.name`.
    let project = Project::init();
    project.seed_plan(SINGLE_PENDING);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "transition", "demo", "approved"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["kind"], "plan");
    assert_eq!(actual["name"], "demo");
    assert_eq!(actual["previous"], "pending");
    assert_eq!(actual["current"], "approved");

    assert_golden("transition-plan-approved.json", actual);
}

#[test]
fn transition_rejects_per_entry_in_progress() {
    // Per-entry `in-progress` is owned by `plan next`. `plan transition`
    // must reject the request with an argument-shape error (exit 2).
    let project = Project::init();
    project.seed_plan(SINGLE_PENDING);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["plan", "transition", "foo", "in-progress"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8");
    assert!(stderr.contains("plan next"), "stderr should point at `plan next`, got: {stderr:?}");
}

#[test]
fn plan_transition_rejects_retired_states() {
    // `blocked`, `failed`, and `skipped` are not valid per-entry states.
    // Each must be rejected with the same argument-shape error.
    let project = Project::init();
    project.seed_plan(SINGLE_PENDING);

    for invalid in ["blocked", "failed", "skipped"] {
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["plan", "transition", "foo", invalid])
            .assert()
            .failure();
        assert_eq!(
            assert.get_output().status.code(),
            Some(2),
            "invalid state `{invalid}` must yield exit 2"
        );
    }
}

// `plan transition <name> failed --reason <text>` is not a valid form —
// there is no per-entry `failed` state; see
// `plan_transition_rejects_retired_states` above.

#[test]
fn transition_rejects_unknown_reason() {
    // `--reason` is not a valid flag (no per-entry state accepts a
    // reason). Clap surfaces unknown flags as exit 2 with `--reason`
    // named in stderr.
    let project = Project::init();
    project.seed_plan(SINGLE_PENDING);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["plan", "transition", "foo", "in-progress", "--reason", "x"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8");
    assert!(stderr.contains("--reason"), "stderr should mention '--reason', got: {stderr:?}");
}

// Re-entry to `pending` retired with the per-entry status purge
// (the 2.0 collapse removed the per-entry enum to `pending | in-progress | done`).
