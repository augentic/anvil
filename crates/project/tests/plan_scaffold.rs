//! Overwrite policy for the public [`project::plan::scaffold`] gate.

use std::collections::BTreeMap;

use error::Error;
use project::plan::{Plan, scaffold};

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
            assert_eq!(code, "plan-already-exists");
            assert!(detail.contains("--force"), "{detail}");
        }
        other => panic!("expected plan-already-exists Diag, got {other}"),
    }
}

#[test]
fn force_replaces() {
    let (_tmp, path) = tmp_plan();
    scaffold(&path, "demo", BTreeMap::new(), false).expect("fresh").save(&path).expect("save");

    let replaced = scaffold(&path, "renamed", BTreeMap::new(), true).expect("force replaces");
    assert_eq!(replaced.name.as_str(), "renamed");
    replaced.save(&path).expect("save");

    let loaded = Plan::load(&path).expect("load");
    assert_eq!(loaded.name.as_str(), "renamed");
    assert!(loaded.entries.is_empty());
}
