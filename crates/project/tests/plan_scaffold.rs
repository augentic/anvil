//! Overwrite policy for the public [`project::plan::scaffold`] gate.

use std::collections::BTreeMap;

use error::Error;
use project::plan::{Lifecycle, Plan, scaffold};

fn tmp_plan() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let path = tmp.path().join("plan.yaml");
    (tmp, path)
}

#[test]
fn fresh_scaffolds() {
    let (_tmp, path) = tmp_plan();
    let plan = scaffold(&path, "demo", BTreeMap::new(), false).expect("fresh scaffold");
    plan.save(&path).expect("save");
    assert!(path.exists());
}

#[test]
fn existing_refused_without_force() {
    let (_tmp, path) = tmp_plan();
    scaffold(&path, "demo", BTreeMap::new(), false).expect("fresh").save(&path).expect("save");

    let err = scaffold(&path, "other", BTreeMap::new(), false).expect_err("refuses overwrite");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "already-exists");
            assert!(detail.contains("--force"), "{detail}");
        }
        other => panic!("expected already-exists Diag, got {other}"),
    }
}

#[test]
fn force_replaces_pending() {
    let (_tmp, path) = tmp_plan();
    scaffold(&path, "demo", BTreeMap::new(), false).expect("fresh").save(&path).expect("save");

    let replaced =
        scaffold(&path, "renamed", BTreeMap::new(), true).expect("force replaces pending");
    assert_eq!(replaced.name.as_str(), "renamed");
    replaced.save(&path).expect("save");

    let loaded = Plan::load(&path).expect("load");
    assert_eq!(loaded.name.as_str(), "renamed");
    assert_eq!(loaded.lifecycle, Lifecycle::Pending);
}

#[test]
fn force_replaces_approved() {
    let (_tmp, path) = tmp_plan();
    scaffold(&path, "demo", BTreeMap::new(), false).expect("fresh").save(&path).expect("save");

    let mut plan = Plan::load(&path).expect("load");
    plan.lifecycle = Lifecycle::Approved;
    plan.save(&path).expect("save approved");

    let replaced =
        scaffold(&path, "renamed", BTreeMap::new(), true).expect("force replaces approved");
    assert_eq!(replaced.name.as_str(), "renamed");
    replaced.save(&path).expect("save");

    let loaded = Plan::load(&path).expect("load");
    assert_eq!(loaded.name.as_str(), "renamed");
    assert_eq!(loaded.lifecycle, Lifecycle::Pending);
}
