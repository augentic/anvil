//! Output-home integration tests.

#[path = "support/storage.rs"]
mod storage;

use emery_engine::home::{Diff, Home, SpecSet};
use storage::Memory;

fn set(spec: &str) -> SpecSet {
    SpecSet {
        spec: spec.to_string(),
        design: "# Design\n".to_string(),
    }
}

fn generation_object(id: &str, name: &str) -> String {
    format!("generations/{id}/{name}")
}

async fn commit(home: &Home<'_, Memory>, spec: &SpecSet) -> emery_engine::home::Committed {
    let observed = home.observe().await;
    home.commit(spec, &observed).await.expect("commit")
}

#[tokio::test]
async fn commit_behind_pointer() {
    let store = Memory::default();
    let home = Home::new(&store);

    let committed = commit(&home, &set("# Spec\n")).await;

    let spec = store.object("spec", &generation_object(&committed.id, "spec.md")).expect("spec");
    assert_eq!(spec, b"# Spec\n");
    let design =
        store.object("spec", &generation_object(&committed.id, "design.md")).expect("design");
    assert_eq!(design, b"# Design\n");
    let pointer = store.state("spec/current").expect("pointer");
    assert_eq!(
        String::from_utf8_lossy(&pointer).trim(),
        committed.id,
        "the pointer names the committed generation"
    );
    let current = home.current().await.expect("current").expect("committed");
    assert_eq!(current, committed);
}

#[tokio::test]
async fn rerun_is_byte_stable() {
    let store = Memory::default();
    let home = Home::new(&store);

    commit(&home, &set("# Spec\n")).await;
    let before = store.snapshot();
    commit(&home, &set("# Spec\n")).await;

    assert_eq!(before, store.snapshot(), "an identical re-run must be byte-stable");
}

#[tokio::test]
async fn swap_prunes_superseded() {
    let store = Memory::default();
    let home = Home::new(&store);

    let first = commit(&home, &set("# Spec v1\n")).await;
    let second = commit(&home, &set("# Spec v2\n")).await;

    assert_ne!(first.id, second.id);
    assert!(
        store.object("spec", &generation_object(&first.id, "spec.md")).is_none(),
        "the superseded generation is pruned"
    );
    assert_eq!(home.current().await.expect("current").expect("committed").id, second.id);
}

#[tokio::test]
async fn concurrent_commit_conflicts() {
    let store = Memory::default();
    let home = Home::new(&store);

    // Both runs observe the empty home; the winner swaps first.
    let stale = home.observe().await;
    let winner = commit(&home, &set("# Spec winner\n")).await;

    let err = home
        .commit(&set("# Spec loser\n"), &stale)
        .await
        .expect_err("a stale observation must never last-write-wins over the swapped pointer");
    assert!(err.to_string().contains("spec-pointer-conflict"), "typed failure: {err}");
    assert_eq!(
        home.current().await.expect("current").expect("committed").id,
        winner.id,
        "the pointer still names the winner"
    );
    let spec = store.object("spec", &generation_object(&winner.id, "spec.md")).expect("spec");
    assert_eq!(spec, b"# Spec winner\n", "the winning generation is intact");
}

fn block(id: u32, name: &str, body: &str) -> String {
    format!(
        "### Requirement: {name}\n\nID: REQ-{id:03}\nSources: [mock-docs]\nStatus: agreed\n\n{body}\n\n"
    )
}

#[tokio::test]
async fn observe_reads_current() {
    let store = Memory::default();
    let home = Home::new(&store);
    assert!(home.observe().await.into_outgoing().is_none(), "no generation, no outgoing set");

    let committed = commit(&home, &set("# Spec\n")).await;
    let (from, outgoing) = home.observe().await.into_outgoing().expect("outgoing after commit");
    assert_eq!(from, committed.id, "the outgoing id is the pointer's");
    assert_eq!(outgoing, set("# Spec\n"), "the outgoing set reads back verbatim");
}

// Positional requirement ids must not affect section identity.
#[test]
fn remine_diff_sections() {
    let old_spec = format!(
        "# Specification\n\n{}{}{}",
        block(1, "login.flow", "Users sign in with email and password."),
        block(2, "legacy.export", "Exports ship nightly."),
        block(3, "session.timeout", "Sessions expire after 30 minutes of inactivity."),
    );
    let new_spec = format!(
        "# Specification\n\n{}{}{}",
        block(1, "access.audit", "Access is audited."),
        block(2, "login.flow", "Users sign in with email and password."),
        block(3, "session.timeout", "Sessions expire after 45 minutes of inactivity."),
    );
    let outgoing = SpecSet {
        design: "# Design v1\n".to_string(),
        ..set(&old_spec)
    };
    let incoming = SpecSet {
        design: "# Design v2\n".to_string(),
        ..set(&new_spec)
    };

    let diff = Diff::between("cafe".to_string(), &outgoing, &incoming);

    assert!(!diff.is_empty());
    assert_eq!(diff.from, "cafe");
    assert_eq!(diff.artifacts, ["spec.md", "design.md"], "changed artifacts in set order");
    assert_eq!(diff.added, ["access.audit"]);
    assert_eq!(diff.removed, ["legacy.export"]);
    assert_eq!(
        diff.changed,
        ["session.timeout"],
        "`login.flow` shifted from REQ-001 to REQ-002 unchanged — positional ids never count"
    );
}

#[test]
fn remine_diff_empty() {
    let identical =
        set(&format!("# Specification\n\n{}", block(1, "login.flow", "Users sign in.")));
    let diff = Diff::between("cafe".to_string(), &identical, &identical);
    assert!(diff.is_empty(), "a byte-stable re-run is an explicit empty diff: {diff:?}");
}

#[tokio::test]
async fn dangling_pointer_fails() {
    let store = Memory::default();
    let home = Home::new(&store);
    store.insert_state("spec/current", b"0123456789abcdef\n");

    let err =
        home.current().await.expect_err("a dangling pointer is corruption, not an empty result");
    assert!(err.to_string().contains("spec-home-corrupt"), "typed failure: {err}");
}
