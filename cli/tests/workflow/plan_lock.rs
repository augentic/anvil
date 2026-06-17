//! Dual-driving refusal and the `specify plan lock -- <cmd>` wrapper.
//!
//! The plan-state-writing verbs (`plan next`, per-entry
//! `plan transition`, `slice merge run`) probe
//! `<plan-root>/.specify/plan.lock` and refuse an unlocked driver with
//! `plan-lock-not-held` (exit 2); `Project::hold_plan_lock` stands in
//! for a driver session in those refusal tests. The `plan lock` verb is
//! the CLI-native acquirer: it holds the lock for the spawned child's
//! lifetime, passes the child's exit code through, refuses a busy lock
//! with `plan-lock-busy` (exit 2), and skips re-acquisition under
//! `SPECIFY_PLAN_LOCK_HELD=1`. This file is the named CLI-test
//! replacement for the retired `dual-driving-refused` eval scenario.

use crate::support::*;

/// Absolute path to the `specify` binary under test, used as the child
/// command for `plan lock -- specify …` round-trips.
fn specify_bin() -> String {
    assert_cmd::cargo::cargo_bin("specify").to_string_lossy().into_owned()
}

#[test]
fn gated_verbs_refuse_unlocked_driver() {
    // One case per gated verb: (seed, argv, status line that must
    // survive). The surviving status proves the refusal wrote no plan
    // state; the journal must stay absent because the lock probe fires
    // before any event bracket (`plan.entry.advanced`, the
    // `slice.merge.*` pair). The merge slice doesn't even need to exist
    // for the refusal path.
    let cases: [(&str, &[&str], &str); 4] = [
        (SINGLE_PENDING, &["plan", "next"], "status: pending"),
        (A_IN_PROGRESS, &["plan", "transition", "a", "done"], "status: in-progress"),
        (ALL_DONE, &["plan", "transition", "a", "--undo"], "status: done"),
        (A_IN_PROGRESS, &["slice", "merge", "run", "a"], "status: in-progress"),
    ];

    for (seed, args, surviving_status) in cases {
        let project = Project::init();
        project.seed_plan(seed);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json"])
            .args(args)
            .assert()
            .failure();
        assert_eq!(
            assert.get_output().status.code(),
            Some(2),
            "{args:?}: lock refusal is a validation exit"
        );
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "plan-lock-not-held", "{args:?}: stderr envelope: {stderr}");

        let plan = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
        assert!(
            plan.contains(surviving_status),
            "{args:?}: refusal must not write plan state: {plan}"
        );
        assert!(
            !project.root().join(".specify/journal.jsonl").exists(),
            "{args:?}: refusal must not journal"
        );
    }
}

#[test]
fn gate_one_approved_is_exempt() {
    // The plan-level Gate 1 stamp precedes any driver session — it must
    // succeed without the lock.
    let project = Project::init();
    project.seed_plan(SINGLE_PENDING);

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "transition", "demo", "approved"])
        .assert()
        .success();
}

#[test]
fn gated_verbs_pass_under_lock() {
    let project = Project::init();
    project.seed_plan(SINGLE_PENDING);
    let _lock = project.hold_plan_lock();

    specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "transition", "foo", "done"])
        .assert()
        .success();
}

#[test]
fn holds_so_child_can_advance() {
    // The wrapper holds the lock for the child's lifetime, so a nested
    // `plan next` (which probes the lock) passes and advances the entry
    // — the end-to-end proof that CLI-owned acquisition satisfies the
    // CLI-owned probe.
    let project = Project::init();
    project.seed_plan(SINGLE_PENDING);

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "lock", "--", &specify_bin(), "plan", "next"])
        .assert()
        .success();

    let plan = load_plan(&project);
    let foo = plan.entries.iter().find(|e| e.name == "foo").expect("entry foo");
    assert_eq!(foo.status, Status::InProgress, "child `plan next` must advance under the lock");

    // The lock is released once the wrapper's child exits, so a fresh
    // driver session can acquire it again.
    assert_eq!(
        specify_workflow::plan_lock::probe(&project.root().join(".specify/plan.lock"))
            .expect("probe"),
        specify_workflow::plan_lock::LockProbe::Unheld,
        "wrapper must release the lock on child exit"
    );
}

#[test]
fn busy_when_another_driver_holds() {
    // A second driver that finds the lock held fails fast with
    // `plan-lock-busy` (exit 2) before spawning the child.
    let project = Project::init();
    project.seed_plan(SINGLE_PENDING);
    let _held = project.hold_plan_lock();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "lock", "--", &specify_bin(), "--version"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2), "busy lock is a validation exit");
    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "plan-lock-busy", "stderr envelope: {stderr}");
}

#[cfg(unix)]
#[test]
fn passes_child_exit_code_through() {
    // The child's exit code is forwarded unchanged (here, a non-zero
    // code that is neither 0 nor a CLI exit code).
    let project = Project::init();
    project.seed_plan(SINGLE_PENDING);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["plan", "lock", "--", "sh", "-c", "exit 7"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(7), "wrapper must pass the child code");
}

#[test]
fn reentrant_skips_acquire_when_held() {
    // A breakout under a parent `/spec:execute` inherits
    // `SPECIFY_PLAN_LOCK_HELD=1`; the wrapper must skip acquisition
    // rather than deadlock on the lock the parent already holds. The
    // in-process guard stands in for the parent session.
    let project = Project::init();
    project.seed_plan(SINGLE_PENDING);
    let _held = project.hold_plan_lock();

    specify_cmd()
        .current_dir(project.root())
        .env("SPECIFY_PLAN_LOCK_HELD", "1")
        .args(["plan", "lock", "--", &specify_bin(), "plan", "next"])
        .assert()
        .success();

    let plan = load_plan(&project);
    let foo = plan.entries.iter().find(|e| e.name == "foo").expect("entry foo");
    assert_eq!(foo.status, Status::InProgress, "re-entrant child must still advance the entry");
}

#[test]
fn resolves_workspace_lock_via_plan_dir() {
    // The lock anchors at the plan root (`--plan-dir`), so slot-side
    // work locks the workspace, not the slot CWD.
    let project = Project::init();
    let workspace = tempdir().expect("workspace tempdir");

    specify_cmd()
        .current_dir(project.root())
        .args(["--plan-dir", workspace.path().to_str().expect("utf-8 path")])
        .args(["plan", "lock", "--", &specify_bin(), "--version"])
        .assert()
        .success();

    assert!(
        workspace.path().join(".specify/plan.lock").exists(),
        "lock must be created at the --plan-dir workspace root"
    );
    assert!(
        !project.root().join(".specify/plan.lock").exists(),
        "lock must not be created at the slot CWD"
    );
}
