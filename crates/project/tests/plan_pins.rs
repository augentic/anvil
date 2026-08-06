//! Source `cid` pin close (RFC-86 D4 / D25).

use std::collections::BTreeMap;
use std::path::Path;

use project::plan::{Plan, SourceBinding, close_source_pins, file_cid, value_cid};
use project::snapshot::SnapshotId;
use project::workspace::Store;

fn binding_value(value: &str) -> SourceBinding {
    SourceBinding {
        adapter: "intent".into(),
        version: None,
        path: None,
        value: Some(value.into()),
        cid: None,
    }
}

fn binding_path(path: &str) -> SourceBinding {
    SourceBinding {
        adapter: "documentation".into(),
        version: None,
        path: Some(path.into()),
        value: None,
        cid: None,
    }
}

fn plan_with(sources: BTreeMap<String, SourceBinding>) -> Plan {
    Plan {
        name: "demo".into(),
        sources,
        entries: vec![],
    }
}

#[test]
fn value_binding_pins_one_file_tree() {
    let mut plan = plan_with(BTreeMap::from([("intent".into(), binding_value("hello"))]));
    close_source_pins(&mut plan, Path::new(".")).expect("close");
    let cid = plan.sources["intent"].cid.as_ref().expect("cid");
    assert_eq!(cid, &value_cid("hello"));
    assert_eq!(cid.as_str().len(), "sha256:".len() + 64);
}

#[test]
fn path_dir_matches_store_snapshot() {
    let root = tempfile::tempdir().expect("tempdir");
    let docs = root.path().join("docs");
    std::fs::create_dir_all(&docs).expect("mkdir");
    std::fs::write(docs.join("a.md"), b"alpha").expect("write");
    std::fs::write(docs.join("b.md"), b"beta").expect("write");

    let mut plan = plan_with(BTreeMap::from([(
        "docs".into(),
        binding_path(docs.to_str().expect("utf8")),
    )]));
    close_source_pins(&mut plan, root.path()).expect("close");
    let cid = plan.sources["docs"].cid.clone().expect("cid");

    let store = Store::new(root.path().join("snapshots"));
    let stored = store.snapshot(&docs).expect("store snapshot");
    assert_eq!(cid, stored);
}

#[test]
fn path_file_is_one_file_tree() {
    let root = tempfile::tempdir().expect("tempdir");
    let file = root.path().join("notes.md");
    std::fs::write(&file, b"body").expect("write");

    let mut plan = plan_with(BTreeMap::from([(
        "docs".into(),
        binding_path(file.to_str().expect("utf8")),
    )]));
    close_source_pins(&mut plan, root.path()).expect("close");
    assert_eq!(
        plan.sources["docs"].cid.as_ref(),
        Some(&file_cid("notes.md", b"body"))
    );
}

#[test]
fn missing_path_refuses() {
    let mut plan = plan_with(BTreeMap::from([(
        "docs".into(),
        binding_path("missing-docs"),
    )]));
    let err = close_source_pins(&mut plan, Path::new("/tmp")).expect_err("missing");
    assert!(err.to_string().contains("source-pin-missing"), "{err}");
}

#[test]
fn value_cid_is_stable_snapshot_id() {
    let a = value_cid("x");
    let b = SnapshotId::parse(a.as_str()).expect("parse");
    assert_eq!(a, b);
}
