use tempfile::tempdir;

use super::super::model::Status;
use super::super::{PLAN_EXAMPLE_YAML, change, change_with_deps, plan_with_changes};
use super::*;

#[test]
fn save_load_round_trips() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("plan.yaml");
    let original: Plan = serde_saphyr::from_str(PLAN_EXAMPLE_YAML).expect("parse plan fixture");
    original.save(&path).expect("save ok");
    let loaded = Plan::load(&path).expect("load ok");
    assert_eq!(loaded, original, "full plan should round-trip through save -> load");
}

#[test]
fn save_emits_trailing_newline() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("plan.yaml");
    let mut plan = plan_with_changes(vec![]);
    plan.name = "init".into();
    plan.save(&path).expect("save ok");

    let bytes = std::fs::read(&path).expect("read ok");
    assert!(!bytes.is_empty(), "saved file should not be empty");
    assert_eq!(*bytes.last().unwrap(), b'\n', "saved file should end with a newline");

    let content = std::str::from_utf8(&bytes).expect("utf8");
    assert!(content.contains("name: init"), "file should contain `name: init`, got:\n{content}");
}

#[test]
fn save_overwrites_atomically() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("plan.yaml");
    std::fs::write(&path, "garbage that should be overwritten").expect("write garbage");

    let mut plan = plan_with_changes(vec![change("only-entry", Status::Pending)]);
    plan.name = "fresh".into();
    plan.save(&path).expect("save ok");

    let loaded = Plan::load(&path).expect("load ok");
    assert_eq!(loaded, plan, "loaded plan should equal saved plan");

    let raw = std::fs::read_to_string(&path).expect("read ok");
    assert!(!raw.contains("garbage"), "pre-existing garbage content should be gone, got:\n{raw}");
}

#[test]
fn load_missing_returns_not_found() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("does-not-exist.yaml");
    let err = Plan::load(&path).expect_err("expected error on missing file");
    match err {
        Error::ArtifactNotFound { kind, path: p } => {
            assert_eq!(kind, "plan.yaml");
            assert_eq!(p, path);
        }
        other => panic!("expected Error::ArtifactNotFound, got {other:?}"),
    }
}

#[test]
fn load_no_trailing_newline() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("plan.yaml");
    std::fs::write(&path, "name: foo\nslices: []").expect("write without trailing newline");
    let plan = Plan::load(&path).expect("load ok");
    assert_eq!(plan.name, "foo");
    assert!(plan.entries.is_empty());
}

#[test]
fn load_rejects_rogue_top_level_field() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("plan.yaml");
    std::fs::write(&path, "name: foo\nrogue: true\nslices: []\n").expect("write rogue plan");

    let err = Plan::load(&path).expect_err("rogue top-level field should fail schema");
    let Error::Validation { code, detail } = err else {
        panic!("expected Error::Validation, got {err:?}");
    };
    assert_eq!(code, "plan-schema");
    assert!(detail.contains("/rogue"), "expected detail to mention `/rogue`, got {detail}");
}

#[test]
fn save_writes_kebab_case() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("plan.yaml");
    let mut plan =
        plan_with_changes(vec![change_with_deps("entry-one", Status::InProgress, &["foo"])]);
    plan.name = "demo".into();
    plan.save(&path).expect("save ok");

    let content = std::fs::read_to_string(&path).expect("read ok");
    assert!(content.contains("depends-on:"), "expected kebab-case `depends-on:`, got:\n{content}");
    assert!(
        content.contains("status: in-progress"),
        "expected kebab-case enum value `in-progress`, got:\n{content}"
    );
    assert!(
        !content.contains("depends_on"),
        "snake_case `depends_on` leaked onto disk, got:\n{content}"
    );
    assert!(
        !content.contains("in_progress"),
        "snake_case `in_progress` leaked onto disk, got:\n{content}"
    );
}

#[test]
fn save_no_intermediate_state() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("plan.yaml");

    let mut first = plan_with_changes(vec![]);
    first.name = "first".into();
    first.save(&path).expect("save first ok");

    let mut second = plan_with_changes(vec![change("new-entry", Status::Pending)]);
    second.name = "second".into();
    second.save(&path).expect("save second ok");

    let loaded = Plan::load(&path).expect("load ok");
    assert_eq!(loaded, second, "after a successful save, only the new content is observable");
    assert_ne!(loaded, first, "the previous plan should no longer be on disk");

    let bytes = std::fs::read(&path).expect("read bytes");
    assert!(!bytes.is_empty(), "saved file should not be empty after overwrite");
    assert_eq!(*bytes.last().unwrap(), b'\n', "overwritten file should still end with newline");
}
