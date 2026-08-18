//! Launcher integration coverage over the public [`launcher::Policy`]
//! assembly and the guest resolver's typed kernel: argv-anchored
//! mounts, adapter resolution with the package-pin pull-on-miss
//! install leg, and fail-closed store verification.
//!
//! The launcher is the only downloader in the deployment: a pinned
//! store miss installs from an OCI registry (tests compose an
//! in-process read-only registry through `Registry::insecure`; the
//! shipped binary hard-codes the first-party GHCR base), while
//! unpinned ids resolve local-first — cache seed, else newest store
//! version, else the pull-latest provisioning leg — with the refresh
//! set (`init`) forcing the registry check. Every test injects
//! explicit [`Locations`] rooted in a tempdir through `Policy::new` —
//! the same explicit-layout seam sandboxes use — so no process
//! environment is read or mutated.

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
    /// it (first-party registry base — safe for tests that never
    /// reach a registry pull: pinned store hits, cache seeds, and
    /// store-newest bare resolves).
    fn resolver(&self) -> Resolver {
        self.policy(&["completions", "zsh"]).resolver()
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
fn mounts_are_well_known() {
    let sandbox = Sandbox::new();
    let policy = sandbox.policy(&["completions", "zsh"]);

    assert_eq!(policy.project_root(), sandbox.root);
    // The writable mount directories are created pre-run so the
    // guest's preopens exist. The global store gets no guest mount —
    // it is host-owned (the install leg creates it on demand).
    assert!(policy.cache_dir().is_dir());
}

#[test]
fn anchors_at_project_root() {
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

    let policy = Policy::new(&nested, &argv(&["specify"]), sandbox.locations.clone());
    assert_eq!(policy.project_root(), sandbox.root);
}

#[test]
fn unparseable_argv_anchors() {
    // Argv the grammar refuses still boots: the guest renders the
    // rejection, so the policy must stay total and anchor at the
    // working directory.
    let sandbox = Sandbox::new();
    let policy = sandbox.policy(&["frobnicate"]);
    assert_eq!(policy.project_root(), sandbox.root);
}

#[test]
fn unanchored_cwd_in_place() {
    // No `project.yaml` ancestor: the policy stays total — it boots
    // in-place at the cwd (pre-init) so `emery init` works and later
    // verbs fail typed in-guest.
    let sandbox = Sandbox::new();
    let policy = sandbox.policy(&["specify"]);
    assert_eq!(policy.project_root(), sandbox.root);
}

// ---------------------------------------------------------------------------
// The refresh surface: `init <bare-name>` refreshes that name and
// `init --upgrade` refreshes the recorded `project.yaml` binding.

#[test]
fn init_bare_name_refreshes() {
    let sandbox = Sandbox::new();
    let policy = sandbox.policy(&["init", "mock"]);
    assert!(policy.refresh().contains("mock"), "an `init <bare-name>` joins the refresh set");
}

#[test]
fn upgrade_refreshes() {
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

    let policy = sandbox.policy(&["init", "--upgrade"]);
    assert!(
        policy.refresh().contains("mock"),
        "`init --upgrade` refreshes the recorded bare binding"
    );
}

#[test]
fn log_flags_keep_refresh() {
    // Omnia peels `--debug` / `--quiet` before the guest sees argv; the
    // policy sees raw process argv, so it must apply the same peel or
    // the refresh grammar parse would fail and drop the projection.
    let sandbox = Sandbox::new();
    let policy = sandbox.policy(&["--debug", "init", "mock", "--quiet"]);
    assert!(policy.refresh().contains("mock"));
}

// ---------------------------------------------------------------------------
// Adapter legs: store hits and cache-backed ids are verify-and-load;
// pinned store misses go through the pull-on-miss install leg.

#[tokio::test]
async fn store_adapter_verify_load() {
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
async fn cold_pin_offline_fails() {
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
    // The operator-facing pinned-miss surface at init/author lands
    // here, so the failure carries the recoveries: name check, local
    // seed, explicit pin.
    assert!(detail.contains("spelled correctly"), "name-check recovery: {detail}");
    assert!(detail.contains("emery adapter add"), "local-seed recovery: {detail}");
    assert!(detail.contains("emery:mock@<semver>"), "explicit-pin recovery: {detail}");
}

#[tokio::test]
async fn versions_resolve() {
    let sandbox = Sandbox::new();
    let one = sandbox.seed_store_adapter("mock", "1.0.0");
    let two = sandbox.seed_store_adapter("mock", "2.0.0");
    assert_ne!(one, two);

    let resolver = sandbox.resolver();
    assert_eq!(resolver.resolve_component("target:mock@1.0.0").await.expect("v1"), one);
    assert_eq!(resolver.resolve_component("target:mock@2.0.0").await.expect("v2"), two);
}

#[tokio::test]
async fn cache_backed_ids_resolve() {
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
async fn bare_miss_offline_fails() {
    // A bare id with nothing local (no cache seed, no store entry)
    // reaches the pull-latest provisioning leg; without a reachable
    // registry the tag listing fails deterministically with the
    // recovery hints.
    let sandbox = Sandbox::new();
    let err = sandbox
        .offline_resolver()
        .resolve_component("target:mock")
        .await
        .expect_err("an offline total miss fails deterministically");
    assert_eq!(code(&err), "adapter-latest-failed");
    let detail = err.to_string();
    assert!(detail.contains("spelled correctly"), "name-check recovery: {detail}");
    assert!(detail.contains("emery adapter add"), "local-seed recovery: {detail}");
    assert!(detail.contains("emery:mock@<semver>"), "explicit-pin recovery: {detail}");
}

#[tokio::test]
async fn missing_sidecar_refused() {
    // An unverifiable entry triggers one reinstall-in-place; with
    // the registry unreachable the heal fails and the original
    // verification refusal stands — without destroying the local
    // artifact.
    let sandbox = Sandbox::new();
    let entry = sandbox.store.join("mock@1.0.0.wasm");
    std::fs::write(&entry, b"mock without provenance").expect("write unverifiable adapter entry");

    let err = sandbox
        .offline_resolver()
        .resolve_component("target:mock@1.0.0")
        .await
        .expect_err("sidecar-less store entry");
    assert_eq!(code(&err), "adapter-sidecar-missing");
    assert!(entry.is_file(), "a failed heal must not unlink the local artifact");
}

#[tokio::test]
async fn digest_drift_refused() {
    let sandbox = Sandbox::new();
    sandbox.seed_store_adapter("mock", "1.0.0");
    let entry = sandbox.store.join("mock@1.0.0.wasm");
    std::fs::write(&entry, b"tampered adapter bytes").expect("tamper with store entry");

    let err = sandbox
        .offline_resolver()
        .resolve_component("target:mock@1.0.0")
        .await
        .expect_err("tampered store entry, heal offline");
    assert_eq!(code(&err), "adapter-digest-mismatch");
    assert!(entry.is_file(), "a failed heal must not unlink the local artifact");
    assert!(
        sandbox.store.join("mock@1.0.0.meta").is_file(),
        "a failed heal must not unlink the sidecar"
    );
}

#[tokio::test]
async fn engine_ids_not_resolvable() {
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
async fn malformed_ids_fail() {
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
async fn cold_miss_install() {
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
async fn both_axes_share_one_store() {
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
async fn torn_install_heals() {
    // A component without its sidecar (the tear the sidecar-first
    // ordering makes unreachable going forward) is unlinked and
    // reinstalled on the next pinned resolve.
    let sandbox = Sandbox::new();
    let server = TestRegistry::serve().await;
    let expected = component_bytes("mock 1.0.0");
    server.publish("mock", "1.0.0", expected.clone());
    std::fs::write(sandbox.store.join("mock@1.0.0.wasm"), b"torn install remnant")
        .expect("write torn entry");

    let bytes = sandbox
        .resolver_over(server.registry())
        .resolve_component("target:mock@1.0.0")
        .await
        .expect("the unverifiable entry heals through reinstall");
    assert_eq!(bytes, expected);
    diagnostics::cache::verify_store_entry(
        &sandbox.store.join("mock@1.0.0.wasm"),
        &sandbox.store.join("mock@1.0.0.meta"),
    )
    .expect("the healed entry verifies");
}

#[tokio::test]
async fn drifted_pin_reinstalls() {
    let sandbox = Sandbox::new();
    sandbox.seed_store_adapter("mock", "1.0.0");
    std::fs::write(sandbox.store.join("mock@1.0.0.wasm"), b"tampered adapter bytes")
        .expect("tamper with store entry");
    let server = TestRegistry::serve().await;
    let expected = component_bytes("mock 1.0.0");
    server.publish("mock", "1.0.0", expected.clone());

    let bytes = sandbox
        .resolver_over(server.registry())
        .resolve_component("target:mock@1.0.0")
        .await
        .expect("the drifted entry heals through reinstall");
    assert_eq!(bytes, expected);
}

#[tokio::test]
async fn meta_orphan_installs() {
    // The inverse tear — sidecar landed, component didn't — is an
    // ordinary store miss: the install overwrites the orphan sidecar.
    let sandbox = Sandbox::new();
    let server = TestRegistry::serve().await;
    let expected = component_bytes("mock 1.0.0");
    server.publish("mock", "1.0.0", expected.clone());
    diagnostics::cache::write_store_meta(
        &sandbox.store.join("mock@1.0.0.meta"),
        "sha256:stale-orphan",
        None,
    )
    .expect("write orphan sidecar");

    let bytes = sandbox
        .resolver_over(server.registry())
        .resolve_component("target:mock@1.0.0")
        .await
        .expect("the orphan sidecar is overwritten by a fresh install");
    assert_eq!(bytes, expected);
}

#[tokio::test]
async fn installed_pin_reuses() {
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
// Local-first bare resolution: cache seed, else newest store version
// (offline), else pull-latest; the refresh set forces the registry
// check but never shadows the cache seed.

#[tokio::test]
async fn bare_miss_installs_newest() {
    // Nothing local: the provisioning leg lists the repository's
    // tags, ignores non-SemVer ones, and installs the maximum.
    let sandbox = Sandbox::new();
    let server = TestRegistry::serve().await;
    server.publish("mock", "1.0.0", component_bytes("mock 1.0.0"));
    let newest = component_bytes("mock 2.1.0");
    server.publish("mock", "2.1.0", newest.clone());
    server.publish("mock", "latest", component_bytes("mock moving tag"));
    server.publish("mock", "not-semver", component_bytes("mock junk tag"));

    let bytes = sandbox
        .resolver_over(server.registry())
        .resolve_component("target:mock")
        .await
        .expect("a bare total miss provisions the newest SemVer");
    assert_eq!(bytes, newest);
    assert!(sandbox.store.join("mock@2.1.0.wasm").is_file(), "installed into the store");
    assert!(!sandbox.store.join("mock@1.0.0.wasm").exists(), "older versions stay uninstalled");
}

#[tokio::test]
async fn semver_tags_latest_none() {
    let sandbox = Sandbox::new();
    let server = TestRegistry::serve().await;
    server.publish("mock", "latest", component_bytes("mock moving tag"));

    let err = sandbox
        .resolver_over(server.registry())
        .resolve_component("target:mock")
        .await
        .expect_err("no exact-SemVer tag to provision");
    assert_eq!(code(&err), "adapter-latest-none");
}

#[tokio::test]
async fn store_newest_offline() {
    // Something local: the newest installed version resolves with no
    // registry consultation (the resolver's registry base refuses
    // connections, so any network attempt would fail the test).
    let sandbox = Sandbox::new();
    sandbox.seed_store_adapter("mock", "1.0.0");
    let newest = sandbox.seed_store_adapter("mock", "2.0.0");

    let bytes = sandbox
        .offline_resolver()
        .resolve_component("target:mock")
        .await
        .expect("the newest store version resolves offline");
    assert_eq!(bytes, newest);
}

#[tokio::test]
async fn cache_seed_shadows_all() {
    // The co-dev seed always wins: with a cache seed, a newer store
    // entry and a newer published version are both ignored.
    let sandbox = Sandbox::new();
    let seeded = sandbox.seed_cached_component("mock");
    sandbox.seed_store_adapter("mock", "2.0.0");
    let server = TestRegistry::serve().await;
    server.publish("mock", "3.0.0", component_bytes("mock 3.0.0"));

    let bytes = sandbox
        .resolver_over(server.registry())
        .resolve_component("target:mock")
        .await
        .expect("the cache seed resolves");
    assert_eq!(bytes, std::fs::read(&seeded).expect("read seed"));
}

#[tokio::test]
async fn settle_journaled() {
    // D5 containment: a non-durable settle (here a cache seed) appends
    // an `adapter.identity.settled` observability fact when the change
    // journal already exists — and never scaffolds one.
    let sandbox = Sandbox::new();
    sandbox.seed_cached_component("mock");
    // An in-place project, so the carried layout journals at
    // `.emery/change/events/`.
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

    sandbox.resolver().resolve_component("target:mock").await.expect("seed resolves");
    let events = sandbox.root.join(".emery/change/events");
    assert!(!events.exists(), "the launcher must not scaffold a change journal");

    std::fs::create_dir_all(&events).expect("mkdir events");
    sandbox.resolver().resolve_component("target:mock").await.expect("seed resolves again");
    let log = std::fs::read_to_string(events.join("launcher.jsonl")).expect("launcher log");
    let event: serde_json::Value =
        serde_json::from_str(log.lines().next().expect("one line")).expect("parseable event");
    assert_eq!(event["event"], "adapter.identity.settled");
    assert_eq!(event["payload"]["adapter"], "target:mock");
    assert_eq!(event["payload"]["origin"], "cache seed");
    assert!(event["payload"].get("version").is_none(), "a seed settles without a version");
}

#[tokio::test]
async fn cache_seed_answers_pins() {
    // The co-dev seed wins for a pinned identity too: detached
    // topology records exact pins, so a seeded local build must not
    // be shadowed by the store or the registry at post-author
    // dispatch (no network attempt — the offline resolver would fail).
    let sandbox = Sandbox::new();
    let seeded = sandbox.seed_cached_component("mock");
    sandbox.seed_store_adapter("mock", "1.0.0");

    let bytes = sandbox
        .offline_resolver()
        .resolve_component("target:mock@1.0.0")
        .await
        .expect("the cache seed answers the pin");
    assert_eq!(bytes, std::fs::read(&seeded).expect("read seed"));
}

#[tokio::test]
async fn refresh_installs_newer() {
    // The explicit update surface: a refreshed name checks the
    // registry even though a store entry exists, installs the newer
    // version, and resolves it.
    let sandbox = Sandbox::new();
    sandbox.seed_store_adapter("mock", "1.0.0");
    let server = TestRegistry::serve().await;
    let newest = component_bytes("mock 2.0.0");
    server.publish("mock", "2.0.0", newest.clone());

    let bytes = sandbox
        .resolver_over(server.registry())
        .refreshing(["mock".to_string()])
        .resolve_component("target:mock")
        .await
        .expect("the refresh installs and resolves the newer version");
    assert_eq!(bytes, newest);
    assert!(sandbox.store.join("mock@2.0.0.wasm").is_file(), "installed into the store");
}

#[tokio::test]
async fn refresh_keeps_current() {
    let sandbox = Sandbox::new();
    let installed = sandbox.seed_store_adapter("mock", "2.0.0");
    let server = TestRegistry::serve().await;
    server.publish("mock", "1.0.0", component_bytes("mock 1.0.0"));
    server.publish("mock", "2.0.0", component_bytes("mock republished 2.0.0"));

    let bytes = sandbox
        .resolver_over(server.registry())
        .refreshing(["mock".to_string()])
        .resolve_component("target:mock")
        .await
        .expect("an up-to-date refresh keeps the installed entry");
    assert_eq!(bytes, installed, "no reinstall over the immutable store entry");
}

#[tokio::test]
async fn refresh_heals_poisoned() {
    // An explicit update on an unverifiable equal-version entry
    // unlinks and reinstalls it rather than failing closed.
    let sandbox = Sandbox::new();
    sandbox.seed_store_adapter("mock", "1.0.0");
    std::fs::write(sandbox.store.join("mock@1.0.0.wasm"), b"tampered adapter bytes")
        .expect("tamper with store entry");
    let server = TestRegistry::serve().await;
    let expected = component_bytes("mock 1.0.0");
    server.publish("mock", "1.0.0", expected.clone());

    let bytes = sandbox
        .resolver_over(server.registry())
        .refreshing(["mock".to_string()])
        .resolve_component("target:mock")
        .await
        .expect("the refresh heals the poisoned entry");
    assert_eq!(bytes, expected);
}

#[tokio::test]
async fn refresh_offline_fails() {
    // An explicit update means checking the registry; offline, that
    // check fails deterministically instead of silently keeping the
    // local version.
    let sandbox = Sandbox::new();
    sandbox.seed_store_adapter("mock", "1.0.0");

    let err = sandbox
        .offline_resolver()
        .refreshing(["mock".to_string()])
        .resolve_component("target:mock")
        .await
        .expect_err("an offline refresh fails deterministically");
    assert_eq!(code(&err), "adapter-latest-failed");
}

#[tokio::test]
async fn refresh_keeps_cache_seed() {
    // The cache seed wins even under refresh — and no registry call
    // happens (the offline base would fail one).
    let sandbox = Sandbox::new();
    let seeded = sandbox.seed_cached_component("mock");

    let bytes = sandbox
        .offline_resolver()
        .refreshing(["mock".to_string()])
        .resolve_component("target:mock")
        .await
        .expect("the seed resolves without any registry consultation");
    assert_eq!(bytes, std::fs::read(&seeded).expect("read seed"));
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
fn mcp_route_declines_others() {
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
        // `engine` is not a legal adapter axis; the engine guest
        // serves no MCP shelf (the slice synthesis shelf is deleted).
        "/mcp/engine",
        "/mcp/engine/",
        "/mcp/engine/synthesis",
    ] {
        assert!(launcher::mcp_route(path).is_none(), "{path} must decline");
    }
}

#[tokio::test]
async fn non_wasm_artifact_refused() {
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
