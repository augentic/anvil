//! `specify plan {add,remove,amend}` CLI tests — the L1.J write-side
//! commands.

use crate::support::*;

const EMPTY_PLAN: &str = "\
name: demo
slices: []
";

const WITH_DESCRIPTION: &str = "\
name: demo
slices:
  - name: foo
    project: default
    status: pending
    description: original
";

// -- plan add ---------------------------------------------------------

#[test]
fn plan_add_appends_pending_entry_json() {
    let project = Project::init();
    project.seed_plan(EMPTY_PLAN);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "add", "foo"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());

    assert_eq!(actual["action"], "create");
    assert_eq!(actual["entry"]["name"], "foo");
    assert_eq!(actual["entry"]["status"], "pending");
    assert_eq!(actual["entry"]["status-reason"], Value::Null);
    assert_eq!(actual["plan"]["name"], "demo");

    // Parse the persisted plan and assert the entry's fields rather than
    // substring-matching the raw YAML.
    let plan = load_plan(&project);
    let entry =
        plan.entries.iter().find(|e| e.name == "foo").expect("saved plan must contain entry `foo`");
    assert_eq!(entry.status, Status::Pending, "saved entry must be pending");

    assert_golden("create-foo.json", actual);
}

#[test]
fn plan_add_rejects_duplicate_name_text() {
    let project = Project::init();
    project.seed_plan(EMPTY_PLAN);

    add_pending_entry(&project, "foo");

    let assert =
        specify_cmd().current_dir(project.root()).args(["plan", "add", "foo"]).assert().failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("already contains a change"),
        "stderr should flag duplicate, got: {stderr:?}"
    );
}

#[test]
fn plan_add_rejects_invalid_name() {
    let project = Project::init();
    project.seed_plan(EMPTY_PLAN);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["plan", "add", "NotKebab"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(1));

    let saved = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
    assert!(!saved.contains("NotKebab"), "invalid name must not land in the plan:\n{saved}");
}

// -- plan remove ------------------------------------------------------

#[test]
fn plan_remove_drops_pending_entry() {
    let project = Project::init();
    project.seed_plan(
        "\
name: demo
slices:
  - name: a
    project: default
    status: pending
  - name: b
    project: default
    status: pending
",
    );

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "remove", "a"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["action"], "remove");
    assert_eq!(actual["entry"]["name"], "a");

    let saved = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
    assert!(!saved.contains("name: a"), "removed entry must not remain:\n{saved}");
    assert!(saved.contains("name: b"), "other entry must remain:\n{saved}");
}

#[test]
fn plan_remove_refuses_when_depended_on() {
    let project = Project::init();
    project.seed_plan(
        "\
name: demo
slices:
  - name: a
    project: default
    status: pending
  - name: b
    project: default
    status: pending
    depends-on: [a]
",
    );

    let assert =
        specify_cmd().current_dir(project.root()).args(["plan", "remove", "a"]).assert().failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("plan-remove-entry-referenced"),
        "stderr should name the validation code, got: {stderr:?}"
    );
}

// -- plan amend -------------------------------------------------------

#[test]
fn plan_amend_replaces_depends_on() {
    let project = Project::init();
    project.seed_plan(
        "\
name: demo
slices:
  - name: a
    project: default
    status: done
  - name: b
    project: default
    status: done
  - name: foo
    project: default
    status: pending
    depends-on: [a]
",
    );

    let assert = specify_cmd()
        .current_dir(project.root())
        .args([
            "--format",
            "json",
            "plan",
            "amend",
            "foo",
            "--depends-on",
            "a",
            "--depends-on",
            "b",
        ])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["action"], "amend");
    assert_eq!(actual["entry"]["name"], "foo");
    let deps = actual["entry"]["depends-on"].as_array().expect("deps array");
    let names: Vec<&str> = deps.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(names, ["a", "b"]);

    assert_golden("amend-replace-depends-on.json", actual);

    // Assert the persisted `depends-on` as a parsed list, not a pair of
    // raw-YAML substring probes.
    let plan = load_plan(&project);
    let entry =
        plan.entries.iter().find(|e| e.name == "foo").expect("amended entry must be present");
    assert_eq!(
        entry.depends_on,
        vec!["a", "b"],
        "amended depends-on must round-trip to disk in order"
    );
}

#[test]
fn plan_amend_clears_description() {
    let project = Project::init();
    project.seed_plan(WITH_DESCRIPTION);

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "foo", "--description", ""])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read");
    assert!(
        !saved.contains("description: original"),
        "original description should be gone:\n{saved}"
    );
}

#[test]
fn plan_amend_leaves_field_alone() {
    let project = Project::init();
    project.seed_plan(WITH_DESCRIPTION);

    // --depends-on (clear) but no --description; description must stay.
    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "foo", "--depends-on"])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read");
    assert!(saved.contains("description: original"), "description should be preserved:\n{saved}");
}

#[test]
fn plan_amend_on_missing_entry_fails() {
    let project = Project::init();
    project.seed_plan(SINGLE_PENDING);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "nope", "--description", "x"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8");
    assert!(
        stderr.contains("no slice named"),
        "stderr should mention missing change, got: {stderr:?}"
    );
}
