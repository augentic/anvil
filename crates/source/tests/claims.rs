//! Claim-id gate tests.

use emery_source::claims::{claim_id_findings, validate_evidence};
use emery_source::types::{Error, Evidence};

fn evidence(json: &str) -> Evidence {
    serde_json::from_str(json).expect("evidence parses")
}

#[test]
fn clean_evidence_passes() {
    let clean = evidence(
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement","id":"password-reset.request"},
            {"kind":"decision"}
        ]}"#,
    );
    assert!(claim_id_findings(&clean.claims).is_empty());
    validate_evidence(&clean).expect("clean evidence passes the gate");
}

#[test]
fn malformed_ids_fail_closed() {
    let malformed = evidence(
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement"},
            {"kind":"criterion","id":"Not.Valid"},
            {"kind":"section"}
        ]}"#,
    );
    assert_eq!(claim_id_findings(&malformed.claims).len(), 2, "optional-id kinds pass unset");
    let Err(Error::Internal(detail)) = validate_evidence(&malformed) else {
        panic!("malformed evidence must fail the gate");
    };
    assert!(detail.contains("claims require an id"), "finding names the missing id: {detail}");
    assert!(detail.contains("`Not.Valid`"), "finding names the malformed id: {detail}");
}
