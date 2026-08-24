//! The output home's CAS concurrency invariant.
//!
//! Retained at crate level (testing.md "Crate integration"): two runs
//! racing one pointer cannot be arranged through the CLI entry point,
//! which observes and commits inside a single `specify`. Everything
//! else the home does — commit, read, pruning, corruption, re-mine
//! diffs — is owned by the root `specify` scenarios.

use emery_engine::home::{Home, SpecSet};
use emery_testkit::Memory;

fn set(spec: &str) -> SpecSet {
    SpecSet {
        spec: spec.to_string(),
        design: "# Design\n".to_string(),
    }
}

#[tokio::test]
async fn concurrent_commit_conflicts() {
    let store = Memory::default();
    let home = Home::new(&store);

    // Both runs observe the empty home; the winner swaps first.
    let stale = home.observe().await;
    let observed = home.observe().await;
    let winner = home.commit(&set("# Spec winner\n"), &observed).await.expect("commit");

    let err = home
        .commit(&set("# Spec loser\n"), &stale)
        .await
        .expect_err("a stale observation must never last-write-wins over the swapped pointer");
    assert_eq!(err.code(), "spec-pointer-conflict", "typed failure");
    assert!(
        err.description().contains("concurrent `emery specify`"),
        "typed failure: {}",
        err.description()
    );
    assert_eq!(
        home.current().await.expect("current").expect("committed").id,
        winner.id,
        "the pointer still names the winner"
    );
    let spec =
        store.object("spec", &format!("generations/{}/spec.md", winner.id)).expect("winning spec");
    assert_eq!(spec, b"# Spec winner\n", "the winning generation is intact");
}
