//! Init-time component resolution through the public `Init`
//! operation: the dev-probe path, the linked (component-free)
//! resolver shape, and local-component mirroring into the project
//! cache. The component-free workspace/journal init coverage lives
//! with the slice handler suite.

use std::fs;

mod support;

use mock::invoke::run;
use project::adapter::AdapterSelector;
use project::handler::{Anchor as _, ExecutionPaths};
use support::Provider;

#[tokio::test]
async fn regular_mode() {
    let project = Provider::bare();

    // Stage a fake `demo` component at the resolver's in-repo dev
    // probe path with a digest-valid metadata sidecar beside it:
    // the resolver probes file presence and the sidecar supplies
    // the answer, so no metadata dispatch runs.
    let dev_dir = project.root.join("target/wasm32-wasip2/release");
    fs::create_dir_all(&dev_dir).expect("mkdir dev release dir");
    let component = dev_dir.join("demo.wasm");
    fs::write(&component, b"\0asm-component").expect("stage component");
    let digest = diagnostics::cache::file_content_digest(&component);
    fs::write(
        dev_dir.join("demo.wasm.metadata.json"),
        format!("{{ \"digest\": \"{digest}\", \"metadata\": {{}} }}"),
    )
    .expect("stage metadata sidecar");

    let body = run::<project::init::handlers::Init, _, _>(
        &project,
        project::init::handlers::InitInput {
            adapter: Some("demo".into()),
            name: Some("demo-project".into()),
            description: None,
            workspace: false,
            platforms: None,
            upgrade: false,
        },
    )
    .await
    .expect("scaffold succeeds");
    assert_eq!(body.adapter_name, "demo");
    let config =
        fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
    assert!(config.contains("adapter: demo"), "the adapter is recorded:\n{config}");
    assert!(project.root.join(".specify/slices").is_dir(), "the slice tree is scaffolded");

    assert!(body.context_generated, "init generates AGENTS.md context when absent");
    let agents = fs::read_to_string(project.root.join("AGENTS.md")).expect("AGENTS.md");
    assert!(
        agents.contains("<!-- specify:context begin"),
        "the generated context is fenced:\n{agents}"
    );
    assert!(
        agents.contains("adapter `demo`"),
        "the resolved adapter surfaces in Conventions:\n{agents}"
    );
    assert!(
        project.root.join(".specify/context.lock").is_file(),
        "the fingerprint sidecar lands beside the generated context"
    );
}

/// A provider whose resolver answers from memory — the linked-host
/// shape: no component file exists anywhere on disk.
#[derive(Clone)]
struct Linked(Provider);

impl project::handler::Anchor for Linked {
    fn paths(&self) -> &ExecutionPaths {
        self.0.paths()
    }
}

impl project::adapter::Resolver for Linked {
    fn resolve_source(
        &self, selector: &AdapterSelector, _paths: &ExecutionPaths,
    ) -> Result<project::adapter::ResolvedSource, error::Error> {
        project::adapter::resolver::source(
            &selector.name()?,
            linked_version(),
            project::adapter::metadata::Metadata::default(),
            linked_origin(),
        )
    }

    fn resolve_target(
        &self, selector: &AdapterSelector, _paths: &ExecutionPaths,
    ) -> Result<project::adapter::ResolvedTarget, error::Error> {
        project::adapter::resolver::target(
            &selector.name()?,
            linked_version(),
            project::adapter::metadata::Metadata::default(),
            linked_origin(),
        )
    }
}

const fn linked_version() -> semver::Version {
    semver::Version::new(0, 0, 0)
}

fn linked_origin() -> project::adapter::Origin {
    project::adapter::Origin {
        label: "native".to_string(),
        reference: "rust:target:demo".to_string(),
    }
}

#[tokio::test]
async fn regular_mode_component_free() {
    // A bare adapter name is an identity, not a file: init defers
    // component resolution to the injected resolver, so no `.wasm`
    // artifact is staged anywhere for this test.
    let project = Provider::bare();
    let body = omnia_guest::api::invoke::Invoker::new("specify", Linked(project.clone()))
        .invoke::<project::init::handlers::Init>(omnia_guest::api::invocation::Invocation::new(
            project::init::handlers::InitInput {
                adapter: Some("demo".into()),
                name: Some("demo-project".into()),
                description: None,
                workspace: false,
                platforms: None,
                upgrade: false,
            },
        ))
        .await
        .expect("component-free scaffold succeeds");
    assert_eq!(body.adapter_name, "demo");
    let config =
        fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
    assert!(config.contains("adapter: demo"), "the bare identity is recorded:\n{config}");
    assert!(!body.cache_present, "no component is mirrored into the project cache for a bare name");
}

#[tokio::test]
async fn local_component_mirrored() {
    // The only route an externally built adapter has into a project
    // (there is no sibling-checkout probe): an operator-supplied
    // local `.wasm` at init is mirrored into the project component
    // cache, where the bare-name resolver finds it afterwards.
    let project = Provider::bare();
    let staged = project.root.join("downloads/demo.wasm");
    fs::create_dir_all(staged.parent().expect("parent")).expect("mkdir downloads");
    fs::write(&staged, b"\0asm-component").expect("stage local component");

    let body = run::<project::init::handlers::Init, _, _>(
        &project,
        project::init::handlers::InitInput {
            adapter: Some(staged.display().to_string()),
            name: Some("demo-project".into()),
            description: None,
            workspace: false,
            platforms: None,
            upgrade: false,
        },
    )
    .await
    .expect("local-component scaffold succeeds");
    assert_eq!(body.adapter_name, "demo", "the name derives from the component filename");
    assert!(body.cache_present, "the local component is mirrored into the project cache");

    let components = support::expected_cache_dir(&project).join("components");
    assert_eq!(
        fs::read(components.join("demo.wasm")).expect("mirrored component"),
        b"\0asm-component",
        "the mirror is a byte-copy of the supplied file"
    );
    let meta =
        fs::read_to_string(components.join("component-meta.yaml")).expect("provenance stamp");
    assert!(meta.contains("file://"), "provenance records the local source:\n{meta}");
    let config =
        fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
    assert!(config.contains("file://"), "the recorded adapter value is the file URI:\n{config}");

    let resolved = project::adapter::Resolver::resolve_target(
        &support::resolver(),
        &AdapterSelector::parse("demo").expect("bare selector"),
        project.paths(),
    )
    .expect("the mirrored component resolves the bare name");
    assert_eq!(resolved.manifest.name, "demo");

    // The persisted selector survives the operator's original file:
    // `--upgrade` re-ensures the recorded `file://` binding through
    // the project-cache mirror after the download is deleted.
    fs::remove_file(&staged).expect("remove the operator's original file");
    let body = run::<project::init::handlers::Init, _, _>(
        &project,
        project::init::handlers::InitInput {
            adapter: None,
            name: None,
            description: None,
            workspace: false,
            platforms: None,
            upgrade: true,
        },
    )
    .await
    .expect("re-ensure resolves through the mirrored component");
    assert_eq!(body.adapter_name, "demo");
}
