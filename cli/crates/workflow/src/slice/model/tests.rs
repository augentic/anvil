use std::collections::BTreeMap;
use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::journal::test_timestamp;

/// A fully-projected `model.yaml` (kernel-owned fields present) on
/// the earned core (`requirements` + `tasks`), so it validates
/// against the trimmed `model.schema.json`. REQ-001 is a multi-claim
/// agreement; REQ-002 is an authority-resolved divergence.
const PROJECTED_MODEL: &str = "version: 1
slice: identity-service
project: identity-service
requirements:
  - id: REQ-001
    title: Request password reset
    status: agreed
    domain: password-reset
    agreement: agreed
    sources: [docs, legacy]
    claims:
      - source: docs
        id: password-reset.request
        kind: requirement
      - source: legacy
        id: password-reset.request
        kind: example
    statement: The system lets a user request a reset link.
  - id: REQ-002
    title: Reset link expiry
    status: divergence
    domain: password-reset
    agreement: disagreed
    sources: [docs, legacy]
    claims:
      - source: docs
        id: password-reset.expiry
        kind: criterion
        winner: true
      - source: legacy
        id: password-reset.expiry
        kind: example
        winner: false
    statement: Reset links expire after 30 minutes.
tasks:
  - id: TASK-001
    text: Implement password reset request handling.
    satisfies: [REQ-001]
";

/// `evidence/docs.yaml` — documentation-authority claims the
/// projection reads `value` / `path` from.
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

/// `evidence/legacy.yaml` — behaviour-authority claims (loses the
/// `password-reset.expiry` divergence to the documentation claim).
const EVIDENCE_LEGACY: &str = "authority: behaviour
lead: password-reset
claims:
  - id: password-reset.request
    kind: example
    output: \"POST /password-reset returns 202.\"
    path: src/users/reset.ts#L42
  - id: password-reset.expiry
    kind: example
    output: \"expiresAt = createdAt + 24h\"
    path: src/users/reset.ts#L88
";

/// Stage a slice dir with the two Evidence documents so
/// [`SliceModel::to_provenance_index`] can read `value` / `path` and
/// re-resolve authority.
fn stage_slice_dir() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    let evidence = dir.path().join("evidence");
    std::fs::create_dir_all(&evidence).expect("mkdir evidence");
    std::fs::write(evidence.join("docs.yaml"), EVIDENCE_DOCS).expect("write docs.yaml");
    std::fs::write(evidence.join("legacy.yaml"), EVIDENCE_LEGACY).expect("write legacy.yaml");
    dir
}

fn project(model: &SliceModel, slice_dir: &Path) -> Result<ProvenanceIndex> {
    model.to_provenance_index(
        slice_dir,
        &BTreeMap::new(),
        test_timestamp("2026-05-28T05:45:00Z"),
        "specify@2.1.0".to_string(),
    )
}

#[test]
fn parses_and_validates_projected_model() {
    let model = SliceModel::parse_yaml(PROJECTED_MODEL).expect("projected model must validate");
    assert_eq!(model.slice.as_deref(), Some("identity-service"));
    assert_eq!(model.requirements.len(), 2);
    assert_eq!(model.tasks.len(), 1);
    assert_eq!(model.requirements[0].title, "Request password reset");
}

#[test]
fn projects_single_value_agreement() {
    let dir = stage_slice_dir();
    let model = SliceModel::parse_yaml(PROJECTED_MODEL).expect("parse");
    let index = project(&model, dir.path()).expect("projection succeeds");
    assert_eq!(index.slice, "identity-service");
    assert_eq!(index.requirements.len(), 2);

    let req = &index.requirements[0];
    assert_eq!(req.id, "REQ-001");
    // Two agreeing claims → recomputed `single-value-agreement`.
    assert_eq!(req.resolution, ProvenanceResolution::SingleValueAgreement);
    assert!(req.resolution_trace.is_none());
    assert_eq!(req.contributing_claims.len(), 2);
    // `value` / `path` are read from Evidence, not the model.
    let docs_claim = &req.contributing_claims[0];
    assert_eq!(docs_claim.value.as_deref(), Some("The system lets a user request a reset link."));
    assert_eq!(docs_claim.path.as_deref(), Some("docs/identity/reset.md#L4"));
    index.validate().expect("projected index must validate");
}

#[test]
fn projects_authority_resolved_divergence() {
    let dir = stage_slice_dir();
    let model = SliceModel::parse_yaml(PROJECTED_MODEL).expect("parse");
    let index = project(&model, dir.path()).expect("projection succeeds");

    let req = &index.requirements[1];
    assert_eq!(req.id, "REQ-002");
    // documentation `criterion` beats behaviour `example` →
    // recomputed `authority-resolved`, with a default-ordering trace.
    assert_eq!(req.resolution, ProvenanceResolution::AuthorityResolved);
    let trace = req.resolution_trace.as_ref().expect("divergence carries a trace");
    assert_eq!(trace.step, "default-authority-ordering");
    assert_eq!(trace.winner.as_deref(), Some("docs"));
    // The losing claim still reads its Evidence `value`.
    let legacy_claim = &req.contributing_claims[1];
    assert_eq!(legacy_claim.value.as_deref(), Some("expiresAt = createdAt + 24h"));
    index.validate().expect("projected index must validate");
}

#[test]
fn projection_rejects_pre_projection_draft() {
    let dir = stage_slice_dir();
    let mut model = SliceModel::parse_yaml(PROJECTED_MODEL).expect("parse");
    model.requirements[0].id = None;
    let err =
        project(&model, dir.path()).expect_err("a draft without projected ids cannot project");
    assert!(matches!(err, Error::Validation { .. }));
}

#[test]
fn rejects_missing_required_sections() {
    let err = SliceModel::parse_yaml("version: 1\nslice: x\nrequirements: []\n")
        .expect_err("a document missing the required `tasks` section must fail the schema");
    assert!(matches!(err, Error::Validation { .. }));
}
