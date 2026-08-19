//! Launcher integration coverage over the public
//! [`emery::launcher::assemble`] seam and the guest resolver's typed
//! kernel: anchored mounts, local-only adapter resolution, and
//! fail-closed store verification.
//!
//! There is no download path (ADR-0002 deletions): resolution is the
//! project cache seed, else the embedded first-party registry (empty
//! until Phase 4), else a verified global-store entry — nothing local
//! is a typed miss. Every test injects explicit [`Locations`] rooted
//! in a tempdir through `launcher::assemble`, so no process
//! environment is read or mutated.

use std::path::PathBuf;

use emery::launcher::{self, Resolver};
use engine::handler::{CachePlacement, ExecutionPaths, Locations};

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
    /// `emery init <path/to/name.wasm>` leaves behind).
    fn seed_cached_component(&self, name: &str) -> PathBuf {
        let components =
            ExecutionPaths::new(&self.root, self.locations.clone()).cache_dir().join("components");
        std::fs::create_dir_all(&components).expect("mkdir component cache");
        let path = components.join(format!("{name}.wasm"));
        std::fs::write(&path, format!("{name} cached component")).expect("write cached component");
        path
    }

    /// Install a pinned adapter into the sandbox store with a valid
    /// digest sidecar — the state an explicit install leaves behind.
    fn seed_store_adapter(&self, name: &str, version: &str) -> Vec<u8> {
        let bytes = format!("{name} {version} store bytes").into_bytes();
        let entry = self.store.join(format!("{name}@{version}.wasm"));
        std::fs::write(&entry, &bytes).expect("write store entry");
        let digest = diagnostics::cache::file_content_digest(&entry);
        let meta = self.store.join(format!("{name}@{version}.meta"));
        diagnostics::cache::write_store_meta(&meta, &digest, None).expect("write store sidecar");
        bytes
    }

    fn paths(&self) -> ExecutionPaths {
        launcher::assemble(&self.root, self.locations.clone())
    }

    /// The deployment's resolver, assembled the way the binary does it.
    fn resolver(&self) -> Resolver {
        Resolver::new(self.paths())
    }
}

