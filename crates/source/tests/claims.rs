//! Claim gate tests: id grammar and required per-kind extras.

use emery_source::claims::{extras_findings, id_findings};
use emery_source::types::{ClaimKind, Error, Evidence};

#[test]
fn clean_evidence_passes() {
    let clean = evidence(
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement","id":"password-reset.request","statement":"Users reset by email."},
            {"kind":"criterion","id":"password-reset.expiry","criterion":"Links expire in 30m."},
            {"kind":"example","id":"password-reset.stale","replay-digest":"sha256:00"},
            {"kind":"decision"}
        ]}"#,
    );
    assert!(id_findings(&clean.claims).is_empty());
    assert!(extras_findings(&clean.claims).is_empty());
    clean.validate().expect("clean evidence passes the gate");
}

#[test]
fn malformed_ids_fail_closed() {
    let malformed = evidence(
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement","statement":"Unnamed."},
            {"kind":"criterion","id":"Not.Valid","criterion":"Misnamed."},
            {"kind":"section"}
        ]}"#,
    );
    assert_eq!(id_findings(&malformed.claims).len(), 2, "optional-id kinds pass unset");
    assert!(extras_findings(&malformed.claims).is_empty(), "extras are present");
    let Err(Error::Internal(detail)) = malformed.validate() else {
        panic!("malformed evidence must fail the gate");
    };
    assert!(detail.contains("claims require an id"), "finding names the missing id: {detail}");
    assert!(detail.contains("`Not.Valid`"), "finding names the malformed id: {detail}");
}

// The closed table is the single A8 rule both gates consume.
#[test]
fn missing_extras_fail_closed() {
    assert_eq!(ClaimKind::Requirement.required_extras(), ["statement"]);
    assert_eq!(ClaimKind::Criterion.required_extras(), ["criterion"]);
    assert_eq!(ClaimKind::Example.required_extras(), ["replay-digest"]);
    assert!(ClaimKind::Decision.required_extras().is_empty());

    let bare = evidence(
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement","id":"password-reset.request"},
            {"kind":"example","id":"password-reset.stale","input":{}},
            {"kind":"section","synopsis":"no extras required"}
        ]}"#,
    );
    assert!(id_findings(&bare.claims).is_empty(), "ids are well-formed");
    let findings = extras_findings(&bare.claims);
    assert_eq!(findings.len(), 2, "one finding per absent extra: {findings:?}");
    assert!(findings[0].contains("`password-reset.request` is missing its required `statement`"));
    assert!(findings[1].contains("`password-reset.stale` is missing its required `replay-digest`"));
    let Err(Error::Internal(detail)) = bare.validate() else {
        panic!("absent extras must fail the gate");
    };
    assert!(detail.contains("missing its required `statement`"), "{detail}");
}

fn evidence(json: &str) -> Evidence {
    serde_json::from_str(json).expect("evidence parses")
}
