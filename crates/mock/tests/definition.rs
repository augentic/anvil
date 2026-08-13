//! Fixture-builder self-checks: degenerate intent form and multi-target.

use mock::definition::{Spec, mint};
use project::definition::{INTENT, resolve};

#[test]
fn degenerate_intent_value() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let minted = mint(tmp.path(), &Spec::degenerate("operator brief")).expect("mint");
    let reviewed = resolve(tmp.path(), &minted.wave).expect("resolve");
    assert_eq!(reviewed.digest, minted.digest);
    let scope = &reviewed.handoff.wave.evidence_scopes[0];
    assert_eq!(scope.source, INTENT);
    assert_eq!(scope.value.as_deref(), Some("operator brief"));
    assert!(scope.source_cid.is_none(), "intent carries value, not a CID");
    assert_eq!(reviewed.handoff.wave.targets.len(), 1);
}

#[test]
fn multi_target_home() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let minted = mint(tmp.path(), &Spec::multi_target()).expect("mint");
    let reviewed = resolve(tmp.path(), "extract-orders").expect("resolve");
    assert_eq!(reviewed.digest, minted.digest);
    assert_eq!(reviewed.handoff.wave.targets.len(), 2);
    assert_eq!(reviewed.handoff.wave.evidence_scopes.len(), 2);
    assert!(
        reviewed.handoff.wave.evidence_scopes.iter().all(|scope| {
            scope.source_cid.is_some() && scope.value.is_none() && scope.source != INTENT
        }),
        "location-backed scopes carry source-cid only"
    );
}