fn code(err: &error::Error) -> &str {
    match err {
        error::Error::Diag { code, .. } => code,
        other => panic!("expected a Diag error, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Deployment policy: the mounts anchor from the working directory;
// the writable mount directories are created pre-run.

#[test]
fn mounts_are_well_known() {
    let sandbox = Sandbox::new();
    let paths = sandbox.paths();

    assert_eq!(paths.project_root(), sandbox.root);
    // The writable mount directories are created pre-run so the
    // guest's preopens exist. The global store gets no guest mount —
    // it is host-owned.
    assert!(paths.cache_dir().is_dir());
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

    let paths = launcher::assemble(&nested, sandbox.locations.clone());
    assert_eq!(paths.project_root(), sandbox.root);
}

#[test]
fn unanchored_cwd_in_place() {
    // No `project.yaml` ancestor: assembly stays total — it boots
    // in-place at the cwd (pre-init) so `emery init` works and later
    // verbs fail typed in-guest.
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.paths().project_root(), sandbox.root);
}

// ---------------------------------------------------------------------------
// Adapter legs: cache seeds and verified store entries resolve
// offline; everything else is a typed miss — no download path.

#[test]
fn store_adapter_verify_load() {
    let sandbox = Sandbox::new();
    let expected = sandbox.seed_store_adapter("mock", "1.0.0");

    let bytes = sandbox
        .resolver()
        .resolve_component("source:mock@1.0.0")
        .expect("store adapter resolves offline");
    assert_eq!(bytes, expected);
}

#[test]
fn versions_resolve() {
    let sandbox = Sandbox::new();
    let one = sandbox.seed_store_adapter("mock", "1.0.0");
    let two = sandbox.seed_store_adapter("mock", "2.0.0");
    assert_ne!(one, two);

    let resolver = sandbox.resolver();
    assert_eq!(resolver.resolve_component("source:mock@1.0.0").expect("v1"), one);
    assert_eq!(resolver.resolve_component("source:mock@2.0.0").expect("v2"), two);
}

#[test]
fn cache_backed_ids_resolve() {
    let sandbox = Sandbox::new();
    let seeded = sandbox.seed_cached_component("mock-source");

    let resolver = sandbox.resolver();
    assert_eq!(
        resolver.resolve_component("source:mock-source").expect("cached source"),
        std::fs::read(&seeded).expect("read cached source")
    );
}

#[test]
fn cold_pin_is_a_typed_miss() {
    // A pinned store miss is terminal: there is no install-on-miss
    // leg (ADR-0002 deletions), so the failure names the identity and
    // the local-seed recovery.
    let sandbox = Sandbox::new();
    let err = sandbox
        .resolver()
        .resolve_component("source:mock@9.9.9")
        .expect_err("a cold pinned miss fails typed");
    assert_eq!(code(&err), "adapter-not-found");
    let detail = err.to_string();
    assert!(detail.contains("mock@9.9.9"), "{detail}");
    assert!(detail.contains("emery init"), "local-seed recovery: {detail}");
}

#[test]
fn bare_miss_is_a_typed_miss() {
    // A bare id with nothing local (no cache seed, no embedded
    // first-party entry) is terminal — no pull-latest provisioning.
    let sandbox = Sandbox::new();
    let err = sandbox
        .resolver()
        .resolve_component("source:mock")
        .expect_err("a bare total miss fails typed");
    assert_eq!(code(&err), "adapter-not-found");
    let detail = err.to_string();
    assert!(detail.contains("emery init"), "local-seed recovery: {detail}");
}

#[test]
fn missing_sidecar_refused() {
    // An unverifiable entry is refused terminally — there is no
    // reinstall-in-place heal — and the local artifact survives.
    let sandbox = Sandbox::new();
    let entry = sandbox.store.join("mock@1.0.0.wasm");
    std::fs::write(&entry, b"mock without provenance").expect("write unverifiable adapter entry");

    let err = sandbox
        .resolver()
        .resolve_component("source:mock@1.0.0")
        .expect_err("sidecar-less store entry");
    assert_eq!(code(&err), "adapter-sidecar-missing");
    assert!(entry.is_file(), "the refusal must not unlink the local artifact");
}

#[test]
fn digest_drift_refused() {
    let sandbox = Sandbox::new();
    sandbox.seed_store_adapter("mock", "1.0.0");
    let entry = sandbox.store.join("mock@1.0.0.wasm");
    std::fs::write(&entry, b"tampered adapter bytes").expect("tamper with store entry");

    let err = sandbox
        .resolver()
        .resolve_component("source:mock@1.0.0")
        .expect_err("tampered store entry");
    assert_eq!(code(&err), "adapter-digest-mismatch");
    assert!(entry.is_file(), "the refusal must not unlink the local artifact");
    assert!(
        sandbox.store.join("mock@1.0.0.meta").is_file(),
        "the refusal must not unlink the sidecar"
    );
}

#[test]
fn cache_seed_shadows_store() {
    // The co-dev seed always wins: with a cache seed, a newer store
    // entry is ignored.
    let sandbox = Sandbox::new();
    let seeded = sandbox.seed_cached_component("mock");
    sandbox.seed_store_adapter("mock", "2.0.0");

    let bytes = sandbox.resolver().resolve_component("source:mock").expect("the cache seed wins");
    assert_eq!(bytes, std::fs::read(&seeded).expect("read seed"));
}

#[test]
fn cache_seed_answers_pins() {
    // The co-dev seed wins for a pinned identity too: a seeded local
    // build must not be shadowed by the store at dispatch.
    let sandbox = Sandbox::new();
    let seeded = sandbox.seed_cached_component("mock");
    sandbox.seed_store_adapter("mock", "1.0.0");

    let bytes = sandbox
        .resolver()
        .resolve_component("source:mock@1.0.0")
        .expect("the cache seed answers the pin");
    assert_eq!(bytes, std::fs::read(&seeded).expect("read seed"));
}

#[test]
fn engine_ids_not_resolvable() {
    // The engine guest is embedded and registered statically at boot;
    // its package identity never reaches the resolver, and asking for
    // it fails like any other identity outside the routed grammar.
    let sandbox = Sandbox::new();
    let err = sandbox
        .resolver()
        .resolve_component(&format!("emery:engine@{}", env!("CARGO_PKG_VERSION")))
        .expect_err("no engine leg exists");
    assert_eq!(code(&err), "adapter-routed-id-malformed");
}

#[test]
fn malformed_ids_fail() {
    let sandbox = Sandbox::new();
    sandbox.seed_cached_component("mock");
    let resolver = sandbox.resolver();

    for malformed in ["mock", "widget:mock", "source:", "source:mock@not-semver"] {
        let err = resolver.resolve_component(malformed).expect_err("outside the routed grammar");
        assert_eq!(code(&err), "adapter-routed-id-malformed", "{malformed}");
    }
}

// ---------------------------------------------------------------------------
// The MCP HTTP route: `/mcp/<axis>/<name>[@<version>]` maps back
// onto the routed adapter id the grant URL was derived from; anything
// outside the routed grammar is `None` (an ordinary 404).

#[test]
fn mcp_route_maps_routed_ids() {
    for (path, id) in [
        ("/mcp/source/typescript", "source:typescript"),
        ("/mcp/source/intent@1.2.3", "source:intent@1.2.3"),
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
        "/mcp/source",
        "/mcp/source/",
        "/mcp/plugin/omnia",
        "/mcp/source/intent@1",
        "/mcp/source/intent@not-semver",
        // `engine` is not a legal adapter axis; the engine guest
        // serves no MCP shelf (the slice synthesis shelf is deleted).
        "/mcp/engine",
        "/mcp/engine/",
        "/mcp/engine/synthesis",
    ] {
        assert!(launcher::mcp_route(path).is_none(), "{path} must decline");
    }
}
