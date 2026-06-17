//! `specify plan next` CLI tests.

use crate::support::*;

#[test]
fn plan_next_picks_first_pending_text() {
    let project = Project::init();
    project.seed_plan(A_DONE_B_PENDING);
    let _lock = project.hold_plan_lock();

    let assert =
        specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
    let stdout = std::str::from_utf8(&assert.get_output().stdout).expect("utf8");
    assert_eq!(stdout, "b\n", "text next should be bare '<name>\\n', got: {stdout:?}");
}

#[test]
fn plan_next_picks_first_pending_json() {
    let project = Project::init();
    project.seed_plan(A_DONE_B_PENDING);
    let _lock = project.hold_plan_lock();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "next"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["next"], "b");
    assert_eq!(actual["reason"], Value::Null);
    assert_eq!(actual["active"], Value::Null);
    assert_eq!(actual["project"], "default", "project should match seeded value");
    assert_eq!(actual["description"], Value::Null, "description should be present");
    assert!(
        actual.get("sources").is_some(),
        "sources field should be present in plan next response"
    );
    assert_golden("next-first-pending.json", actual);
}

#[test]
fn plan_next_journals_entry_advanced() {
    // `plan next` is the sole writer of per-entry `in-progress`; the
    // matching `plan.entry.advanced` event fires only on the write.
    let project = Project::init();
    project.seed_plan(A_DONE_B_PENDING);
    let _lock = project.hold_plan_lock();

    specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();

    let journal = project.root().join(".specify").join("journal.jsonl");
    let raw = fs::read_to_string(&journal).expect("read journal.jsonl");
    let lines: Vec<&str> = raw.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1, "exactly one event per fresh advance, got:\n{raw}");
    assert!(
        lines[0].contains(r#""event":"plan.entry.advanced""#),
        "advance must journal plan.entry.advanced, got:\n{}",
        lines[0]
    );
    assert!(lines[0].contains(r#""plan-name":"demo""#), "got:\n{}", lines[0]);
    assert!(lines[0].contains(r#""slice-name":"b""#), "got:\n{}", lines[0]);

    // Re-running `plan next` returns the active entry unchanged — no
    // second advance event, so probes can read "did not advance"
    // from the journal window.
    specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
    let raw_after = fs::read_to_string(&journal).expect("read journal.jsonl");
    assert_eq!(
        raw_after.lines().filter(|l| !l.is_empty()).count(),
        1,
        "returning the active entry must not append a second event, got:\n{raw_after}"
    );
}

#[test]
fn plan_next_drained_no_journal() {
    let project = Project::init();
    project.seed_plan(ALL_DONE);
    let _lock = project.hold_plan_lock();

    specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
    assert!(
        !project.root().join(".specify").join("journal.jsonl").exists(),
        "a drained plan must not journal plan.entry.advanced"
    );
}

#[test]
fn plan_next_reports_in_progress() {
    let project = Project::init();
    project.seed_plan(A_IN_PROGRESS);
    let _lock = project.hold_plan_lock();

    let text = specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
    let stdout = std::str::from_utf8(&text.get_output().stdout).expect("utf8");
    assert!(stdout.contains('a'), "text output should mention 'a': {stdout:?}");

    let json = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "next"])
        .assert()
        .success();
    let actual = parse_stdout(&json.get_output().stdout, project.root());
    assert_eq!(actual["next"], Value::Null);
    assert_eq!(actual["reason"], "in-progress");
    assert_eq!(actual["active"], "a");
    assert_golden("next-in-progress.json", actual);
}

#[test]
fn plan_next_all_done_text() {
    let project = Project::init();
    project.seed_plan(ALL_DONE);
    let _lock = project.hold_plan_lock();

    let text = specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
    let stdout = std::str::from_utf8(&text.get_output().stdout).expect("utf8");
    assert!(stdout.contains("drained"), "drained text should mention drained, got: {stdout:?}");

    let json = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "next"])
        .assert()
        .success();
    let actual = parse_stdout(&json.get_output().stdout, project.root());
    assert_eq!(actual["reason"], "drained");
    assert_eq!(actual["next"], Value::Null);
    assert_eq!(actual["active"], Value::Null);
    assert_golden("next-all-done.json", actual);
}
