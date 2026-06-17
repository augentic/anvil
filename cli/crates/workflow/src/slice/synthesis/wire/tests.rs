use serde_json::json;

use super::*;
use crate::registry::Surface;
use crate::slice::synthesis::authority::Agreement;

/// The synthesis-response worked example — the agent's
/// `kind: response` with kernel-owned and header model fields
/// omitted.
const RFC_RESPONSE: &str = "\
kind: response
version: 1
slice: identity-service
model:
  requirements:
    - title: Request password reset
      domain: password-reset
      agreement: agreed
      claims:
        - { source: docs,   id: password-reset.request,       kind: requirement }
        - { source: legacy, id: users.password-reset.request, kind: example }
      statement: The system lets a registered user request a password reset link by email.
      scenarios:
        - Given a registered email, when the user requests a reset, then the system accepts it.
  tasks:
    - id: TASK-001
      text: Implement password reset request handling.
      satisfies: [REQ-001]
artifacts:
  proposal: \"# Password reset\\n…\"
  design: \"# Design\\n…\"
  tasks: \"# Tasks\\n- [ ] TASK-001 …\"
  specs:
    - domain: password-reset
      content: \"## Request password reset\\nThe system lets a registered user…\"
";

#[test]
fn response_round_trips_rfc_example() {
    let response: SynthesisResponse =
        serde_saphyr::from_str(RFC_RESPONSE).expect("response deserialises");

    assert_eq!(response.version, SYNTHESIS_VERSION);
    assert_eq!(response.kind, SynthesisKind::Response);
    assert_eq!(response.slice, "identity-service");

    // The kernel-omitted header/per-requirement fields deserialise
    // cleanly as `None` against the optional `SliceModel` shape.
    assert!(response.model.version.is_none());
    assert!(response.model.slice.is_none());
    assert_eq!(response.model.requirements.len(), 1);
    let req = &response.model.requirements[0];
    assert_eq!(req.title, "Request password reset");
    assert_eq!(req.domain.as_deref(), Some("password-reset"));
    assert_eq!(req.agreement, Some(Agreement::Agreed));
    assert!(req.id.is_none());
    assert!(req.status.is_none());
    assert_eq!(req.claims.len(), 2);
    assert_eq!(req.claims[0].source, "docs");
    assert!(req.claims[0].winner.is_none());
    assert_eq!(response.model.tasks.len(), 1);
    assert_eq!(response.model.tasks[0].id, "TASK-001");

    assert_eq!(response.artifacts.specs.len(), 1);
    assert_eq!(response.artifacts.specs[0].domain, "password-reset");
    assert!(response.artifacts.proposal.starts_with("# Password reset"));

    // Re-serialise into JSON and back; the shape is stable.
    let json = serde_json::to_string(&response).expect("serialise response");
    let reparsed: SynthesisResponse = serde_json::from_str(&json).expect("re-deserialise response");
    assert_eq!(response, reparsed);
}

#[test]
fn response_rejects_unknown_field() {
    let bogus = format!("{RFC_RESPONSE}stray-field: true\n");
    serde_saphyr::from_str::<SynthesisResponse>(&bogus)
        .expect_err("deny_unknown_fields rejects stray top-level keys");
}

/// `evidence/docs.yaml` fixture carrying a document-level
/// `authority` the inputs builder must drop.
const EVIDENCE_DOCS: &str = "authority: documentation
lead: password-reset
claims:
  - id: password-reset.request
    kind: requirement
    statement: The system lets a user request a reset link.
    path: docs/identity/reset.md#L4
  - id: password-reset.expiry
    kind: criterion
    criterion: Reset links expire after 30 minutes.
    path: docs/identity/reset.md#L7
";

#[test]
fn build_inputs_carries_lead_and_claims() {
    let docs = SynthesisSourceInput::from_evidence_yaml("docs", EVIDENCE_DOCS)
        .expect("docs evidence shapes");
    let legacy = SynthesisSourceInput {
        source: "legacy".to_string(),
        lead: "password-reset".to_string(),
        claims: vec![json!({
            "id": "password-reset.expiry",
            "kind": "example",
            "output": "expiresAt = createdAt + 24h",
            "path": "src/users/reset.ts#L88",
        })],
    };

    let baseline = vec![Surface {
        domain: "identity".to_string(),
        requirements: vec!["User registration".to_string()],
        more: None,
    }];
    let inputs = build_synthesis_inputs(
        "identity-service",
        &[docs, legacy],
        "# Shape brief\nbody",
        &baseline,
    );

    assert_eq!(inputs.version, SYNTHESIS_VERSION);
    assert_eq!(inputs.kind, SynthesisInputsKind::Inputs);
    assert_eq!(inputs.slice, "identity-service");
    assert_eq!(inputs.shape_brief, "# Shape brief\nbody");
    assert_eq!(inputs.baseline, baseline);

    assert_eq!(inputs.sources.len(), 2);
    let docs = &inputs.sources[0];
    assert_eq!(docs.source, "docs");
    assert_eq!(docs.lead, "password-reset");
    assert_eq!(docs.claims.len(), 2);
    // Body fields pass through verbatim.
    assert_eq!(docs.claims[0]["id"], json!("password-reset.request"));
    assert_eq!(docs.claims[1]["criterion"], json!("Reset links expire after 30 minutes."));

    // Authority is resolved post-response by the kernel; it must
    // never reach the agent step.
    let serialised = serde_json::to_string(&inputs).expect("serialise inputs");
    assert!(!serialised.contains("authority"), "authority must be absent: {serialised}");
    assert!(serialised.contains("shape-brief"), "shape-brief renders kebab-case");
}

#[test]
fn from_evidence_file_reads_and_shapes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("docs.yaml");
    std::fs::write(&path, EVIDENCE_DOCS).expect("write evidence");

    let shaped = SynthesisSourceInput::from_evidence_file("docs", &path).expect("file shapes");
    assert_eq!(shaped.source, "docs");
    assert_eq!(shaped.lead, "password-reset");
    assert_eq!(shaped.claims.len(), 2);
}
