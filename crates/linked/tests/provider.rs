//! Provider gates over the shared probe implementors: linked ensure
//! (bare and exact-pin matching, mismatched-pin and component-selector
//! refusal, runtime floor enforcement) and the operation legs crossing
//! the workflow seam.
//!
//! Linked coverage only: these tests claim nothing about component
//! ABI, WIT mapping, instance isolation, digests, or the adapter
//! store — those gates stay with the component deployment.

mod support;

use linked::{Catalog, DynModel, Provider, ReferenceMode};
use omnia_testkit::model::Scripted;
use project::adapter::{AdapterSelector, Resolver as _};
use project::handler::ExecutionPaths;
use project::seam::{self, Source as _, Target as _};
use support::{FailGuidance, Floored, Pinned, Probe};

fn provider(root: &std::path::Path, answers: &[&str]) -> Provider {
    let catalog = Catalog::builder()
        .source::<Probe>()
        .target::<Probe>()
        .target::<FailGuidance>()
        .target::<Floored>()
        .target::<Pinned>()
        .build()
        .expect("valid catalog");
    Provider::new(
        ExecutionPaths::operator(root),
        DynModel::new(Scripted::answers(answers.iter().copied())),
        catalog,
        ReferenceMode::Offline,
    )
}

fn bare(name: &str) -> AdapterSelector {
    AdapterSelector::parse(name).expect("bare selector")
}

// Bare selectors resolve by name and report the entry's actual
// compiled version, not a placeholder.
#[test]
fn bare_resolution() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);
    let paths = ExecutionPaths::operator(tmp.path());

    let source = provider.resolve_source(&bare("mock"), &paths).expect("linked source resolves");
    assert_eq!(source.manifest.version.to_string(), "0.0.0");
    assert_eq!(source.origin.label, "linked");
    assert_eq!(source.origin.reference, "rust:source:mock");

    let target = provider.resolve_target(&bare("mock"), &paths).expect("linked target resolves");
    assert_eq!(target.origin.reference, "rust:target:mock");

    let unknown =
        provider.resolve_target(&bare("unknown"), &paths).expect_err("unlinked adapter refuses");
    assert_eq!(unknown.variant_str(), "adapter-not-linked");
}

// An exact package pin succeeds only on the exact compiled identity
// of a published adapter; placeholder identities are bare-only.
#[tokio::test]
async fn exact_pin_matching() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);
    let paths = ExecutionPaths::operator(tmp.path());

    let exact = AdapterSelector::parse("specify:pinned@1.2.3").expect("package selector");
    let resolved =
        provider.ensure_target(&exact, &paths).await.expect("the exact compiled pin ensures");
    assert_eq!(resolved.manifest.name, "pinned");
    assert_eq!(resolved.manifest.version.to_string(), "1.2.3");

    let mismatch = AdapterSelector::parse("specify:pinned@1.0.0").expect("package selector");
    let err = provider
        .ensure_target(&mismatch, &paths)
        .await
        .expect_err("a mismatched pin refuses before any cache mutation");
    assert_eq!(err.variant_str(), "adapter-not-linked");
    // The refusal names the linked identity the pin missed.
    assert!(err.to_string().contains("pinned@1.2.3"), "{err}");

    // The mock probe compiles with the `0.0.0` development
    // placeholder, so even its "exact" pin refuses: unpublished
    // identities match only bare references.
    let placeholder = AdapterSelector::parse("specify:mock@0.0.0").expect("package selector");
    let err =
        provider.ensure_target(&placeholder, &paths).await.expect_err("a placeholder pin refuses");
    assert_eq!(err.variant_str(), "adapter-not-linked");
    assert!(err.to_string().contains("placeholder"), "{err}");
}

// A local component selector can never select a same-named compiled
// adapter.
#[tokio::test]
async fn component_selector_refused() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);
    let paths = ExecutionPaths::operator(tmp.path());

    let component = AdapterSelector::parse("./mock.wasm").expect("component selector");
    let err = provider
        .ensure_target(&component, &paths)
        .await
        .expect_err("linked execution does not load supplied components");
    assert_eq!(err.variant_str(), "adapter-not-linked");
    assert!(err.to_string().contains("does not load the supplied component"), "{err}");
}

