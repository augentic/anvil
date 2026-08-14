//! Source `cid` pin close (RFC-86 D4 / D25).

use std::collections::BTreeMap;
use std::path::Path;

use project::adapter::catalog::Pin;
use project::plan::{
    Plan, SourceBinding, close_source_pins, dir_cid, empty_cid, file_cid, value_cid,
};
use project::snapshot::SnapshotId;
use project::workspace::Store;

fn binding_value(value: &str) -> SourceBinding {
    SourceBinding::intent(Pin::emery("intent", semver::Version::new(0, 12, 0)), value)
}

fn binding_path(path: &str) -> SourceBinding {
    SourceBinding {
        adapter: Pin::emery("documentation", semver::Version::new(0, 12, 0)),
        locator: Some(path.into()),
        value: None,
        cid: None,
    }
}

fn plan_with(sources: BTreeMap<String, SourceBinding>) -> Plan {
    let mut plan = Plan::named("demo");
    plan.sources = sources;
    plan
}

#[test]
fn value_binding_skips_cid() {
    let mut plan = plan_with(BTreeMap::from([("intent".into(), binding_value("hello"))]));
    close_source_pins(&mut plan, Path::new(".")).expect("close");
    assert!(plan.sources["intent"].cid.is_none());
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

#[tokio::test]
async fn ignore_policy_parity() {
    let root = tempfile::tempdir().expect("tempdir");
    let tree = root.path().join("tree");
    std::fs::create_dir_all(tree.join(".emery/specs")).expect("mkdir specs");
    std::fs::create_dir_all(tree.join(".emery/change")).expect("mkdir change");
    std::fs::create_dir_all(tree.join(".git")).expect("mkdir git");
    std::fs::write(tree.join(".emery/project.yaml"), b"name: demo\n").expect("project.yaml");
    std::fs::write(tree.join(".emery/specs/a.md"), b"spec\n").expect("spec");
    std::fs::write(tree.join(".emery/change/plan.yaml"), b"name: demo\n").expect("plan");
    std::fs::write(tree.join(".git/config"), b"[core]\n").expect("git");
    std::fs::write(tree.join("src.rs"), b"fn main() {}\n").expect("src");

    let store = Store::new(root.path().join("snapshots"));
    assert_eq!(
        dir_cid(&tree).expect("dir cid"),
        store.snapshot(&tree).await.expect("store snapshot")
    );
}

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
