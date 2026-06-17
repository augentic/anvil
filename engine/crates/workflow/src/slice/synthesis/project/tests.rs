use specify_model::spec::provenance::RequirementStatus;

use super::*;
use crate::slice::model::{ModelRequirement, ModelTask, validate_model_doc};
use crate::slice::synthesis::authority::Agreement;

fn header() -> ProjectionHeader {
    ProjectionHeader {
        version: 1,
        slice: "identity-service".to_string(),
        project: Some("identity-service".to_string()),
    }
}

fn authority(pairs: &[(&str, AuthorityClass)]) -> BTreeMap<String, AuthorityClass> {
    pairs.iter().map(|(source, class)| ((*source).to_string(), *class)).collect()
}

fn evidence(pairs: &[(&str, &str, ClaimKind)]) -> BTreeMap<(String, String), ClaimKind> {
    pairs
        .iter()
        .map(|(source, id, kind)| (((*source).to_string(), (*id).to_string()), *kind))
        .collect()
}

fn claim(source: &str, id: &str, kind: ClaimKind) -> ModelClaim {
    ModelClaim {
        source: source.to_string(),
        id: id.to_string(),
        kind,
        winner: None,
    }
}

fn requirement(
    title: &str, statement: &str, agreement: Option<Agreement>, claims: Vec<ModelClaim>,
) -> ModelRequirement {
    ModelRequirement {
        id: None,
        title: title.to_string(),
        status: None,
        agreement,
        domain: Some("password-reset".to_string()),
        sources: Vec::new(),
        claims,
        statement: statement.to_string(),
        scenarios: Vec::new(),
        notes: None,
    }
}

fn task(id: &str, text: &str, satisfies: &[&str]) -> ModelTask {
    ModelTask {
        id: id.to_string(),
        text: text.to_string(),
        depends_on: Vec::new(),
        satisfies: satisfies.iter().map(|s| (*s).to_string()).collect(),
    }
}

/// The slice-model envelope: REQ-001 is a
/// multi-claim agreement; REQ-002 is a per-kind divergence where the
/// documentation `criterion` beats the behaviour `example`.
fn rfc_response() -> SliceModel {
    SliceModel {
        version: None,
        slice: None,
        project: None,
        requirements: vec![
            requirement(
                "Request password reset",
                "The system lets a registered user request a password reset link by email.",
                Some(Agreement::Agreed),
                vec![
                    claim("docs", "password-reset.request", ClaimKind::Requirement),
                    claim("legacy", "password-reset.request", ClaimKind::Example),
                ],
            ),
            requirement(
                "Reset link expiry",
                "Reset links expire after 30 minutes.",
                Some(Agreement::Disagreed),
                vec![
                    claim("docs", "password-reset.expiry", ClaimKind::Criterion),
                    claim("legacy", "password-reset.expiry", ClaimKind::Example),
                ],
            ),
        ],
        tasks: vec![task("TASK-001", "Implement password reset request handling.", &["REQ-001"])],
    }
}

fn rfc_authority() -> BTreeMap<String, AuthorityClass> {
    authority(&[("docs", AuthorityClass::Documentation), ("legacy", AuthorityClass::Behaviour)])
}

fn rfc_evidence() -> BTreeMap<(String, String), ClaimKind> {
    evidence(&[
        ("docs", "password-reset.request", ClaimKind::Requirement),
        ("legacy", "password-reset.request", ClaimKind::Example),
        ("docs", "password-reset.expiry", ClaimKind::Criterion),
        ("legacy", "password-reset.expiry", ClaimKind::Example),
    ])
}

#[test]
fn projects_model_to_valid_output() {
    let projected =
        project(rfc_response(), header(), &rfc_authority(), &BTreeMap::new(), &rfc_evidence())
            .expect("the RFC envelope projects");

    // Header stamped from `header`.
    assert_eq!(projected.version, Some(1));
    assert_eq!(projected.slice.as_deref(), Some("identity-service"));
    assert_eq!(projected.project.as_deref(), Some("identity-service"));

    // REQ-001 — multi-claim agreement, no winners, docs before legacy.
    let req1 = &projected.requirements[0];
    assert_eq!(req1.id.as_deref(), Some("REQ-001"));
    assert_eq!(req1.status, Some(RequirementStatus::Agreed));
    assert_eq!(req1.sources, vec!["docs", "legacy"]);
    assert!(req1.claims.iter().all(|c| c.winner.is_none()));

    // REQ-002 — documentation criterion beats behaviour example.
    let req2 = &projected.requirements[1];
    assert_eq!(req2.id.as_deref(), Some("REQ-002"));
    assert_eq!(req2.status, Some(RequirementStatus::Divergence));
    assert_eq!(req2.sources, vec!["docs", "legacy"]);
    assert_eq!(req2.claims[0].winner, Some(true));
    assert_eq!(req2.claims[1].winner, Some(false));

    // The projected model round-trips through the slice-model schema.
    let value = serde_json::to_value(&projected).expect("serialise projected model");
    validate_model_doc(&value).expect("projected model is schema-valid");
}

