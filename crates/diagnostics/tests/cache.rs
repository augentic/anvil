//! Pure cache-math behavior: project-id stability, content digesting,
//! and the verify-on-read digest sidecar — all root-parameterized, no
//! environment involved.

use std::path::Path;

use diagnostics::cache::{
    file_content_digest, project_id, read_store_meta, verify_store_entry, write_store_meta,
};

#[test]
fn project_ids_stable_distinct() {
    let a = project_id(Path::new("/some/project/a"));
    let b = project_id(Path::new("/some/project/b"));
    assert_ne!(a, b);
    assert_eq!(a, project_id(Path::new("/some/project/a")));
}

#[test]
fn sidecar_verifies_content() {
    let store = tempfile::tempdir().expect("store root");
    let entry = store.path().join("demo-target@1.2.0.wasm");
    let meta = store.path().join("demo-target@1.2.0.meta");
    std::fs::write(&entry, b"\0asm-component").expect("write component");

    let digest = file_content_digest(&entry);
    assert!(digest.starts_with("sha256:"));
    write_store_meta(&meta, &digest, Some("sha256:registry")).expect("write sidecar");
    assert_eq!(read_store_meta(&meta).as_deref(), Some(digest.as_str()));
    verify_store_entry(&entry, &meta).expect("unchanged entry verifies");

    std::fs::write(&entry, b"\0asm-component-changed").expect("mutate component");
    let mismatch = verify_store_entry(&entry, &meta).expect_err("drift must fail");
    assert_eq!(mismatch.recorded, digest);
    assert_eq!(mismatch.actual, file_content_digest(&entry));
}

#[test]
fn missing_sidecar_is_fail_open() {
    let store = tempfile::tempdir().expect("store root");
    let entry = store.path().join("demo-target@1.2.0.wasm");
    std::fs::write(&entry, b"\0asm-component").expect("write component");
    verify_store_entry(&entry, &store.path().join("demo-target@1.2.0.meta"))
        .expect("a legacy entry without a sidecar passes");
}
