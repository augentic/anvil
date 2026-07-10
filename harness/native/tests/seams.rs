//! Seam-level coverage of [`NativeProvider`]: the in-process dispatch
//! table reaches the real adapter operations (scripted through
//! `testkit::MockModel`), the DTO mappings match the guest shim's WIT
//! projections (claim JSON keys, report widening), and the describe
//! runner answers both axes.

use serde_json::json;
use specify_dev::provider::{NativeProvider, describe};
use tempfile::TempDir;
use testkit::MockModel;
use workflow::adapter::Axis;
use workflow::adapter::describe::DescribeRequest;
use workflow::seam::{Error, Input, Lead, SourceSeam as _, TargetSeam as _, WorkingTree};
use workflow::slice::BuildStatus;

fn lead(id: &str) -> Lead {
    Lead {
        lead: id.to_string(),
        synopsis: format!("Operator intent for {id}."),
        topics: Vec::new(),
    }
}

fn tree() -> WorkingTree {
    WorkingTree {
        base: "live".to_string(),
        subpath: None,
    }
}

#[tokio::test]
async fn survey_dispatches_to_intent() {
    let tmp = TempDir::new().expect("tempdir");
    let model = MockModel::answering([
        r#"{"leads":[{"lead":"password-reset","synopsis":"Let users reset passwords."}]}"#,
    ]);
    let provider = NativeProvider::new(tmp.path(), model);

    let leads = provider.survey("source:intent".to_string()).await.expect("survey");

    assert_eq!(leads.len(), 1);
    assert_eq!(leads[0].lead, "password-reset");
}

#[tokio::test]
async fn extract_projects_claim_json() {
    let tmp = TempDir::new().expect("tempdir");
    let model = MockModel::answering([
        r#"{"authority":"intent","claims":[{"kind":"intent","id":"password-reset","statement":"Let users reset passwords."}]}"#,
    ]);
    let provider = NativeProvider::new(tmp.path(), model);

    let evidence = provider
        .extract("source:intent".to_string(), lead("password-reset"))
        .await
        .expect("extract");

    assert_eq!(evidence.authority, artifacts::evidence::AuthorityClass::Intent);
    // The claim crosses through the compact seam record, exactly like
    // the WIT path: modeled keys survive, open per-kind fields do not.
    assert_eq!(evidence.claims, vec![json!({ "kind": "intent", "id": "password-reset" })]);
}

#[tokio::test]
async fn mcp_base_grants_reference_url() {
    let tmp = TempDir::new().expect("tempdir");
    let model = MockModel::answering([r#"{"leads":[]}"#]);
    let provider =
        NativeProvider::new(tmp.path(), model).mcp_base("http://127.0.0.1:7737".to_string());

    provider.survey("source:intent".to_string()).await.expect("survey");

    let requests = provider.model().requests();
    let grants = testkit::mcp_grants(&requests[0]);
    assert_eq!(grants.len(), 1, "one references grant per judgment leg");
    assert_eq!(grants[0].name, "intent-references");
    assert_eq!(grants[0].url, "http://127.0.0.1:7737/mcp/intent");
}

#[tokio::test]
async fn guidance_serves_embedded_prompts() {
    let tmp = TempDir::new().expect("tempdir");
    let provider = NativeProvider::new(tmp.path(), MockModel::answering([]));

    let omnia = provider.guidance("target:omnia".to_string()).await.expect("omnia guidance");
    assert!(omnia.starts_with("# Omnia target — guidance prompt"), "{omnia:.60}");

    let contracts =
        provider.guidance("target:contracts".to_string()).await.expect("contracts guidance");
    assert!(contracts.starts_with("# contracts.guidance"), "{contracts:.60}");
}

#[tokio::test]
async fn build_widens_report() {
    let tmp = TempDir::new().expect("tempdir");
    let model = MockModel::answering([
        r#"{"applicable":true,"summary":"generation complete"}"#,
        r#"{"applicable":true,"summary":"review complete"}"#,
        r#"{"applicable":false,"summary":"no captures binding"}"#,
        r#"{"status":"success","findings":[]}"#,
    ]);
    let provider = NativeProvider::new(tmp.path(), model);

    let report = provider
        .build(
            "target:omnia".to_string(),
            "demo".to_string(),
            vec![Input::Proposal("PROPOSAL-BODY".to_string())],
            tree(),
        )
        .await
        .expect("build");

    assert_eq!(report.status, BuildStatus::Success);
    assert_eq!(report.slice, "demo");
    assert_eq!(report.target, "omnia", "axis prefix stripped in the envelope");
    assert!(report.findings.is_empty());
}

#[tokio::test]
async fn unlinked_adapter_refused() {
    let tmp = TempDir::new().expect("tempdir");
    let provider = NativeProvider::new(tmp.path(), MockModel::answering([]));

    let err = provider.survey("source:unknown".to_string()).await.expect_err("unlinked source");
    assert!(matches!(err, Error::InvalidRequest(detail) if detail.contains("source:unknown")));

    let err = provider.guidance("target:unknown".to_string()).await.expect_err("unlinked target");
    assert!(matches!(err, Error::InvalidRequest(_)));
}

#[test]
fn describe_answers_both_axes() {
    let component = std::path::Path::new("unused.wasm");
    let source = describe(&DescribeRequest {
        component,
        axis: Axis::Source,
        adapter_id: "source:intent",
    })
    .expect("intent describes");
    assert_eq!(source.specify_floor, None);
    assert!(source.inputs.is_empty());

    let target = describe(&DescribeRequest {
        component,
        axis: Axis::Target,
        adapter_id: "target:omnia",
    })
    .expect("omnia describes");
    assert!(target.platforms.is_none());

    let err = describe(&DescribeRequest {
        component,
        axis: Axis::Source,
        adapter_id: "source:unknown",
    })
    .expect_err("unlinked adapter refuses");
    assert!(err.to_string().contains("source:unknown"), "{err}");
}