#[test]
fn normalizes_agent_supplied_kernel_fields() {
    // The agent pre-assigns wrong kernel/header values; the kernel
    // ignores and re-derives every one of them (normalize, never
    // reject).
    let mut response = rfc_response();
    response.version = Some(99);
    response.slice = Some("bogus-slice".to_string());
    response.project = Some("bogus-project".to_string());
    response.requirements[0].id = Some("REQ-999".to_string());
    response.requirements[0].status = Some(RequirementStatus::Conflict);
    response.requirements[0].sources = vec!["wrong".to_string()];
    for claim in &mut response.requirements[0].claims {
        claim.winner = Some(true);
    }

    let projected =
        project(response, header(), &rfc_authority(), &BTreeMap::new(), &rfc_evidence())
            .expect("a normalizing projection succeeds");

    let req1 = &projected.requirements[0];
    assert_eq!(projected.version, Some(1));
    assert_eq!(projected.slice.as_deref(), Some("identity-service"));
    assert_eq!(projected.project.as_deref(), Some("identity-service"));
    assert_eq!(req1.id.as_deref(), Some("REQ-001"));
    assert_eq!(req1.status, Some(RequirementStatus::Agreed));
    assert_eq!(req1.sources, vec!["docs", "legacy"]);
    assert!(req1.claims.iter().all(|c| c.winner.is_none()));
}

#[test]
fn kernel_is_deterministic() {
    // Kernel determinism: identical inputs yield byte-identical
    // output. Target-independence holds by
    // construction — `project` takes no `target` or shape-brief input.
    let first =
        project(rfc_response(), header(), &rfc_authority(), &BTreeMap::new(), &rfc_evidence())
            .expect("first projection");
    let second =
        project(rfc_response(), header(), &rfc_authority(), &BTreeMap::new(), &rfc_evidence())
            .expect("second projection");

    let first_json = serde_json::to_string(&first).expect("serialise first");
    let second_json = serde_json::to_string(&second).expect("serialise second");
    assert_eq!(first_json, second_json);
}

#[test]
fn aborts_on_source_orphan() {
    let mut response = rfc_response();
    response.requirements[0].claims.push(claim("ghost", "no.such.claim", ClaimKind::Excerpt));

    let err = project(response, header(), &rfc_authority(), &BTreeMap::new(), &rfc_evidence())
        .expect_err("an unanchored claim aborts");
    match err {
        Error::Validation { code, .. } => assert_eq!(code, "slice-model-source-orphan"),
        other => panic!("expected slice-model-source-orphan, got {other:?}"),
    }
}

#[test]
fn aborts_on_claim_kind_mismatch() {
    // Evidence records `password-reset.expiry` as a `criterion`; the
    // claim asserts `requirement`.
    let response = SliceModel {
        requirements: vec![requirement(
            "Reset link expiry",
            "Reset links expire after 30 minutes.",
            None,
            vec![claim("docs", "password-reset.expiry", ClaimKind::Requirement)],
        )],
        ..rfc_response()
    };

    let err = project(response, header(), &rfc_authority(), &BTreeMap::new(), &rfc_evidence())
        .expect_err("a kind mismatch aborts");
    match err {
        Error::Validation { code, .. } => assert_eq!(code, "slice-model-claim-kind-mismatch"),
        other => panic!("expected slice-model-claim-kind-mismatch, got {other:?}"),
    }
}

#[test]
fn aborts_on_cross_ref_orphan() {
    let mut response = rfc_response();
    // Only REQ-001 / REQ-002 are projected; satisfy a missing REQ-003.
    response.tasks = vec![task("TASK-001", "Dangling satisfy.", &["REQ-003"])];

    let err = project(response, header(), &rfc_authority(), &BTreeMap::new(), &rfc_evidence())
        .expect_err("a dangling satisfies ref aborts");
    match err {
        Error::Validation { code, .. } => assert_eq!(code, "slice-model-cross-ref-orphan"),
        other => panic!("expected slice-model-cross-ref-orphan, got {other:?}"),
    }
}

#[test]
fn aborts_on_id_grammar() {
    let mut response = rfc_response();
    // Agent-authored task id outside the closed grammar; its
    // (empty) satisfies list keeps the cross-ref check clean so the
    // grammar gate is what fires.
    response.tasks = vec![task("TASK-1", "Malformed task id.", &[])];

    let err = project(response, header(), &rfc_authority(), &BTreeMap::new(), &rfc_evidence())
        .expect_err("a malformed task id aborts");
    match err {
        Error::Validation { code, .. } => assert_eq!(code, "slice-model-id-grammar"),
        other => panic!("expected slice-model-id-grammar, got {other:?}"),
    }
}
