use super::super::model::SliceAuthorityOverride;
use super::super::{change, change_with_deps, plan_with_changes};
use super::*;

#[test]
fn create_forces_pending() {
    let mut plan = plan_with_changes(vec![]);
    let incoming = Entry {
        name: "foo".into(),
        project: Some("default".into()),
        // Even an entry that arrives with `Done` (the only other
        // legal status post-2.0) must be re-stamped to `Pending`
        // by `Plan::create` — the single-writer rule on the
        // per-entry status state machine.
        status: Status::Done,
        depends_on: vec![],
        sources: vec![],
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: SliceAuthorityOverride::default(),
    };
    plan.create(incoming).expect("create ok");
    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].name, "foo");
    assert_eq!(
        plan.entries[0].status,
        Status::Pending,
        "create must force status to Pending regardless of input"
    );
}

#[test]
fn create_rejects_duplicate() {
    let mut plan = plan_with_changes(vec![change("foo", Status::Pending)]);
    let dup = change("foo", Status::Pending);
    let err = plan.create(dup).expect_err("duplicate must be rejected");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "plan-entry-duplicate-name");
            assert!(
                detail.contains("already contains") && detail.contains("foo"),
                "unexpected message: {detail}"
            );
        }
        other => panic!("expected Error::Diag, got {other:?}"),
    }
    assert_eq!(plan.entries.len(), 1, "plan must still have exactly one entry");
}

#[test]
fn create_rejects_bad_name() {
    let mut plan = plan_with_changes(vec![]);
    let bad = change("Bad-Name", Status::Pending);
    let err = plan.create(bad).expect_err("invalid name must be rejected");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "invalid-name");
            assert!(detail.contains("kebab-case"), "expected kebab-case in detail, got: {detail}");
        }
        other => panic!("expected invalid-name diag, got {other:?}"),
    }
    assert!(plan.entries.is_empty(), "plan must remain untouched after invalid name");
}

#[test]
fn create_rejects_unknown_depends_on() {
    let mut plan = plan_with_changes(vec![
        change("a", Status::Pending),
        change_with_deps("b", Status::Pending, &["a"]),
    ]);
    let c = change_with_deps("c", Status::Pending, &["does-not-exist"]);
    let err = plan.create(c).expect_err("unknown depends-on must roll back");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "plan-create-validation-failed");
            assert!(
                detail.contains("plan validation failed after create"),
                "rollback message missing, got: {detail}"
            );
        }
        other => panic!("expected Error::Diag, got {other:?}"),
    }
    assert_eq!(plan.entries.len(), 2, "plan must still have only its original entries");
    let names: Vec<&str> = plan.entries.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["a", "b"], "existing entries must be untouched");
}

#[test]
fn create_rolls_back_on_failure() {
    let mut plan = plan_with_changes(vec![change("foo", Status::Pending)]);
    let bar = change_with_deps("bar", Status::Pending, &["nonexistent"]);
    let err = plan.create(bar).expect_err("must Err");
    assert!(matches!(err, Error::Diag { code, .. } if code == "plan-create-validation-failed"));
    assert_eq!(plan.entries.len(), 1, "plan length unchanged after rollback");
    assert_eq!(plan.entries[0].name, "foo");
    assert_eq!(plan.entries[0].status, Status::Pending);
    assert!(plan.entries[0].depends_on.is_empty());
}

#[test]
fn create_allows_omitted_project() {
    // A slice may omit `project`; it resolves to the sole topology
    // project at read time. `Plan::create` (no topology) must accept it.
    let mut plan = plan_with_changes(vec![]);
    let entry = Entry {
        name: "no-project".into(),
        project: None,
        status: Status::Pending,
        depends_on: vec![],
        sources: vec![],
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: SliceAuthorityOverride::default(),
    };
    plan.create(entry).expect("create must accept an entry that omits project");
    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].project, None);
}

#[test]
fn create_preserves_context() {
    let mut plan = plan_with_changes(vec![]);
    let entry = Entry {
        name: "with-ctx".into(),
        project: Some("default".into()),
        status: Status::Pending,
        depends_on: vec![],
        sources: vec![],
        context: vec!["contracts/http/foo.yaml".into()],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: SliceAuthorityOverride::default(),
    };
    plan.create(entry).expect("create ok");
    assert_eq!(
        plan.entries[0].context,
        vec!["contracts/http/foo.yaml"],
        "create must preserve context"
    );
}

#[test]
fn create_rejects_bad_context() {
    let mut plan = plan_with_changes(vec![]);
    let entry = Entry {
        name: "bad-ctx".into(),
        project: Some("default".into()),
        status: Status::Pending,
        depends_on: vec![],
        sources: vec![],
        context: vec!["../escape".into()],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: SliceAuthorityOverride::default(),
    };
    let err = plan.create(entry).expect_err("invalid context path must be rejected");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "plan-create-validation-failed");
            assert!(
                detail.contains("context-path-invalid") || detail.contains(".."),
                "error should mention context path issue, got: {detail}"
            );
        }
        other => panic!("expected Error::Diag, got {other:?}"),
    }
    assert!(plan.entries.is_empty(), "rollback must remove the entry");
}

#[test]
fn init_empty_plan() {
    let plan = Plan::init("platform-v2", BTreeMap::new()).expect("init ok");
    assert_eq!(plan.name, "platform-v2");
    assert!(plan.sources.is_empty(), "sources should default to empty");
    assert!(plan.entries.is_empty(), "changes should default to empty");
}

#[test]
fn init_preserves_sources() {
    let mut sources = BTreeMap::new();
    sources.insert("monolith".to_string(), SourceBinding::path("typescript", "/path/to/legacy"));
    sources.insert(
        "orders".to_string(),
        SourceBinding::path("typescript", "git@github.com:org/orders.git"),
    );
    sources.insert(
        "payments".to_string(),
        SourceBinding::path("typescript", "git@github.com:org/payments.git"),
    );

    let plan = Plan::init("big", sources.clone()).expect("init ok");
    assert_eq!(plan.sources, sources, "init must preserve the sources map verbatim");
    assert_eq!(plan.sources.len(), 3);
}

#[test]
fn init_rejects_bad_name() {
    let err = Plan::init("BAD_NAME", BTreeMap::new()).expect_err("invalid name must Err");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "invalid-name");
            assert!(detail.contains("kebab-case"), "expected kebab-case in detail, got: {detail}");
        }
        other => panic!("expected invalid-name diag, got {other:?}"),
    }
}

#[test]
fn init_accepts_kebab_case() {
    let plan = Plan::init("a-b-c", BTreeMap::new()).expect("kebab name accepted");
    assert_eq!(plan.name, "a-b-c");
}

#[test]
fn init_validates() {
    let plan = Plan::init("foo", BTreeMap::new()).expect("init ok");
    let findings = plan.validate(None, None);
    assert!(
        findings.is_empty(),
        "freshly-scaffolded plan must pass validation, got: {findings:#?}"
    );
}
