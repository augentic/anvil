//! The generation store's CAS concurrency invariant.
//!
//! Retained at crate level (testing.md "Crate integration"): two runs
//! racing one pointer cannot be arranged through the CLI entry point,
//! which observes and commits inside a single `specify`. Everything
//! else the store does — commit, read, pruning, corruption, re-mine
//! diffs — is owned by the root `specify` scenarios.

use emery_engine::store::{SpecSet, Store};
use omnia_test::guest::Memory;

#[tokio::test]
async fn concurrent_commit_conflicts() {
    let memory = Memory::default();
    let store = Store::new(&memory);

    // Both runs observe the empty store; the winner swaps first.
    let stale = store.observe().await;
    let observed = store.observe().await;
    let winner = store.commit(&set("# Spec winner\n"), &observed).await.expect("commit");

    let err = store
        .commit(&set("# Spec loser\n"), &stale)
        .await
        .expect_err("a stale observation must never last-write-wins over the swapped pointer");
    assert_eq!(err.code(), "server_error", "typed failure");
    assert!(
        err.description().contains("concurrent `emery specify`"),
        "typed failure: {}",
        err.description()
    );
    assert_eq!(
        store.current().await.expect("current").expect("committed").id,
        winner.id,
        "the pointer still names the winner"
    );
    let spec =
        memory.object("spec", &format!("generations/{}/spec.md", winner.id)).expect("winning spec");
    assert_eq!(spec, b"# Spec winner\n", "the winning generation is intact");
}

fn set(spec: &str) -> SpecSet {
    SpecSet {
        spec: spec.to_string(),
        design: "# Design\n".to_string(),
    }
}
