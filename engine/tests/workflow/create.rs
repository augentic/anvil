//! `specify plan create` CLI tests, the human-driven replay loop, and
//! the `--auto-approve` Gate-1 contract.

use crate::support::*;

// -- human-driven replay (the human-driven plan loop) -----------

#[test]
fn plan_human_replay_matches_fixture() {
    let project = Project::init();
    project.seed_plan(
        "\
name: demo
slices:
  - name: user-registration
    project: default
    status: done
",
    );
    let _lock = project.hold_plan_lock();

    specify_cmd()
        .current_dir(project.root())
        .args([
            "plan",
            "add",
            "registration-duplicate-email-crash",
            "--description",
            "Duplicate email submission returns 500 instead of 409. Modifies user-registration.",
        ])
        .assert()
        .success();

    specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();

    specify_cmd()
        .current_dir(project.root())
        .args([
            "plan",
            "amend",
            "registration-duplicate-email-crash",
            "--description",
            "Clarified scope",
        ])
        .assert()
        .success();

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "transition", "registration-duplicate-email-crash", "done"])
        .assert()
        .success();

    let actual = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
    let fixture_path = plan_fixtures().join("human-replay-final.yaml");

    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        fs::create_dir_all(plan_fixtures()).expect("mkdir plan fixtures");
        fs::write(&fixture_path, &actual).expect("write fixture");
        return;
    }

    let expected = fs::read_to_string(&fixture_path).unwrap_or_else(|err| {
        panic!(
            "fixture {} missing ({err}); regenerate via \
                 REGENERATE_GOLDENS=1 cargo nextest run --test plan",
            fixture_path.display()
        )
    });

    assert_eq!(
        actual,
        expected,
        "plan.yaml after replay diverged from fixture {}\n--- actual ---\n{actual}\n--- expected ---\n{expected}",
        fixture_path.display()
    );
}

#[test]
fn create_scaffolds_matches_golden() {
    let project = Project::init();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "create", "my-change"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());

    assert_eq!(actual["name"], "my-change");
    let plan_path = actual["plan"].as_str().expect("plan string");
    assert!(
        plan_path.ends_with("/plan.yaml"),
        "plan should end with /plan.yaml at the repo root, got: {plan_path}"
    );

    assert!(project.plan_path().exists(), "plan.yaml should be created");
    assert!(!project.root().join("change.md").exists(), "plan create must not write change.md");

    assert_golden("plan-create.json", actual);
}

#[test]
fn create_refuses_overwrite() {
    let project = Project::init();
    specify_cmd().current_dir(project.root()).args(["plan", "create", "first"]).assert().success();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "create", "second"])
        .assert()
        .failure();
    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "already-exists");
}

#[test]
fn plan_create_then_validate_passes_clean() {
    let project = Project::init();

    specify_cmd().current_dir(project.root()).args(["plan", "create", "fresh"]).assert().success();

    let assert =
        specify_cmd().current_dir(project.root()).args(["plan", "validate"]).assert().success();
    assert_eq!(assert.get_output().status.code(), Some(0));
    let stdout = std::str::from_utf8(&assert.get_output().stdout).expect("utf8");
    assert!(
        !stdout.contains("ERROR"),
        "freshly-scaffolded plan must pass `specify plan validate` with no errors, got:\n{stdout}"
    );
}

// -- plan create --auto-approve (auto-approve Gate-1 contract) ---------------------------

