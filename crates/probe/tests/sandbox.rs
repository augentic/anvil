//! Persistent-sandbox gates: the single-writer lock and lifecycle
//! helpers.

use probe::sandbox;
use tempfile::TempDir;

#[test]
fn single_writer_excludes_a_second_eval() {
    let tmp = TempDir::new().expect("tempdir");
    let sandbox = tmp.path().join("sandbox");

    let guard = sandbox::single_writer(&sandbox).expect("first writer locks");
    let err = sandbox::single_writer(&sandbox)
        .expect_err("a second concurrent eval over the same sandbox refuses");
    assert!(format!("{err:#}").contains("already running"), "{err:#}");

    drop(guard);
    sandbox::single_writer(&sandbox).expect("the released lock is reusable");
}

#[test]
fn require_refuses_uninitialised() {
    let tmp = TempDir::new().expect("tempdir");
    let err = sandbox::require(tmp.path()).expect_err("no project.yaml refuses");
    assert!(format!("{err:#}").contains("not initialised"), "{err:#}");
}

#[test]
fn replace_resets_the_tree() {
    let tmp = TempDir::new().expect("tempdir");
    let sandbox = tmp.path().join("sandbox");
    std::fs::create_dir_all(sandbox.join("stale")).expect("mkdir");

    let root = sandbox::replace(&sandbox).expect("replace");
    assert!(!root.join("stale").exists(), "previous contents are gone");
}
