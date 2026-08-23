//! Verify-on-read for pinned store entries.
//!
//! Retained at crate level (testing.md "Crate integration"): package
//! pins resolve dispatch-first while the dynamic resolver is deferred,
//! so no CLI input reaches the pinned store leg of `locate` today.
//! The fail-closed verification contract must hold for the resolver's
//! return. The unpinned cache leg is owned by the root `specify`
//! scenarios (mirroring and mirror survival).

use emery_diagnostics::cache::content_digest;
use emery_engine::handler::ExecutionPaths;
use emery_engine::resolve::Axis;
use emery_engine::resolve::resolver::locate;
use emery_testkit::Memory;

const COMPONENT: &[u8] = b"\0asm-demo";

const fn version() -> semver::Version {
    semver::Version::new(1, 2, 0)
}

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

// Every unverifiable pinned entry refuses typed, never resolves.
#[tokio::test]
async fn unverifiable_pins_refuse() {
    let missing_sidecar = Memory::default();
    missing_sidecar.insert_object("store", "demo@1.2.0.wasm", COMPONENT);
    let cases: [(Memory, &str); 3] = [
        (missing_sidecar, "adapter-sidecar-missing"),
        (seeded_store("sha256:recorded-elsewhere"), "adapter-digest-mismatch"),
        (Memory::default(), "adapter-not-found"),
    ];

    for (store, code) in cases {
        let err =
            locate(Axis::Source, "demo", Some(&version()), &store, &ExecutionPaths::deployed())
                .await
                .expect_err(code);
        assert!(err.to_string().contains(code), "typed failure: {err}");
    }
}