#[test]
fn create_auto_approve_stamps() {
    // auto-approve Gate-1 contract: `--auto-approve` is the operator's Gate-1 consent at
    // create time. The on-disk plan carries `lifecycle: approved`
    // directly (single atomic write — no transient `pending`
    // observable to readers) and the journal carries exactly one
    // `plan.transition.approved` event matching the post-create stamp.
    let project = Project::init();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "create", "fresh", "--auto-approve"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["name"], "fresh");
    assert_eq!(actual["lifecycle"], "approved");

    let on_disk = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
    assert!(
        on_disk.contains("lifecycle: approved"),
        "plan.yaml must carry `lifecycle: approved` after --auto-approve, got:\n{on_disk}"
    );
    assert!(
        !on_disk.contains("lifecycle: pending"),
        "no transient `lifecycle: pending` must remain on disk, got:\n{on_disk}"
    );

    let journal = project.root().join(".specify").join("journal.jsonl");
    let raw = fs::read_to_string(&journal).expect("read journal.jsonl");
    let lines: Vec<&str> = raw.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "exactly one journal event (plan.transition.approved) per --auto-approve create, got:\n{raw}"
    );
    assert!(
        lines[0].contains(r#""event":"plan.transition.approved""#),
        "first (and only) line must be plan.transition.approved, got:\n{}",
        lines[0]
    );
    assert!(
        lines[0].contains(r#""plan-name":"fresh""#),
        "plan-name must serialise kebab-case, got:\n{}",
        lines[0]
    );
    assert!(
        lines[0].contains(r#""actor":"operator""#),
        "--auto-approve is operator consent, so the stamp records actor: operator, got:\n{}",
        lines[0]
    );
}

#[test]
fn plan_create_auto_approve_idempotent() {
    // auto-approve Gate-1 contract: running `specify plan transition <name> approved`
    // after a successful `--auto-approve` create must be a no-op —
    // exit 0, no second `plan.transition.approved` event, plan.yaml
    // unchanged.
    let project = Project::init();

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "create", "fresh", "--auto-approve"])
        .assert()
        .success();
    let journal = project.root().join(".specify").join("journal.jsonl");
    let before = fs::read_to_string(&journal).expect("read journal.jsonl");
    let before_lines = before.lines().filter(|l| !l.is_empty()).count();
    let plan_before = fs::read_to_string(project.plan_path()).expect("read plan.yaml");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "transition", "fresh", "approved"])
        .assert()
        .success();
    let body = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(body["kind"], "plan");
    assert_eq!(
        body["previous"], "approved",
        "previous lifecycle must already be approved (no-op), got:\n{body}"
    );
    assert_eq!(body["current"], "approved");

    let plan_after = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
    assert_eq!(
        plan_before, plan_after,
        "plan.yaml must not change under the idempotent no-op transition"
    );
    let after = fs::read_to_string(&journal).expect("read journal.jsonl");
    let after_lines = after.lines().filter(|l| !l.is_empty()).count();
    assert_eq!(
        before_lines, after_lines,
        "explicit `transition approved` after --auto-approve must not append a second event"
    );
}

#[test]
fn plan_create_auto_approve_invalid_name() {
    // auto-approve Gate-1 contract: `--auto-approve` does NOT bypass validation. An
    // invalid (non-kebab) name refuses the create with the same
    // exit code and envelope as the post-create path; no `plan.yaml`
    // lands on disk and the journal stays untouched.
    let project = Project::init();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "create", "Bad_Name", "--auto-approve"])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(code, 1, "kebab-case violation surfaces via Error::Diag (exit 1)");
    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "change-name-not-kebab");

    assert!(
        !project.plan_path().exists(),
        "plan.yaml must not be written when --auto-approve fails validation"
    );
    let journal = project.root().join(".specify").join("journal.jsonl");
    assert!(
        !journal.exists(),
        "journal must stay empty when --auto-approve validation fails, found: {}",
        journal.display()
    );
}

#[test]
fn create_auto_approve_no_partial_events() {
    // auto-approve Gate-1 contract: validation failure under --auto-approve must not
    // surface a partial-state event sequence — no orphan
    // `plan.amend.authority-override` without the matching
    // `plan.transition.approved`, no half-written plan.yaml. An
    // unknown `--authority-override` slice (the cheapest validation
    // gate to trip on a fresh plan) must refuse the create and
    // leave the journal untouched.
    let project = Project::init();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args([
            "plan",
            "create",
            "fresh",
            "--auto-approve",
            "--authority-override",
            "ghost-slice",
            "criterion=runtime",
        ])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));

    assert!(
        !project.plan_path().exists(),
        "plan.yaml must not be written when --auto-approve + --authority-override fails"
    );
    let journal = project.root().join(".specify").join("journal.jsonl");
    assert!(
        !journal.exists(),
        "journal must stay empty on validation failure, found: {}",
        journal.display()
    );
}

#[test]
fn create_auto_approve_then_validate_passes() {
    // The empty-scaffold + `--auto-approve` combination must still
    // validate cleanly — `--auto-approve` is a Gate-1 consent flag,
    // not a validation bypass, but it also must not introduce any
    // new validation drift on the empty-scaffold path.
    let project = Project::init();

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "create", "fresh", "--auto-approve"])
        .assert()
        .success();

    let assert =
        specify_cmd().current_dir(project.root()).args(["plan", "validate"]).assert().success();
    assert_eq!(assert.get_output().status.code(), Some(0));
}
