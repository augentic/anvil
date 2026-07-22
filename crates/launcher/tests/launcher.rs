//! Launcher integration coverage over the public [`launcher::Policy`]
//! assembly and the guest resolver's typed kernel: argv-anchored
//! mounts (including the `adapter add` seed preopen), adapter
//! verify-and-load, and fail-closed store verification.
//!
//! No download path exists anywhere in this crate: the engine guest is
//! embedded in the binary and every adapter identity is hydrated by
//! the engine guest's own ensure legs — the resolver only verifies and
//! loads. Every test injects explicit [`Locations`] rooted in a
//! tempdir through `Policy::new` — the same explicit-layout seam
//! sandboxes use — so no process environment is read or mutated.

use std::path::PathBuf;

use launcher::{Policy, Resolver};
use project::handler::{CachePlacement, Locations};

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
    /// `specify adapter add` leaves behind).
    fn seed_cached_component(&self, name: &str) -> PathBuf {
        let components = project::handler::ExecutionPaths::new(&self.root, self.locations.clone())
            .cache_dir()
            .join("components");
        std::fs::create_dir_all(&components).expect("mkdir component cache");
        let path = components.join(format!("{name}.wasm"));
        std::fs::write(&path, format!("{name} cached component")).expect("write cached component");
        path
    }

    /// Install a pinned adapter into the sandbox store with a valid
    /// digest sidecar — the state the engine guest's ensure legs leave
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
    /// it.
    fn resolver(&self) -> Resolver {
        self.policy(&["plan", "status"]).resolver()
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
    assert_eq!(policy.store_dir(), sandbox.store);
    // The writable mount directories are created pre-run so the
    // guest's preopens exist.
    assert!(policy.cache_dir().is_dir());
    assert!(policy.store_dir().is_dir());
    // No seed in argv: the seed preopen degenerates to a harmless
    // read-only duplicate of the project mount.
    assert_eq!(policy.seed_dir(), sandbox.root);
    assert_eq!(policy.seed_mount_name(), sandbox.root.display().to_string());
}

#[test]
fn anchors_at_the_project_root_ancestor() {
    let sandbox = Sandbox::new();
    let specify = sandbox.root.join(".specify");
    std::fs::create_dir_all(&specify).expect("mkdir .specify");
    std::fs::write(
        specify.join("project.yaml"),
        format!(
            "name: fixture\nadapter: mock\nspecify: {}\nrules: {{}}\n",
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
// Adapter legs: verify-and-load only — no download path exists.

#[test]
fn store_adapter_verify_and_load() {
    let sandbox = Sandbox::new();
    let expected = sandbox.seed_store_adapter("mock", "1.0.0");

    let bytes = sandbox
        .resolver()
        .resolve_component("target:mock@1.0.0")
        .expect("store adapter resolves offline");
    assert_eq!(bytes, expected);
}

#[test]
fn adapter_store_miss_is_a_hard_failure() {
    // The adapters are hydrated by the engine guest's ensure legs
    // before any dispatch can miss; a resolve-time miss is the
    // fail-closed backstop, never a second download path — even for a
    // pinned identity a registry could serve.
    let sandbox = Sandbox::new();
    let err = sandbox
        .resolver()
        .resolve_component("target:mock@9.9.9")
        .expect_err("unprovisioned pin fails deterministically");
    assert_eq!(code(&err), "adapter-not-found");
}

#[test]
fn simultaneous_versions_resolve_distinctly() {
    let sandbox = Sandbox::new();
    let one = sandbox.seed_store_adapter("mock", "1.0.0");
    let two = sandbox.seed_store_adapter("mock", "2.0.0");
    assert_ne!(one, two);

    let resolver = sandbox.resolver();
    assert_eq!(resolver.resolve_component("target:mock@1.0.0").expect("v1"), one);
    assert_eq!(resolver.resolve_component("target:mock@2.0.0").expect("v2"), two);
}

#[test]
fn cache_backed_ids_resolve_the_project_cache() {
    let sandbox = Sandbox::new();
    let target = sandbox.seed_cached_component("mock");
    let source = sandbox.seed_cached_component("mock-source");

    let resolver = sandbox.resolver();
    assert_eq!(
        resolver.resolve_component("target:mock").expect("cached target"),
        std::fs::read(&target).expect("read cached target")
    );
    assert_eq!(
        resolver.resolve_component("source:mock-source").expect("cached source"),
        std::fs::read(&source).expect("read cached source")
    );
}

#[test]
fn cache_miss_is_a_hard_failure() {
    let sandbox = Sandbox::new();
    let err = sandbox
        .resolver()
        .resolve_component("target:mock")
        .expect_err("empty cache fails deterministically");
    assert_eq!(code(&err), "adapter-not-found");
}

#[test]
fn adapter_missing_sidecar_is_refused() {
    let sandbox = Sandbox::new();
    std::fs::write(sandbox.store.join("mock@1.0.0.wasm"), b"mock without provenance")
        .expect("write unverifiable adapter entry");

    let err = sandbox
        .resolver()
        .resolve_component("target:mock@1.0.0")
        .expect_err("sidecar-less store entry");
    assert_eq!(code(&err), "adapter-sidecar-missing");
}

#[test]
fn adapter_digest_drift_is_refused() {
    let sandbox = Sandbox::new();
    sandbox.seed_store_adapter("mock", "1.0.0");
    std::fs::write(sandbox.store.join("mock@1.0.0.wasm"), b"tampered adapter bytes")
        .expect("tamper with store entry");

    let err = sandbox
        .resolver()
        .resolve_component("target:mock@1.0.0")
        .expect_err("tampered store entry");
    assert_eq!(code(&err), "adapter-digest-mismatch");
}

#[test]
fn engine_identities_are_not_resolvable() {
    // The engine guest is embedded and registered statically at boot;
    // its package identity never reaches the resolver, and asking for
    // it fails like any other unprovisioned identity.
    let sandbox = Sandbox::new();
    let err = sandbox
        .resolver()
        .resolve_component(&format!("specify:engine@{}", env!("CARGO_PKG_VERSION")))
        .expect_err("no engine leg exists");
    assert_eq!(code(&err), "adapter-routed-id-malformed");
}

#[test]
fn malformed_ids_fail_deterministically() {
    let sandbox = Sandbox::new();
    sandbox.seed_cached_component("mock");
    let resolver = sandbox.resolver();

    for malformed in ["mock", "widget:mock", "target:", "target:mock@not-semver"] {
        let err = resolver.resolve_component(malformed).expect_err("outside the routed grammar");
        assert_eq!(code(&err), "adapter-routed-id-malformed", "{malformed}");
    }
}
