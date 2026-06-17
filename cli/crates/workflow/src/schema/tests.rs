use super::*;

// Pure-schema accept/reject fixtures live in the schema crate
// (`crates/schema/tests/schemas.rs`); this module keeps only the
// workflow wrapper edges (error codes and Ok paths).

/// An empty evidence directory (or missing one) passes — empty
/// extraction is a legal slice state per workflow §Extraction
/// reliability.
#[test]
fn missing_evidence_dir_is_ok() {
    let dir = tempfile::tempdir().expect("tempdir");
    validate_evidence_dir(dir.path()).expect("missing evidence dir is ok");
}

/// The multi-source `kind: request` envelope example validates.
#[test]
fn proposal_accepts_rfc_request() {
    let request = r#"
version: 1
kind: request
projects:
  - name: identity-contracts
    target: contracts@1.0.0
    description: "Versioned API contracts crate for the identity domain."
  - name: identity-service
    target: omnia@1.0.0
    description: "Omnia identity service implementing auth and password flows."
leads:
  - source: docs
    lead: identity-api
    synopsis: "Identity API contract for authentication and account access."
  - source: legacy
    lead: identity-api
    synopsis: "Legacy identity endpoints."
  - source: docs
    lead: password-reset
    synopsis: "Users can request a password reset email."
  - source: legacy
    lead: reset-password
    synopsis: "Legacy reset-password flow."
"#;
    validate_proposal_json(request).expect("RFC request example validates");
}

/// The N=1 degenerate `kind: response` envelope example validates.
#[test]
fn proposal_accepts_rfc_n1_response() {
    let response = r"
version: 1
kind: response
slices:
  - name: fix-typo
    sources:
      - { source: intent, lead: fix-typo }
";
    validate_proposal_json(response).expect("RFC N=1 response example validates");
}

/// The multi-source fan-out `kind: response` envelope example validates.
#[test]
fn proposal_accepts_rfc_fanout_response() {
    let response = r#"
version: 1
kind: response
slices:
  - name: identity-contracts
    sources:
      - { source: docs, lead: identity-api }
      - { source: legacy, lead: identity-api }
    project: identity-contracts
    rationale: "identity API surface matched by shared slug across docs + legacy"
  - name: identity-service
    sources:
      - { source: docs, lead: identity-api }
      - { source: legacy, lead: identity-api }
    project: identity-service
    depends-on: [identity-contracts]
  - name: password-reset
    sources:
      - { source: docs, lead: password-reset }
      - { source: legacy, lead: reset-password }
    project: identity-service
    rationale: "password-reset (docs) and reset-password (legacy) are the same flow by summary judgment"
"#;
    validate_proposal_json(response).expect("RFC fan-out response example validates");
}

/// A request missing the required `inputs` block is rejected.
#[test]
fn build_request_rejects_malformed() {
    let request = r#"{"version": 1, "slice": "identity-service", "project-dir": "/w"}"#;
    match validate_build_request_json(request) {
        Err(Error::Validation { code, .. }) => assert_eq!(code, "target-build-request-schema"),
        other => panic!("expected target-build-request-schema, got {other:?}"),
    }
}

/// A report with an out-of-enum `status` is rejected.
#[test]
fn build_report_rejects_malformed() {
    let report = r#"{
            "version": 1,
            "slice": "identity-service",
            "target": "omnia@1.0.0",
            "status": "partial",
            "findings": []
        }"#;
    match validate_build_report_json(report) {
        Err(Error::Validation { code, .. }) => assert_eq!(code, "target-build-report-schema"),
        other => panic!("expected target-build-report-schema, got {other:?}"),
    }
}

/// A malformed envelope (missing `kind`, which leaves it matching
/// neither `oneOf` branch) is rejected with the `proposal-schema`
/// code.
#[test]
fn proposal_rejects_malformed_envelope() {
    let malformed = r"
version: 1
slices:
  - name: orphan
    sources:
      - { source: intent, lead: orphan }
";
    match validate_proposal_json(malformed) {
        Err(Error::Validation { code, .. }) => assert_eq!(code, "proposal-schema"),
        other => panic!("expected proposal-schema validation error, got {other:?}"),
    }
}
