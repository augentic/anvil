//! Provider gates over the shared probe implementors: linked-only
//! resolution, pinned-identity refusal, and the operation legs
//! crossing the workflow seam.

mod support;

use harness::catalog::Catalog;
use harness::provider::Provider;
use omnia_testkit::model::Scripted;
use project::adapter::{AdapterSelector, Resolver as _};
use project::handler::ExecutionPaths;
use project::seam::{self, Source as _, Target as _};
use support::{FailGuidance, Probe};

fn provider(root: &std::path::Path, answers: &[&str]) -> Provider<Scripted> {
    let catalog: Catalog<Scripted> =
        Catalog::builder().source::<Probe>().target::<Probe>().target::<FailGuidance>().build();
    Provider::new(
        ExecutionPaths::operator(root),
        Scripted::answers(answers.iter().copied()),
        catalog,
    )
}

fn bare(name: &str) -> AdapterSelector {
    AdapterSelector::parse(name).expect("bare selector")
}

#[test]
fn linked_resolution() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);
    let paths = ExecutionPaths::operator(tmp.path());

    let source = provider.resolve_source(&bare("fixture"), &paths).expect("linked source resolves");
    assert_eq!(source.origin.label, "native");
    assert_eq!(source.origin.reference, "rust:source:fixture");

    let target = provider.resolve_target(&bare("fixture"), &paths).expect("linked target resolves");
    assert_eq!(target.origin.reference, "rust:target:fixture");

    let unknown =
        provider.resolve_target(&bare("unknown"), &paths).expect_err("unlinked adapter refuses");
    assert_eq!(unknown.variant_str(), "adapter-not-found");
}

// The provider stays linked-only: pinned identities are component
// deployments.
#[test]
fn pinned_refused() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);
    let paths = ExecutionPaths::operator(tmp.path());

    let pinned = provider
        .resolve_target(&AdapterSelector::parse("specify:fixture@1.0.0").expect("package"), &paths)
        .expect_err("pinned identities remain component-only");
    assert_eq!(pinned.variant_str(), "adapter-not-found");
}

#[tokio::test]
async fn guidance_crosses_workflow_seam() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);

    let prompt =
        provider.guidance("target:fixture".to_string()).await.expect("guidance dispatches");
    assert_eq!(prompt, "fixture guidance");

    // The adapter's typed guidance error survives catalog dispatch and
    // the SDK-to-workflow error mapping.
    let err = provider
        .guidance("target:fixture-fail-guidance".to_string())
        .await
        .expect_err("the failing identity fails guidance");
    assert!(
        matches!(err, seam::Error::Internal(detail) if detail.contains("fixture-fail-guidance")),
        "the typed error crosses the workflow seam"
    );
}

#[tokio::test]
async fn survey_crosses_workflow_seam() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // The probe's survey threads the model, so the scripted answer
    // crossing the seam proves the model reached the adapter leg.
    let provider = provider(tmp.path(), &["greeting"]);

    let leads = provider.survey("source:fixture".to_string()).await.expect("survey dispatches");
    assert_eq!(leads.len(), 1);
    assert_eq!(leads[0].lead, "greeting");
    assert_eq!(leads[0].synopsis, "surveyed by source:fixture");
}
