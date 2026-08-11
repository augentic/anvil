//! `emery adapter add` — the pre-init cache-seeding surface: seed →
//! bare init, seed → bare source resolve, per-component provenance
//! sidecars that never clobber each other, and same-name reseeding as
//! the replacement act.

use std::fs;

mod support;

use mock::invoke::run;
use project::adapter::handlers::{AdapterAdd, AddInput};
use project::adapter::{AdapterSelector, ComponentMeta, Resolver as _};
use project::handler::Anchor as _;
use support::Provider;

async fn seed(provider: &Provider, component: &str) -> project::adapter::handlers::AddBody {
    run::<AdapterAdd, _, _>(
        provider,
        AddInput {
            component: component.into(),
            project_dir: None,
        },
    )
    .await
    .expect("adapter add seeds the cache")
}

fn stage(provider: &Provider, relative: &str, bytes: &[u8]) {
    let path = provider.root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, bytes).expect("stage component");
}

#[tokio::test]
async fn target_seed_then_bare_init() {
    // No `.emery/` exists yet: the seed lands in the out-of-tree
    // cache, and the subsequent bare init resolves it as the project
    // target without any development build present.
    let project = Provider::bare();
    stage(&project, "downloads/demo.wasm", b"\0asm-target");

    let body = seed(&project, "downloads/demo.wasm").await;
    assert_eq!(body.name, "demo");
    assert!(!project.root.join(".emery").exists(), "seeding scaffolds nothing");

    let init = run::<project::init::handlers::Init, _, _>(
        &project,
        project::init::handlers::InitInput {
            adapter: Some("demo".into()),
            name: Some("demo-project".into()),
            description: None,
            platforms: None,
            upgrade: false,
        },
    )
    .await
    .expect("bare init resolves the seeded cache entry");
    assert_eq!(init.adapter_name, "demo");
    assert!(init.cache_present, "init projects the seeded component's sidecar");
}

#[tokio::test]
async fn source_seed_resolves_bare() {
    // Axis-neutral seeding: the same cache entry satisfies a bare
    // source resolve — the binding supplies the axis, not the seed.
    let project = Provider::bare();
    stage(&project, "downloads/docs-source.wasm", b"\0asm-source");

    let body = seed(&project, "downloads/docs-source.wasm").await;
    assert_eq!(body.name, "docs-source");

    let resolved = support::resolver()
        .resolve_source(
            &AdapterSelector::parse("docs-source").expect("bare selector"),
            project.paths(),
        )
        .expect("the seeded entry resolves the bare source binding");
    assert_eq!(resolved.manifest.name, "docs-source");
}

#[tokio::test]
async fn sidecars_never_clobber() {
    // Two seeded adapters carry independent `<name>.meta.yaml`
    // provenance — the second seed leaves the first sidecar intact.
    let project = Provider::bare();
    stage(&project, "downloads/alpha.wasm", b"\0asm-alpha");
    stage(&project, "downloads/beta.wasm", b"\0asm-beta");

    seed(&project, "downloads/alpha.wasm").await;
    seed(&project, "downloads/beta.wasm").await;

    let alpha = fs::read_to_string(ComponentMeta::path(project.paths(), "alpha"))
        .expect("alpha sidecar survives the beta seed");
    let beta = fs::read_to_string(ComponentMeta::path(project.paths(), "beta"))
        .expect("beta sidecar written");
    assert!(alpha.contains("alpha.wasm"), "alpha provenance records its own source:\n{alpha}");
    assert!(beta.contains("beta.wasm"), "beta provenance records its own source:\n{beta}");
}

#[tokio::test]
async fn reseeding_replaces() {
    // Re-seeding the same name replaces the entry and its sidecar —
    // the explicit command is the approval act, no staleness check.
    let project = Provider::bare();
    stage(&project, "v1/demo.wasm", b"\0asm-v1");
    stage(&project, "v2/demo.wasm", b"\0asm-v2");

    let first = seed(&project, "v1/demo.wasm").await;
    seed(&project, "v2/demo.wasm").await;

    let entry = fs::read(&first.entry).expect("cache entry");
    assert_eq!(entry, b"\0asm-v2", "the second seed replaced the entry bytes");
    let sidecar = fs::read_to_string(ComponentMeta::path(project.paths(), "demo"))
        .expect("provenance sidecar");
    assert!(sidecar.contains("v2/demo.wasm"), "provenance follows the replacement:\n{sidecar}");
}
