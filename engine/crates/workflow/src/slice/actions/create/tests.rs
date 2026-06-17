use jiff::Timestamp;
use specify_error::Error;

use super::{CreateIfExists, create};
use crate::slice::{LifecycleStatus, SliceMetadata};

fn ts() -> Timestamp {
    "2026-06-01T00:00:00Z".parse().expect("valid timestamp")
}

#[test]
fn seeds_metadata_and_specs() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let made =
        create(tmp.path(), "my-slice", "omnia@1.0.0", CreateIfExists::Fail, ts()).expect("create");

    assert!(made.created, "fresh dir is created");
    assert!(!made.restarted);
    assert!(made.dir.join("specs").is_dir(), "specs/ scaffolded");
    assert!(SliceMetadata::path(&made.dir).exists(), "metadata.yaml written");
    assert_eq!(made.metadata.status, LifecycleStatus::Refining);
    assert_eq!(made.metadata.target, "omnia@1.0.0");
    assert_eq!(made.metadata.created_at, Some(ts()));

    let loaded = SliceMetadata::load(&made.dir).expect("reload");
    assert_eq!(loaded, made.metadata, "returned metadata matches what landed on disk");
}

#[test]
fn invalid_name_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let err = create(tmp.path(), "Bad_Name", "omnia@1.0.0", CreateIfExists::Fail, ts())
        .expect_err("non-kebab name");
    assert!(
        matches!(
            err,
            Error::Diag {
                code: "invalid-name",
                ..
            }
        ),
        "got {err:?}"
    );
    assert!(!tmp.path().join("Bad_Name").exists(), "rejected name leaves no dir");
}

#[test]
fn duplicate_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let _first =
        create(tmp.path(), "dup", "omnia@1.0.0", CreateIfExists::Fail, ts()).expect("first create");
    let err = create(tmp.path(), "dup", "omnia@1.0.0", CreateIfExists::Fail, ts())
        .expect_err("second create");
    assert!(
        matches!(
            err,
            Error::Diag {
                code: "slice-already-exists",
                ..
            }
        ),
        "got {err:?}"
    );
}

#[test]
fn continue_reuses_dir() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let _first = create(tmp.path(), "keep", "omnia@1.0.0", CreateIfExists::Fail, ts())
        .expect("first create");
    let reused = create(tmp.path(), "keep", "omnia@1.0.0", CreateIfExists::Continue, ts())
        .expect("continue");
    assert!(!reused.created, "continue reuses, does not create");
    assert!(!reused.restarted);
}

#[test]
fn restart_recreates_dir() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let first = create(tmp.path(), "redo", "omnia@1.0.0", CreateIfExists::Fail, ts())
        .expect("first create");
    let marker = first.dir.join("specs").join("scratch.txt");
    std::fs::write(&marker, b"stale").expect("write marker");

    let again =
        create(tmp.path(), "redo", "omnia@1.0.0", CreateIfExists::Restart, ts()).expect("restart");
    assert!(again.created);
    assert!(again.restarted, "restart replaces the directory");
    assert!(!marker.exists(), "restart wipes prior contents");
}
