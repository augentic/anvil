//! Source `cid` pin close (RFC-86 D4 / D25).

use std::collections::BTreeMap;
use std::path::Path;

use project::plan::{
    Plan, SourceBinding, close_source_pins, dir_cid, empty_cid, file_cid, value_cid,
};
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
fn value_binding_pins_file() {
    let mut plan = plan_with(BTreeMap::from([("intent".into(), binding_value("hello"))]));
    close_source_pins(&mut plan, Path::new(".")).expect("close");
    let cid = plan.sources["intent"].cid.as_ref().expect("cid");
    assert_eq!(cid, &value_cid("hello"));
    assert_eq!(cid.as_str().len(), "sha256:".len() + 64);
}

#[tokio::test]
async fn path_dir_matches_store() {
    let root = tempfile::tempdir().expect("tempdir");
    let docs = root.path().join("docs");
    std::fs::create_dir_all(&docs).expect("mkdir");
    std::fs::write(docs.join("a.md"), b"alpha").expect("write");
    std::fs::write(docs.join("b.md"), b"beta").expect("write");

    let mut plan =
        plan_with(BTreeMap::from([("docs".into(), binding_path(docs.to_str().expect("utf8")))]));
    close_source_pins(&mut plan, root.path()).expect("close");
    let cid = plan.sources["docs"].cid.clone().expect("cid");

    let store = Store::new(root.path().join("snapshots"));
    let stored = store.snapshot(&docs).await.expect("store snapshot");
    assert_eq!(cid, stored);
}

#[test]
fn path_file_file_tree() {
    let root = tempfile::tempdir().expect("tempdir");
    let file = root.path().join("notes.md");
    std::fs::write(&file, b"body").expect("write");

    let mut plan =
        plan_with(BTreeMap::from([("docs".into(), binding_path(file.to_str().expect("utf8")))]));
    close_source_pins(&mut plan, root.path()).expect("close");
    assert_eq!(plan.sources["docs"].cid.as_ref(), Some(&file_cid("notes.md", b"body")));
}

#[test]
fn missing_path_refuses() {
    let mut plan = plan_with(BTreeMap::from([("docs".into(), binding_path("missing-docs"))]));
    let err = close_source_pins(&mut plan, Path::new("/tmp")).expect_err("missing");
    assert!(err.to_string().contains("source-pin-missing"), "{err}");
}

#[test]
fn value_cid_stable_snapshot() {
    let a = value_cid("x");
    let b = SnapshotId::parse(a.as_str()).expect("parse");
    assert_eq!(a, b);
}

#[test]
fn missing_dir_shares_empty() {
    let root = tempfile::tempdir().expect("tempdir");
    let missing = root.path().join("no-specs");
    assert_eq!(dir_cid(&missing).expect("cid"), empty_cid());
}

#[tokio::test]
async fn dir_cid_matches_store() {
    let root = tempfile::tempdir().expect("tempdir");
    let specs = root.path().join("specs");
    let domain = specs.join("a");
    std::fs::create_dir_all(&domain).expect("mkdir");
    std::fs::write(domain.join("spec.md"), b"body").expect("write");

    let store = Store::new(root.path().join("snapshots"));
    assert_eq!(
        dir_cid(&specs).expect("dir cid"),
        store.snapshot(&specs).await.expect("store snapshot")
    );
}

/// RFC-105 D2: the pin walk applies the same membership as the
/// snapshot walk, so pin digest equals freeze digest for a tree with
/// `.gitignore` rules — and equals the identity of the tree that
/// never carried the ignored output at all.
#[tokio::test]
async fn dir_cid_honors_gitignore() {
    let root = tempfile::tempdir().expect("tempdir");
    let dirty = root.path().join("dirty");
    std::fs::create_dir_all(dirty.join("target")).expect("mkdir");
    std::fs::write(dirty.join("src.rs"), b"code").expect("write");
    std::fs::write(dirty.join(".gitignore"), b"target/\n").expect("write");
    std::fs::write(dirty.join("target/foo.o"), b"junk").expect("write");

    let clean = root.path().join("clean");
    std::fs::create_dir_all(&clean).expect("mkdir");
    std::fs::write(clean.join("src.rs"), b"code").expect("write");
    std::fs::write(clean.join(".gitignore"), b"target/\n").expect("write");

    let store = Store::new(root.path().join("snapshots"));
    let pinned = dir_cid(&dirty).expect("dir cid");
    assert_eq!(pinned, store.snapshot(&dirty).await.expect("store snapshot"));
    assert_eq!(pinned, dir_cid(&clean).expect("clean cid"), "ignored output leaves the identity");
}
