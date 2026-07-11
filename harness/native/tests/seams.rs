//! Seam-level coverage of the native [`Provider`]: the in-process dispatch
//! table reaches the real adapter operations (scripted through
//! `omnia_testkit::model::Harness`), the DTO mappings match the guest shim's WIT
//! projections (claim JSON keys, report widening), and the metadata
//! runner answers both axes.

use omnia_testkit::model::{Harness, Scripted, mcp_grants};
use serde_json::json;
use specify_dev::provider::{Provider, metadata};
use tempfile::TempDir;
use workflow::adapter::metadata::Request as MetadataRequest;
use workflow::adapter::{AdapterRef, Axis, Resolver};
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

fn model(answers: impl IntoIterator<Item = &'static str>) -> Harness<Scripted> {
    Harness::new(Scripted::answers(answers))
}

#[tokio::test]
async fn survey_dispatches_intent() {
    let tmp = TempDir::new().expect("tempdir");
    let model =
        model([r#"{"leads":[{"lead":"password-reset","synopsis":"Let users reset passwords."}]}"#]);
    let provider = Provider::new(tmp.path(), model);

    let leads = provider.survey("source:intent".to_string()).await.expect("survey");

    assert_eq!(leads.len(), 1);
    assert_eq!(leads[0].lead, "password-reset");
}

#[tokio::test]
async fn extract_claim_json_projection() {
    let tmp = TempDir::new().expect("tempdir");
    let model = model([
        r#"{"authority":"intent","claims":[{"kind":"intent","id":"password-reset","statement":"Let users reset passwords."}]}"#,
    ]);
    let provider = Provider::new(tmp.path(), model);

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
async fn mcp_base_reference_grant() {
    let tmp = TempDir::new().expect("tempdir");
    let model = model([r#"{"leads":[]}"#]);
    let provider = Provider::new(tmp.path(), model).mcp_base("http://127.0.0.1:7737".to_string());

    provider.survey("source:intent".to_string()).await.expect("survey");

    let requests = provider.model().requests();
    let grants = mcp_grants(&requests[0]);
    assert_eq!(grants.len(), 1, "one references grant per judgment leg");
    assert_eq!(grants[0].name, "intent-references");
    assert_eq!(grants[0].url, "http://127.0.0.1:7737/mcp/intent");
}

#[tokio::test]
async fn guidance_embedded_prompts() {
    let tmp = TempDir::new().expect("tempdir");
    let provider = Provider::new(tmp.path(), model([]));

    let omnia = provider.guidance("target:omnia".to_string()).await.expect("omnia guidance");
    assert!(omnia.starts_with("# Omnia target — guidance prompt"), "{omnia:.60}");

    let contracts =
        provider.guidance("target:contracts".to_string()).await.expect("contracts guidance");
    assert!(contracts.starts_with("# contracts.guidance"), "{contracts:.60}");
}

#[tokio::test]
async fn build_report_widened() {
    let tmp = TempDir::new().expect("tempdir");
    let model = model([
        r#"{"applicable":true,"summary":"generation complete"}"#,
        r#"{"applicable":true,"summary":"review complete"}"#,
        r#"{"applicable":false,"summary":"no captures binding"}"#,
        r#"{"status":"success","findings":[]}"#,
    ]);
    let provider = Provider::new(tmp.path(), model);

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
    let provider = Provider::new(tmp.path(), model([]));

    let err = provider.survey("source:unknown".to_string()).await.expect_err("unlinked source");
    assert!(matches!(err, Error::InvalidRequest(detail) if detail.contains("source:unknown")));

    let err = provider.guidance("target:unknown".to_string()).await.expect_err("unlinked target");
    assert!(matches!(err, Error::InvalidRequest(_)));
}

#[test]
fn metadata_both_axes() {
    let source = metadata(&MetadataRequest {
        axis: Axis::Source,
        adapter_id: "source:intent",
    })
    .expect("intent metadata");
    assert_eq!(source.specify_floor, None);
    assert!(source.inputs.is_empty());

    let target = metadata(&MetadataRequest {
        axis: Axis::Target,
        adapter_id: "target:omnia",
    })
    .expect("omnia metadata");
    assert!(target.platforms.is_none());

    let err = metadata(&MetadataRequest {
        axis: Axis::Source,
        adapter_id: "source:unknown",
    })
    .expect_err("unlinked adapter refuses");
    assert!(err.to_string().contains("source:unknown"), "{err}");
}

#[tokio::test]
async fn catalog_dispatches_every_entry() {
    // Table-driven proof over the declarative linked-adapter table:
    // every declared entry resolves on its own axis, refuses the
    // opposite axis, and dispatches its operation set — `survey` for
    // sources (scripted empty answer), `guidance` for targets.
    let tmp = TempDir::new().expect("tempdir");

    for entry in specify_dev::catalog::entries() {
        let name = entry.name();
        let id = entry.id();
        assert_eq!(
            entry.server_name(),
            format!("{name}-references"),
            "the MCP server name follows the shared convention"
        );
        assert!(!entry.docs().is_empty(), "`{name}` embeds its prose documents");

        match entry.axis() {
            Axis::Source => {
                let provider = Provider::new(tmp.path(), model([r#"{"leads":[]}"#]));
                let resolved = provider
                    .resolve_source(&AdapterRef::bare(name), tmp.path())
                    .unwrap_or_else(|err| panic!("source `{name}` resolves: {err}"));
                assert_eq!(resolved.origin.reference, format!("rust:{id}"));
                provider
                    .resolve_target(&AdapterRef::bare(name), tmp.path())
                    .expect_err("a source never resolves on the target axis");

                let leads = provider
                    .survey(id.clone())
                    .await
                    .unwrap_or_else(|err| panic!("`{id}` survey dispatches: {err:?}"));
                assert!(leads.is_empty(), "the scripted empty survey answer crosses");
            }
            Axis::Target => {
                let provider = Provider::new(tmp.path(), model([]));
                let resolved = provider
                    .resolve_target(&AdapterRef::bare(name), tmp.path())
                    .unwrap_or_else(|err| panic!("target `{name}` resolves: {err}"));
                assert_eq!(resolved.origin.reference, format!("rust:{id}"));
                provider
                    .resolve_source(&AdapterRef::bare(name), tmp.path())
                    .expect_err("a target never resolves on the source axis");

                let prompt = provider
                    .guidance(id.clone())
                    .await
                    .unwrap_or_else(|err| panic!("`{id}` guidance dispatches: {err:?}"));
                assert!(!prompt.is_empty(), "the embedded guidance prompt is served");
            }
        }
    }
}

#[test]
fn resolver_linked_catalog() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = Provider::new(tmp.path(), model([]));

    let source = provider
        .resolve_source(&AdapterRef::bare("intent"), tmp.path())
        .expect("linked source resolves");
    assert_eq!(source.origin.label, "native");
    assert_eq!(source.origin.reference, "rust:source:intent");

    let target = provider
        .resolve_target(&AdapterRef::bare("omnia"), tmp.path())
        .expect("linked target resolves");
    assert_eq!(target.origin.reference, "rust:target:omnia");
    assert!(!tmp.path().join("target/wasm32-wasip2/release/omnia.wasm").exists());

    let unknown = provider
        .resolve_target(&AdapterRef::bare("unknown"), tmp.path())
        .expect_err("unknown linked adapter refuses");
    assert_eq!(unknown.variant_str(), "adapter-not-found");

    let pinned = provider
        .resolve_target(&AdapterRef::pinned("omnia", "1.0.0".parse().expect("semver")), tmp.path())
        .expect_err("pinned identities remain component-only");
    assert_eq!(pinned.variant_str(), "adapter-not-found");
}
