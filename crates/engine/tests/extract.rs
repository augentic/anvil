//! The extract kernel at the crate's public surface (ADR-0009 §3):
//! the fail-closed required-extras gate and deterministic receipts.
//! Seam dispatch is covered over the component seam
//! (`tests/journey.rs`, ADR-0002).

use emery_artifacts::evidence::{AuthorityClass, Claim, ClaimKind};
use emery_engine::extract::{Receipt, SourceSet, validate_set};

fn requirement(id: &str, statement: Option<&str>) -> Claim {
    let mut claim = Claim::new(ClaimKind::Requirement);
    claim.id = Some(id.to_string());
    if let Some(statement) = statement {
        claim.extras.insert("statement".into(), serde_json::Value::String(statement.into()));
    }
    claim
}

fn docs_set(claims: Vec<Claim>) -> SourceSet {
    SourceSet {
        key: "mock-docs".to_string(),
        adapter: "source:mock-docs".to_string(),
        authority: AuthorityClass::Documentation,
        claims,
    }
}

#[test]
fn complete_extras_pass() {
    let set = docs_set(vec![requirement("session.timeout", Some("Sessions expire."))]);
    validate_set(&set).expect("a claim carrying its required extras passes");
}

#[test]
fn missing_extras_refused() {
    let set = docs_set(vec![requirement("greeting.behaviour", None)]);
    let err = validate_set(&set).expect_err("a requirement without `statement` is refused (A8)");
    let message = err.to_string();
    assert!(message.contains("claim-extras-missing"), "{message}");
    assert!(message.contains("mock-docs"), "names the source: {message}");
    assert!(message.contains("greeting.behaviour"), "names the claim: {message}");
    assert!(message.contains("statement"), "names the missing key: {message}");
}

#[test]
fn receipts_deterministic() {
    let set = docs_set(vec![
        requirement("login.flow", Some("Users sign in.")),
        requirement("session.timeout", Some("Sessions expire.")),
    ]);
    let receipt = Receipt::of(&set);
    assert_eq!(receipt.key, "mock-docs");
    assert_eq!(receipt.claims, 2);
    assert!(receipt.digest.starts_with("sha256:"), "{}", receipt.digest);
    assert_eq!(receipt, Receipt::of(&set), "receipts are deterministic");
}
