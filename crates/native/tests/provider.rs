//! Provider gates over the shared probe implementors: native ensure
//! (bare and exact-pin matching, mismatched-pin and component-selector
//! refusal, runtime floor enforcement).
//!
//! Linked coverage only: these tests claim nothing about component
//! ABI, WIT mapping, instance isolation, digests, or the adapter
//! store — those gates stay with the component deployment.

mod support;

use native::{Catalog, DynModel, Provider};
use omnia_testkit::model::Scripted;
use project::adapter::{AdapterSelector, Resolver as _};
use project::handler::{CachePlacement, ExecutionPaths, Locations};
use support::{Floored, Pinned, Probe};

// Explicit tempdir-rooted layout: native ensure performs no component
// I/O, but the carried locations stay hermetic regardless.
fn paths(root: &std::path::Path) -> ExecutionPaths {
    let locations =
        Locations::explicit(root.join("store"), CachePlacement::Parent(root.join("project-cache")));
    ExecutionPaths::new(root, locations)
}

fn provider(root: &std::path::Path) -> Provider {
    let catalog = Catalog::builder()
        .source::<Probe>()
        .source::<Floored>()
        .source::<Pinned>()
        .build()
        .expect("valid catalog");
    Provider::new(paths(root), DynModel::new(Scripted::answers::<&str>([])), catalog)
}

fn bare(name: &str) -> AdapterSelector {
    AdapterSelector::parse(name).expect("bare selector")
}

// Bare selectors resolve by name and report the entry's actual
// compiled version, not a placeholder.
#[test]
fn bare_resolution() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path());
    let paths = paths(tmp.path());

    let source = provider.resolve_source(&bare("mock"), &paths).expect("native source resolves");
    assert_eq!(source.manifest.version.as_ref().map(ToString::to_string).as_deref(), Some("0.0.0"));
    assert_eq!(source.origin.label, "native");
    assert_eq!(source.origin.reference, "rust:source:mock@0.0.0");

    let unknown =
        provider.resolve_source(&bare("unknown"), &paths).expect_err("unlinked adapter refuses");
    assert_eq!(unknown.variant_str(), "adapter-not-linked");
}

// An exact package pin succeeds on the exact compiled identity,
// including native mock identities compiled at `0.0.0`.
#[tokio::test]
async fn exact_pin_matching() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path());
    let paths = paths(tmp.path());

    let exact = AdapterSelector::parse("emery:pinned@1.2.3").expect("package selector");
    let resolved =
        provider.ensure_source(&exact, &paths).await.expect("the exact compiled pin ensures");
    assert_eq!(resolved.manifest.name, "pinned");
    assert_eq!(
        resolved.manifest.version.as_ref().map(ToString::to_string).as_deref(),
        Some("1.2.3")
    );

    let mismatch = AdapterSelector::parse("emery:pinned@1.0.0").expect("package selector");
    let err = provider
        .ensure_source(&mismatch, &paths)
        .await
        .expect_err("a mismatched pin refuses before any cache mutation");
    assert_eq!(err.variant_str(), "adapter-not-linked");
    // The refusal names the linked identity the pin missed.
    assert!(err.to_string().contains("pinned@1.2.3"), "{err}");

    let placeholder = AdapterSelector::parse("emery:mock@0.0.0").expect("package selector");
    let resolved = provider
        .ensure_source(&placeholder, &paths)
        .await
        .expect("an exact pin matching the compiled identity ensures");
    assert_eq!(resolved.manifest.name, "mock");
    assert_eq!(
        resolved.manifest.version.as_ref().map(ToString::to_string).as_deref(),
        Some("0.0.0")
    );
}

// A local component selector can never select a same-named compiled
// adapter.
#[tokio::test]
async fn component_selector() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path());
    let paths = paths(tmp.path());

    let component = AdapterSelector::parse("./mock.wasm").expect("component selector");
    let err = provider
        .ensure_source(&component, &paths)
        .await
        .expect_err("native execution does not load supplied components");
    assert_eq!(err.variant_str(), "adapter-not-linked");
    assert!(err.to_string().contains("does not load the supplied component"), "{err}");
}

// The runtime `emery_floor` gate stays active for linked entries:
// compilation proves trait compatibility, not semantic compatibility.
#[test]
fn floor_enforced_at_resolve() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let provider = provider(tmp.path());
    let paths = paths(tmp.path());

    let err = provider
        .resolve_source(&bare("floored"), &paths)
        .expect_err("a floor above the running binary refuses");
    assert_eq!(err.variant_str(), "adapter-cli-too-old");
}
