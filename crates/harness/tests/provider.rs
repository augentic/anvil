//! Provider gates over the shared probe implementors: linked-only
//! resolution, pinned-identity refusal, hydrator refusal, and the
//! operation legs crossing the workflow seam.

mod support;

use harness::catalog::Catalog;
use harness::provider::Provider;
use omnia_testkit::model::Scripted;
use project::adapter::{AdapterRef, Hydrator as _, Resolver as _};
use project::seam::{self, Source as _, Target as _};
use support::{FailGuidance, Probe};

fn provider(root: &std::path::Path, answers: &[&str]) -> Provider<Scripted> {
    let catalog: Catalog<Scripted> =
        Catalog::builder().source::<Probe>().target::<Probe>().target::<FailGuidance>().build();
    Provider::new(root, Scripted::answers(answers.iter().copied()), catalog)
}

#[test]
fn linked_resolution() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);

    let source = provider
        .resolve_source(&AdapterRef::bare("fixture"), tmp.path())
        .expect("linked source resolves");
    assert_eq!(source.origin.label, "native");
    assert_eq!(source.origin.reference, "rust:source:fixture");

    let target = provider
        .resolve_target(&AdapterRef::bare("fixture"), tmp.path())
        .expect("linked target resolves");
    assert_eq!(target.origin.reference, "rust:target:fixture");

    let unknown = provider
        .resolve_target(&AdapterRef::bare("unknown"), tmp.path())
        .expect_err("unlinked adapter refuses");
    assert_eq!(unknown.variant_str(), "adapter-not-found");
}

// The provider stays linked-only: pinned identities are component
// deployments, and hydration always refuses.
#[test]
fn pinned_and_hydration_refused() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path(), &[]);

    let pinned = provider
        .resolve_target(
            &AdapterRef::pinned("fixture", "1.0.0".parse().expect("semver")),
            tmp.path(),
        )
        .expect_err("pinned identities remain component-only");
    assert_eq!(pinned.variant_str(), "adapter-not-found");

    let hydrate = futures_lite(provider.fetch("https://registry/fixture@1.0.0.wasm"))
        .expect_err("the linked provider hydrates nothing");
    assert_eq!(hydrate.variant_str(), "adapter-hydrate-unavailable");
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

// Minimal block-on for the one async trait call in a sync test.
fn futures_lite<F: Future>(future: F) -> F::Output {
    let runtime = tokio::runtime::Builder::new_current_thread().build().expect("runtime");
    runtime.block_on(future)
}
