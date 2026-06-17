use std::collections::BTreeMap;

use super::super::model::{
    Entry, Lifecycle, Patch, SliceAuthorityOverride, SliceSourceBinding, SourceBinding, Status,
};
use super::super::{change, plan_with_changes};
use super::*;

#[test]
fn amend_deps() {
    let mut plan = plan_with_changes(vec![change("a", Status::Pending), {
        let mut b = change("b", Status::Pending);
        b.depends_on = vec!["a".into()];
        b
    }]);
    let patch = EntryPatch {
        depends_on: Some(vec![]),
        ..EntryPatch::default()
    };
    plan.amend("b", patch).expect("amend ok");
    let b = plan.entries.iter().find(|c| c.name == "b").unwrap();
    assert!(b.depends_on.is_empty(), "depends_on should be replaced with empty vec");
}

#[test]
fn amend_description_three_way() {
    let mut plan = plan_with_changes(vec![Entry {
        name: "foo".into(),
        project: Some("default".into()),
        status: Status::Pending,
        depends_on: vec![],
        sources: vec![],
        context: vec![],
        description: Some("original".into()),
        divergence: None,
        disagreements: Vec::new(),
        authority_override: SliceAuthorityOverride::default(),
    }]);

    plan.amend("foo", EntryPatch::default()).expect("amend none ok");
    assert_eq!(
        plan.entries[0].description.as_deref(),
        Some("original"),
        "None description must leave description unchanged"
    );

    plan.amend(
        "foo",
        EntryPatch {
            description: Patch::Clear,
            ..EntryPatch::default()
        },
    )
    .expect("amend clear ok");
    assert_eq!(
        plan.entries[0].description, None,
        "Patch::Clear description must clear description"
    );

    plan.amend(
        "foo",
        EntryPatch {
            description: Patch::Set("new".into()),
            ..EntryPatch::default()
        },
    )
    .expect("amend replace ok");
    assert_eq!(
        plan.entries[0].description.as_deref(),
        Some("new"),
        "Patch::Set(s) description must replace description"
    );
}

#[test]
fn amend_leaves_unchanged() {
    let plan = Plan {
        name: "test".into(),
        lifecycle: Lifecycle::Pending,
        sources: {
            let mut m = BTreeMap::new();
            m.insert("a".to_string(), SourceBinding::path("typescript", "/path/a"));
            m
        },
        entries: vec![
            Entry {
                name: "foo".into(),
                project: Some("default".into()),
                status: Status::Pending,
                depends_on: vec![],
                sources: vec![SliceSourceBinding::bare("a")],
                context: vec![],
                description: Some("d".into()),
                divergence: None,
                disagreements: Vec::new(),
                authority_override: SliceAuthorityOverride::default(),
            },
            change("b", Status::Pending),
            change("x", Status::Pending),
        ],
    };
    let mut plan = plan;
    let patch = EntryPatch {
        depends_on: Some(vec!["x".into()]),
        ..EntryPatch::default()
    };
    plan.amend("foo", patch).expect("amend ok");
    let foo = plan.entries.iter().find(|c| c.name == "foo").unwrap();
    assert_eq!(foo.depends_on, vec!["x".to_string()]);
    assert_eq!(foo.sources, vec![SliceSourceBinding::bare("a")], "sources untouched");
    assert_eq!(foo.description.as_deref(), Some("d"), "description untouched");
}

#[test]
fn amend_missing_entry() {
    let mut plan = plan_with_changes(vec![change("foo", Status::Pending)]);
    let err = plan.amend("nonexistent", EntryPatch::default()).expect_err("missing entry must Err");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "plan-entry-not-found");
            assert!(detail.contains("nonexistent"), "message should mention name, got: {detail}");
        }
        other => panic!("expected Error::Diag, got {other:?}"),
    }
}

#[test]
fn amend_rejects_cycle() {
    let mut plan =
        plan_with_changes(vec![change("a", Status::Pending), change("b", Status::Pending)]);

    plan.amend(
        "a",
        EntryPatch {
            depends_on: Some(vec!["b".into()]),
            ..EntryPatch::default()
        },
    )
    .expect("a -> [b] is acyclic; amend ok");

    let err = plan
        .amend(
            "b",
            EntryPatch {
                depends_on: Some(vec!["a".into()]),
                ..EntryPatch::default()
            },
        )
        .expect_err("introducing cycle must Err");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "plan-amend-validation-failed");
            assert!(
                detail.contains("plan validation failed after amend"),
                "expected amend rollback message, got: {detail}"
            );
        }
        other => panic!("expected Error::Diag, got {other:?}"),
    }

    let b = plan.entries.iter().find(|c| c.name == "b").unwrap();
    assert!(
        b.depends_on.is_empty(),
        "b.depends_on must be unchanged after failed amend, got {:?}",
        b.depends_on
    );
}

#[test]
fn amend_project_three_way() {
    let mut plan = plan_with_changes(vec![Entry {
        name: "foo".into(),
        project: Some("alpha".into()),
        status: Status::Pending,
        depends_on: vec![],
        sources: vec![],
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: SliceAuthorityOverride::default(),
    }]);

    plan.amend("foo", EntryPatch::default()).expect("amend none ok");
    assert_eq!(
        plan.entries[0].project.as_deref(),
        Some("alpha"),
        "None must leave project unchanged"
    );

    plan.amend(
        "foo",
        EntryPatch {
            project: Patch::Set("beta".into()),
            ..EntryPatch::default()
        },
    )
    .expect("amend replace ok");
    assert_eq!(
        plan.entries[0].project.as_deref(),
        Some("beta"),
        "Patch::Set(s) must replace project"
    );

    plan.amend(
        "foo",
        EntryPatch {
            project: Patch::Clear,
            ..EntryPatch::default()
        },
    )
    .expect("amend clear ok");
    assert_eq!(plan.entries[0].project, None, "Patch::Clear must clear project");
}

#[test]
fn amend_context_replace() {
    let mut entry = change("foo", Status::Pending);
    entry.context = vec!["old/path.yaml".into()];
    let mut plan = plan_with_changes(vec![entry]);

    plan.amend(
        "foo",
        EntryPatch {
            context: Some(vec!["new/path.yaml".into(), "another.md".into()]),
            ..EntryPatch::default()
        },
    )
    .expect("amend ok");
    assert_eq!(
        plan.entries[0].context,
        vec!["new/path.yaml", "another.md"],
        "amend must replace context wholesale"
    );
}

#[test]
fn amend_context_none_unchanged() {
    let mut entry = change("foo", Status::Pending);
    entry.context = vec!["keep/this.yaml".into()];
    let mut plan = plan_with_changes(vec![entry]);

    plan.amend("foo", EntryPatch::default()).expect("amend ok");
    assert_eq!(
        plan.entries[0].context,
        vec!["keep/this.yaml"],
        "None context must leave field unchanged"
    );
}
