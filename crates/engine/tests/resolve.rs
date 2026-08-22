//! Resolver integration: the verify-on-read gate and the cache probe
//! at the crate's public surface, over the scripted in-memory store —
//! entry bytes in the blobstore, the digest sidecar in keyvalue.

#[path = "support/storage.rs"]
mod storage;

use emery_diagnostics::cache::content_digest;
use emery_engine::handler::ExecutionPaths;
use emery_engine::resolve::Axis;
use emery_engine::resolve::resolver::locate;
use storage::Memory;

const COMPONENT: &[u8] = b"\0asm-demo";

const fn version() -> semver::Version {
    semver::Version::new(1, 2, 0)
}

// Seed a store entry for `demo@1.2.0`, its sidecar recording `digest`.
fn seeded_store(digest: &str) -> Memory {
    let store = Memory::default();
    store.insert_object("store", "demo@1.2.0.wasm", COMPONENT);
    store.insert_state("store/demo@1.2.0.meta", format!("tree_digest: {digest}\n").as_bytes());
    store
}

#[tokio::test]
async fn verified_pin_locates() {
    let store = seeded_store(&content_digest(COMPONENT));

    let location =
        locate(Axis::Source, "demo", Some(&version()), &store, &ExecutionPaths::deployed())
            .await
            .expect("a verified pin resolves");

    assert_eq!(location.object(), "demo@1.2.0.wasm");
}

#[tokio::test]
async fn missing_sidecar_refuses() {
    let store = Memory::default();
    store.insert_object("store", "demo@1.2.0.wasm", COMPONENT);

    let err = locate(Axis::Source, "demo", Some(&version()), &store, &ExecutionPaths::deployed())
        .await
        .expect_err("an unverifiable entry is refused, never resolved");

    assert!(err.to_string().contains("adapter-sidecar-missing"), "typed failure: {err}");
}

#[tokio::test]
async fn drifted_digest_refuses() {
    let store = seeded_store("sha256:recorded-elsewhere");

    let err = locate(Axis::Source, "demo", Some(&version()), &store, &ExecutionPaths::deployed())
        .await
        .expect_err("digest drift fails closed");

    assert!(err.to_string().contains("adapter-digest-mismatch"), "typed failure: {err}");
}

#[tokio::test]
async fn missing_entry_not_found() {
    let store = Memory::default();

    let err = locate(Axis::Source, "demo", Some(&version()), &store, &ExecutionPaths::deployed())
        .await
        .expect_err("no store entry, no resolve");

    assert!(err.to_string().contains("adapter-not-found"), "typed failure: {err}");
}

#[tokio::test]
async fn seeded_cache_locates_unpinned() {
    let store = Memory::default();
    store.insert_object("adapters", "demo.wasm", COMPONENT);

    let location = locate(Axis::Source, "demo", None, &store, &ExecutionPaths::deployed())
        .await
        .expect("a seeded mirror resolves without a pin");

    assert_eq!(location.object(), "demo.wasm");
}
