//! Launcher integration coverage over the public [`launcher::Policy`]
//! assembly and the guest resolver's typed kernel: argv-anchored
//! mounts (including the `adapter add` seed preopen), adapter
//! resolution with the package-pin pull-on-miss install leg, and
//! fail-closed store verification.
//!
//! The launcher is the only downloader in the deployment: a pinned
//! store miss installs from an OCI registry (tests compose an
//! in-process read-only registry through `Registry::insecure`; the
//! shipped binary hard-codes the first-party GHCR base), while
//! unpinned ids stay verify-and-load over the seeded project cache.
//! Every test injects explicit [`Locations`] rooted in a tempdir
//! through `Policy::new` — the same explicit-layout seam sandboxes
//! use — so no process environment is read or mutated.

mod registry;

use std::path::PathBuf;

use launcher::{Policy, Registry, Resolver};
use project::handler::{CachePlacement, ExecutionPaths, Locations};
use registry::TestRegistry;

/// One sandboxed invocation context: a project directory plus explicit
/// store and cache roots, all inside one tempdir.
struct Sandbox {
    root: PathBuf,
    store: PathBuf,
    locations: Locations,
    _tmp: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let base = tmp.path().canonicalize().expect("canonical tempdir");
        let root = base.join("project");
        let store = base.join("store");
        let cache = base.join("cache");
        for dir in [&root, &store, &cache] {
            std::fs::create_dir_all(dir).expect("mkdir sandbox dir");
        }
        let locations = Locations::explicit(store.clone(), CachePlacement::Parent(cache));
        Self {
            root,
            store,
            locations,
            _tmp: tmp,
        }
    }

    /// Seed a stub component into the project component cache — the
    /// single bare-name probe under the carried cache placement (what
    /// `emery adapter add` leaves behind).
    fn seed_cached_component(&self, name: &str) -> PathBuf {
        let components =
            ExecutionPaths::new(&self.root, self.locations.clone()).cache_dir().join("components");
        std::fs::create_dir_all(&components).expect("mkdir component cache");
        let path = components.join(format!("{name}.wasm"));
        std::fs::write(&path, format!("{name} cached component")).expect("write cached component");
        path
    }

    /// Install a pinned adapter into the sandbox store with a valid
    /// digest sidecar — the state a prior pull-on-miss install leaves
    /// behind.
    fn seed_store_adapter(&self, name: &str, version: &str) -> Vec<u8> {
        let bytes = format!("{name} {version} store bytes").into_bytes();
        let entry = self.store.join(format!("{name}@{version}.wasm"));
        std::fs::write(&entry, &bytes).expect("write store entry");
        let digest = diagnostics::cache::file_content_digest(&entry);
        let meta = self.store.join(format!("{name}@{version}.meta"));
        diagnostics::cache::write_store_meta(&meta, &digest, None).expect("write store sidecar");
        bytes
    }

    fn policy(&self, args: &[&str]) -> Policy {
        Policy::new(&self.root, &argv(args), self.locations.clone())
    }

    /// The deployment's resolver, assembled the way the binary does
    /// it (first-party registry base — safe for tests that never hit
    /// a pinned store miss).
    fn resolver(&self) -> Resolver {
        self.policy(&["plan", "status"]).resolver()
    }

    /// A resolver over an explicit registry base — the pull-on-miss
    /// integration seam.
    fn resolver_over(&self, registry: Registry) -> Resolver {
        Resolver::with_registry(ExecutionPaths::new(&self.root, self.locations.clone()), registry)
    }

    /// A resolver whose registry base refuses connections — the
    /// deterministic offline stand-in.
    fn offline_resolver(&self) -> Resolver {
        self.resolver_over(Registry::insecure("127.0.0.1:1/adapters"))
    }
}

fn argv(args: &[&str]) -> Vec<String> {
    args.iter().map(ToString::to_string).collect()
}