// The runtime `specify_floor` gate stays active for linked entries:
// compilation proves trait compatibility, not semantic compatibility.
#[test]
fn floor_enforced_at_resolve() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);
    let paths = ExecutionPaths::operator(tmp.path());

    let err = provider
        .resolve_target(&bare("floored"), &paths)
        .expect_err("a floor above the running binary refuses");
    assert_eq!(err.variant_str(), "adapter-cli-too-old");
}

#[tokio::test]
async fn guidance_crosses_workflow_seam() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);

    let prompt = provider.guidance("target:mock".to_string()).await.expect("guidance dispatches");
    assert_eq!(prompt, "mock guidance");

    // The adapter's typed guidance error survives catalog dispatch and
    // the SDK-to-workflow error mapping.
    let err = provider
        .guidance("target:mock-fail-guidance".to_string())
        .await
        .expect_err("the failing identity fails guidance");
    assert!(
        matches!(err, seam::Error::Internal(detail) if detail.contains("mock-fail-guidance")),
        "the typed error crosses the workflow seam"
    );
}

#[tokio::test]
async fn survey_crosses_workflow_seam() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // The probe's survey threads the model, so the scripted answer
    // crossing the seam proves the model reached the adapter leg.
    let provider = provider(tmp.path(), &["greeting"]);

    let leads = provider.survey("source:mock".to_string()).await.expect("survey dispatches");
    assert_eq!(leads.len(), 1);
    assert_eq!(leads[0].lead, "greeting");
    assert_eq!(leads[0].synopsis, "surveyed by source:mock");
}

// The extract leg threads its lead and surfaces the adapter's typed
// error across the seam.
#[tokio::test]
async fn extract_crosses_workflow_seam() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);

    let lead = seam::Lead {
        lead: "password-reset".to_string(),
        synopsis: String::new(),
        topics: Vec::new(),
    };
    let err = provider
        .extract("source:mock".to_string(), lead)
        .await
        .expect_err("the probe's extract fails with a typed error naming the lead");
    assert!(
        matches!(err, seam::Error::Internal(detail) if detail.contains("password-reset")),
        "the lead reached the adapter leg"
    );
}

// The build and merge legs thread slice, inputs, and phase; the probe
// echoes them through the report's single output path.
#[tokio::test]
async fn build_and_merge_cross_workflow_seam() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);
    let tree = || seam::WorkingTree {
        base: "live".to_string(),
        subpath: None,
    };

    let inputs = vec![seam::Input::Proposal("BODY".to_string())];
    let report = provider
        .build("target:mock".to_string(), "demo".to_string(), inputs, tree())
        .await
        .expect("build dispatches");
    assert_eq!(report.outputs[0].path, "build:demo:1");

    let report = provider
        .merge("target:mock".to_string(), "demo".to_string(), seam::MergePhase::Preflight, tree())
        .await
        .expect("merge dispatches");
    assert_eq!(report.outputs[0].path, "merge:demo:preflight");
}

// A routed id never crosses axes, and unlinked ids refuse on both.
#[tokio::test]
async fn axis_routing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);

    let err = provider
        .survey("target:mock".to_string())
        .await
        .expect_err("a target id never reaches the source legs");
    assert!(matches!(err, seam::Error::InvalidRequest(detail) if detail.contains("target:mock")));

    let err = provider
        .guidance("source:mock".to_string())
        .await
        .expect_err("a source id never reaches the target legs");
    assert!(matches!(err, seam::Error::InvalidRequest(_)));

    let err = provider.survey("source:unknown".to_string()).await.expect_err("unlinked refuses");
    assert!(
        matches!(err, seam::Error::InvalidRequest(detail) if detail.contains("source:unknown"))
    );
}