fn code(err: &error::Error) -> &str {
    match err {
        error::Error::Diag { code, .. } => code,
        other => panic!("expected a Diag error, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Deployment policy: the mounts anchor from argv and the working
// directory; the writable mount directories are created pre-run.

#[test]
fn mounts_are_the_well_known_locations() {
    let sandbox = Sandbox::new();
    let policy = sandbox.policy(&["registry", "validate"]);

    assert_eq!(policy.project_root(), sandbox.root);
    // The writable mount directories are created pre-run so the
    // guest's preopens exist. The global store gets no guest mount —
    // it is host-owned (the install leg creates it on demand).
    assert!(policy.cache_dir().is_dir());
    // No seed in argv: the seed preopen degenerates to a harmless
    // read-only duplicate of the project mount.
    assert_eq!(policy.seed_dir(), sandbox.root);
    assert_eq!(policy.seed_mount_name(), sandbox.root.display().to_string());
}

#[test]
fn anchors_at_the_project_root_ancestor() {
    let sandbox = Sandbox::new();
    let emery = sandbox.root.join(".emery");
    std::fs::create_dir_all(&emery).expect("mkdir .emery");
    std::fs::write(
        emery.join("project.yaml"),
        format!(
            "name: fixture\nadapter: mock\nemery: {}\nrules: {{}}\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("write project.yaml");
    let nested = sandbox.root.join("src/deeply/nested");
    std::fs::create_dir_all(&nested).expect("mkdir nested dir");

    let policy = Policy::new(&nested, &argv(&["slice", "build", "s1"]), sandbox.locations.clone());
    assert_eq!(policy.project_root(), sandbox.root);
}

#[test]
fn unparseable_argv_anchors_at_the_walked_directory() {
    // Argv the grammar refuses still boots: the guest renders the
    // rejection, so the policy must stay total and anchor at the
    // working directory.
    let sandbox = Sandbox::new();
    let policy = sandbox.policy(&["frobnicate"]);
    assert_eq!(policy.project_root(), sandbox.root);
    assert_eq!(policy.seed_dir(), sandbox.root);
}

// ---------------------------------------------------------------------------
// The `adapter add` seed preopen: the operator's component directory,
// named by its own absolute host path so the guest opens the argv path
// unchanged.

#[test]
fn seed_preopen_grants_the_component_directory() {
    let sandbox = Sandbox::new();
    let built = sandbox.root.parent().expect("sandbox base").join("built");
    std::fs::create_dir_all(&built).expect("mkdir build dir");
    std::fs::write(built.join("demo.wasm"), b"freshly built component").expect("write component");

    let component = built.join("demo.wasm").display().to_string();
    let policy = sandbox.policy(&["adapter", "add", &component]);
    assert_eq!(policy.seed_dir(), built);
    assert_eq!(policy.seed_mount_name(), built.display().to_string());
}

#[test]
fn relative_seed_resolves_against_the_project_root() {
    let sandbox = Sandbox::new();
    let nested = sandbox.root.join("dist");
    std::fs::create_dir_all(&nested).expect("mkdir dist");
    std::fs::write(nested.join("demo.wasm"), b"component").expect("write component");

    let policy = sandbox.policy(&["adapter", "add", "./dist/demo.wasm"]);
    assert_eq!(policy.seed_dir(), nested);
}

#[test]
fn seed_project_dir_anchors_the_project_mount() {
    let sandbox = Sandbox::new();
    let elsewhere = sandbox.root.parent().expect("sandbox base").join("other-project");
    std::fs::write(sandbox.root.join("demo.wasm"), b"component").expect("write component");

    let policy = sandbox.policy(&[
        "adapter",
        "add",
        &sandbox.root.join("demo.wasm").display().to_string(),
        "--project-dir",
        &elsewhere.display().to_string(),
    ]);
    assert_eq!(policy.project_root(), elsewhere);
    // The anchored project directory is created so its mount opens.
    assert!(elsewhere.is_dir());
    assert_eq!(policy.seed_dir(), sandbox.root);
}

#[test]
fn missing_seed_directory_degenerates_to_the_project_root() {
    // A typo'd component directory must not fail the boot-time preopen
    // open: the guest renders `adapter-component-missing` itself.
    let sandbox = Sandbox::new();
    let policy = sandbox.policy(&["adapter", "add", "/nonexistent/dir/demo.wasm"]);
    assert_eq!(policy.seed_dir(), sandbox.root);
}

// ---------------------------------------------------------------------------
// Adapter legs: store hits and cache-backed ids are verify-and-load;
// pinned store misses go through the pull-on-miss install leg.

#[tokio::test]
async fn store_adapter_verify_and_load() {
    let sandbox = Sandbox::new();
    let expected = sandbox.seed_store_adapter("mock", "1.0.0");

    let bytes = sandbox
        .resolver()
        .resolve_component("target:mock@1.0.0")
        .await
        .expect("store adapter resolves offline");
    assert_eq!(bytes, expected);
}

#[tokio::test]
async fn cold_pinned_miss_offline_is_a_hard_failure() {
    // A pinned store miss triggers the install leg; without a
    // reachable registry it fails deterministically, naming the
    // package identity and the OCI reference.
    let sandbox = Sandbox::new();
    let err = sandbox
        .offline_resolver()
        .resolve_component("target:mock@9.9.9")
        .await
        .expect_err("an offline cold miss fails deterministically");
    assert_eq!(code(&err), "adapter-install-failed");
    let detail = err.to_string();
    assert!(detail.contains("emery:mock@9.9.9"), "{detail}");
    assert!(detail.contains("mock:9.9.9"), "names the OCI reference: {detail}");
    // The operator-facing bare-miss surface at init/author lands here
    // (a bare cache-miss name auto-pins to the train before install),
    // so the failure carries the recoveries: name check, local seed,
    // explicit pin.
    assert!(detail.contains("spelled correctly"), "name-check recovery: {detail}");
    assert!(detail.contains("emery adapter add"), "local-seed recovery: {detail}");
    assert!(detail.contains("emery:mock@<semver>"), "explicit-pin recovery: {detail}");
}

#[tokio::test]
async fn simultaneous_versions_resolve_distinctly() {
    let sandbox = Sandbox::new();
    let one = sandbox.seed_store_adapter("mock", "1.0.0");
    let two = sandbox.seed_store_adapter("mock", "2.0.0");
    assert_ne!(one, two);

    let resolver = sandbox.resolver();
    assert_eq!(resolver.resolve_component("target:mock@1.0.0").await.expect("v1"), one);
    assert_eq!(resolver.resolve_component("target:mock@2.0.0").await.expect("v2"), two);
}

#[tokio::test]
async fn cache_backed_ids_resolve_the_project_cache() {
    let sandbox = Sandbox::new();
    let target = sandbox.seed_cached_component("mock");
    let source = sandbox.seed_cached_component("mock-source");

    let resolver = sandbox.resolver();
    assert_eq!(
        resolver.resolve_component("target:mock").await.expect("cached target"),
        std::fs::read(&target).expect("read cached target")
    );
    assert_eq!(
        resolver.resolve_component("source:mock-source").await.expect("cached source"),
        std::fs::read(&source).expect("read cached source")
    );
}

#[tokio::test]
async fn cache_miss_is_a_hard_failure() {
    // Unpinned ids stay verify-and-load: no install leg exists for
    // the project component cache.
    let sandbox = Sandbox::new();
    let err = sandbox
        .resolver()
        .resolve_component("target:mock")
        .await
        .expect_err("empty cache fails deterministically");
    assert_eq!(code(&err), "adapter-not-found");
}

#[tokio::test]
async fn adapter_missing_sidecar_is_refused() {
    let sandbox = Sandbox::new();
    std::fs::write(sandbox.store.join("mock@1.0.0.wasm"), b"mock without provenance")
        .expect("write unverifiable adapter entry");

    let err = sandbox
        .resolver()
        .resolve_component("target:mock@1.0.0")
        .await
        .expect_err("sidecar-less store entry");
    assert_eq!(code(&err), "adapter-sidecar-missing");
}

#[tokio::test]
async fn adapter_digest_drift_is_refused() {
    let sandbox = Sandbox::new();
    sandbox.seed_store_adapter("mock", "1.0.0");
    std::fs::write(sandbox.store.join("mock@1.0.0.wasm"), b"tampered adapter bytes")
        .expect("tamper with store entry");

    let err = sandbox
        .resolver()
        .resolve_component("target:mock@1.0.0")
        .await
        .expect_err("tampered store entry");
    assert_eq!(code(&err), "adapter-digest-mismatch");
}

#[tokio::test]
async fn engine_identities_are_not_resolvable() {
    // The engine guest is embedded and registered statically at boot;
    // its package identity never reaches the resolver, and asking for
    // it fails like any other identity outside the routed grammar.
    let sandbox = Sandbox::new();
    let err = sandbox
        .resolver()
        .resolve_component(&format!("emery:engine@{}", env!("CARGO_PKG_VERSION")))
        .await
        .expect_err("no engine leg exists");
    assert_eq!(code(&err), "adapter-routed-id-malformed");
}

#[tokio::test]
async fn malformed_ids_fail_deterministically() {
    let sandbox = Sandbox::new();
    sandbox.seed_cached_component("mock");
    let resolver = sandbox.resolver();

    for malformed in ["mock", "widget:mock", "target:", "target:mock@not-semver"] {
        let err =
            resolver.resolve_component(malformed).await.expect_err("outside the routed grammar");
        assert_eq!(code(&err), "adapter-routed-id-malformed", "{malformed}");
    }
}

// ---------------------------------------------------------------------------
// Pull-on-miss install over the in-process OCI registry: cold misses
// on both axes, provenance recording, offline reuse, and artifact
// validation.

/// A syntactically valid component payload (wasm magic + filler).
fn component_bytes(tag: &str) -> Vec<u8> {
    let mut bytes = b"\0asm".to_vec();
    bytes.extend_from_slice(tag.as_bytes());
    bytes
}

#[tokio::test]
async fn cold_miss_installs_and_records_provenance() {
    let sandbox = Sandbox::new();
    let server = TestRegistry::serve().await;
    let expected = component_bytes("mock 1.0.0");
    let manifest_digest = server.publish("mock", "1.0.0", expected.clone());

    let resolver = sandbox.resolver_over(server.registry());
    let bytes = resolver
        .resolve_component("target:mock@1.0.0")
        .await
        .expect("cold pinned miss installs from the registry");
    assert_eq!(bytes, expected);

    // The install lands as the immutable store entry plus a sidecar
    // recording the tree digest and the OCI provenance.
    let entry = sandbox.store.join("mock@1.0.0.wasm");
    assert_eq!(std::fs::read(&entry).expect("installed entry"), expected);
    let meta = sandbox.store.join("mock@1.0.0.meta");
    assert_eq!(
        diagnostics::cache::read_store_meta(&meta).as_deref(),
        Some(diagnostics::cache::file_content_digest(&entry).as_str()),
    );
    let provenance =
        diagnostics::cache::read_store_provenance(&meta).expect("recorded OCI provenance");
    assert_eq!(provenance.repository, format!("{}/mock", server.prefix()));
    assert_eq!(provenance.manifest_digest, manifest_digest);
    assert_eq!(
        provenance.layer_digest,
        format!("sha256:{}", diagnostics::digest::sha256_hex(&expected)),
    );
}

#[tokio::test]
async fn both_axes_install_from_one_store() {
    // The store carries no axis segment: a source pin and a target
    // pin install through the identical leg.
    let sandbox = Sandbox::new();
    let server = TestRegistry::serve().await;
    let source = component_bytes("intent source");
    let target = component_bytes("omnia target");
    server.publish("intent", "0.5.0", source.clone());
    server.publish("omnia", "0.5.0", target.clone());

    let resolver = sandbox.resolver_over(server.registry());
    assert_eq!(
        resolver.resolve_component("source:intent@0.5.0").await.expect("source cold miss"),
        source
    );
    assert_eq!(
        resolver.resolve_component("target:omnia@0.5.0").await.expect("target cold miss"),
        target
    );
}

#[tokio::test]
async fn installed_pin_reuses_offline() {
    let sandbox = Sandbox::new();
    let server = TestRegistry::serve().await;
    let expected = component_bytes("mock 1.0.0");
    server.publish("mock", "1.0.0", expected.clone());

    let resolver = sandbox.resolver_over(server.registry());
    resolver.resolve_component("target:mock@1.0.0").await.expect("cold miss installs");

    // Registry gone: the installed entry keeps resolving offline.
    drop(server);
    let bytes = resolver
        .resolve_component("target:mock@1.0.0")
        .await
        .expect("second resolve is a store hit, no network");
    assert_eq!(bytes, expected);
}

// ---------------------------------------------------------------------------
// The MCP HTTP route: `/mcp/<axis>/<name>[@<version>]` maps back
// onto the routed adapter id the grant URL was derived from; anything
// outside the routed grammar is `None` (an ordinary 404).

#[test]
fn mcp_route_maps_routed_ids() {
    for (path, id) in [
        ("/mcp/target/omnia", "target:omnia"),
        ("/mcp/source/typescript", "source:typescript"),
        ("/mcp/target/omnia@1.2.3", "target:omnia@1.2.3"),
        // A trailing subpath belongs to the shelf, not the identity.
        ("/mcp/source/intent/messages", "source:intent"),
    ] {
        let guest = launcher::mcp_route(path).expect(path);
        assert_eq!(guest.as_str(), id, "{path}");
    }
}

#[test]
fn mcp_route_declines_paths_outside_the_grammar() {
    for path in [
        "/",
        "/health",
        "/mcp",
        "/mcp/",
        "/mcp/target",
        "/mcp/target/",
        "/mcp/plugin/omnia",
        "/mcp/target/omnia@1",
        "/mcp/target/omnia@not-semver",
    ] {
        assert!(launcher::mcp_route(path).is_none(), "{path} must decline");
    }
}

#[tokio::test]
async fn non_wasm_artifact_is_refused() {
    let sandbox = Sandbox::new();
    let server = TestRegistry::serve().await;
    server.publish("mock", "1.0.0", b"#!/bin/sh\necho gotcha".to_vec());

    let err = sandbox
        .resolver_over(server.registry())
        .resolve_component("target:mock@1.0.0")
        .await
        .expect_err("a non-wasm layer is refused");
    assert_eq!(code(&err), "adapter-install-invalid");
    assert!(
        !sandbox.store.join("mock@1.0.0.wasm").exists(),
        "nothing lands in the store on a refused install"
    );
}
