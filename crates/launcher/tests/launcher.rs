//! Launcher integration coverage over the public `prepare_with`
//! surface: closure derivation, hydration with a scripted registry
//! fetch, fail-closed store verification, and typed deployment
//! assembly.
//!
//! Every test injects explicit [`Locations`] rooted in a tempdir
//! through `prepare_with` — the same explicit-layout seam sandboxes
//! use — so no process environment is read or mutated.

use std::path::{Path, PathBuf};

use launcher::{Deployment, Outcome};
use project::handler::{CachePlacement, Locations};

/// The engine version the tests inject — the composition root's pin.
const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The injected engine identity — the binary's version pin.
const ENGINE: launcher::Engine = launcher::Engine { version: ENGINE_VERSION };

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

    /// Initialised project fixture: `.specify/project.yaml` bound to
    /// the bare target `adapter`.
    fn with_project(adapter: &str) -> Self {
        let sandbox = Self::new();
        let specify = sandbox.root.join(".specify");
        std::fs::create_dir_all(&specify).expect("mkdir .specify");
        std::fs::write(
            specify.join("project.yaml"),
            format!(
                "name: launcher-fixture\nadapter: {adapter}\nspecify: {ENGINE_VERSION}\nrules: \
                 {{}}\n"
            ),
        )
        .expect("write project.yaml");
        sandbox
    }

    /// Install the engine into the sandbox store with a valid digest
    /// sidecar — the state a registry hydration leaves behind.
    fn seed_engine(&self) {
        let entry = self.store.join(format!("engine@{ENGINE_VERSION}.wasm"));
        std::fs::write(&entry, b"engine component bytes").expect("write engine entry");
        let digest = diagnostics::cache::file_content_digest(&entry);
        let meta = self.store.join(format!("engine@{ENGINE_VERSION}.meta"));
        diagnostics::cache::write_store_meta(&meta, &digest, None).expect("write engine sidecar");
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
    /// digest sidecar — the state a registry hydration leaves behind.
    fn seed_store_adapter(&self, name: &str, version: &str) -> PathBuf {
        let entry = self.store.join(format!("{name}@{version}.wasm"));
        std::fs::write(&entry, format!("{name} {version} store bytes")).expect("write store entry");
        let digest = diagnostics::cache::file_content_digest(&entry);
        let meta = self.store.join(format!("{name}@{version}.meta"));
        diagnostics::cache::write_store_meta(&meta, &digest, None).expect("write store sidecar");
        entry
    }

    /// Workspace fixture: `workspace: true` project.yaml plus a
    /// `registry.yaml` naming `members` (slots materialise separately
    /// via [`Self::materialise_slot`]).
    fn with_workspace(members: &[&str]) -> Self {
        let sandbox = Self::new();
        let specify = sandbox.root.join(".specify");
        std::fs::create_dir_all(&specify).expect("mkdir .specify");
        std::fs::write(
            specify.join("project.yaml"),
            format!("name: platform\nspecify: {ENGINE_VERSION}\nrules: {{}}\nworkspace: true\n"),
        )
        .expect("write project.yaml");
        let projects = members.iter().fold(String::new(), |mut acc, name| {
            use std::fmt::Write as _;
            writeln!(acc, "  - name: {name}\n    url: .").expect("write registry entry");
            acc
        });
        std::fs::write(
            sandbox.root.join("registry.yaml"),
            format!("version: 1\nprojects:\n{projects}"),
        )
        .expect("write registry.yaml");
        sandbox
    }

    /// Materialise one workspace slot: `workspace/<name>/.specify/
    /// project.yaml` bound to `adapter`.
    fn materialise_slot(&self, name: &str, adapter: &str) {
        let specify = self.root.join("workspace").join(name).join(".specify");
        std::fs::create_dir_all(&specify).expect("mkdir slot .specify");
        std::fs::write(
            specify.join("project.yaml"),
            format!("name: {name}\nadapter: {adapter}\nspecify: {ENGINE_VERSION}\nrules: {{}}\n"),
        )
        .expect("write slot project.yaml");
    }

    /// Minimal `plan.yaml` binding the bare source `mock-code`.
    fn write_plan(&self) {
        std::fs::write(
            self.root.join("plan.yaml"),
            "name: demo\nsources:\n  main:\n    adapter: mock-code\n    value: The brief.\n\
             slices: []\n",
        )
        .expect("write plan.yaml");
    }

    fn prepare(&self, args: &[&str]) -> Outcome {
        launcher::prepare_with(&self.root, &argv(args), ENGINE, self.locations.clone(), refuse)
    }
}

fn argv(args: &[&str]) -> Vec<String> {
    args.iter().map(ToString::to_string).collect()
}

/// Registry transport that refuses every fetch — the tests' default:
/// anything already provisioned must resolve without the network.
async fn refuse(url: String) -> Result<Vec<u8>, error::Error> {
    Err(error::Error::Diag {
        code: "test-fetch-refused",
        detail: format!("unexpected registry fetch for {url}"),
    })
}

fn deployment(outcome: Outcome) -> Deployment {
    match outcome {
        Outcome::Run(deployment) => deployment,
        Outcome::Done { stderr, code, .. } => {
            panic!("expected Run, got exit {code}: {}", String::from_utf8_lossy(&stderr))
        }
    }
}

fn exit(outcome: Outcome) -> (String, u8) {
    match outcome {
        Outcome::Done { stderr, code, .. } => (String::from_utf8_lossy(&stderr).into_owned(), code),
        Outcome::Run(deployment) => panic!("expected Done, got Run: {deployment:?}"),
    }
}

/// A host-side success: exit 0 with the rendered stdout report.
fn done(outcome: Outcome) -> String {
    match outcome {
        Outcome::Done {
            stdout,
            stderr,
            code: 0,
        } => {
            assert!(stderr.is_empty(), "{}", String::from_utf8_lossy(&stderr));
            String::from_utf8_lossy(&stdout).into_owned()
        }
        Outcome::Done { stderr, code, .. } => {
            panic!("expected success, got exit {code}: {}", String::from_utf8_lossy(&stderr))
        }
        Outcome::Run(deployment) => panic!("expected Done, got Run: {deployment:?}"),
    }
}

#[test]
fn grammar_rejection_exits_2_before_hydration() {
    let sandbox = Sandbox::new();
    let (stderr, code) = exit(sandbox.prepare(&["frobnicate"]));
    assert_eq!(code, 2);
    assert!(stderr.contains("unrecognized subcommand"), "{stderr}");
    assert!(!sandbox.store.join(format!("engine@{ENGINE_VERSION}.wasm")).exists());
}

#[test]
fn engine_hydrates_from_registry() {
    let sandbox = Sandbox::new();
    let outcome = launcher::prepare_with(
        &sandbox.root,
        &argv(&["registry", "validate"]),
        ENGINE,
        sandbox.locations.clone(),
        |url| {
            assert_eq!(
                url,
                format!("https://augentic.io/adapters/specify/engine@{ENGINE_VERSION}.wasm")
            );
            async { Ok(b"published engine bytes".to_vec()) }
        },
    );

    let deployment = deployment(outcome);
    let entry = sandbox.store.join(format!("engine@{ENGINE_VERSION}.wasm"));
    assert_eq!(deployment.engine.id, "specify");
    assert_eq!(deployment.engine.component, entry);
    assert_eq!(
        deployment.engine.links,
        ["specify:adapter/source@0.1.0", "specify:adapter/target@0.1.0"]
    );
    assert!(deployment.adapters.is_empty());
    assert!(entry.is_file());
    assert!(sandbox.store.join(format!("engine@{ENGINE_VERSION}.meta")).is_file());

    // The install is durable: the next invocation resolves the store
    // entry without touching the registry.
    let again = deployment_mounts(&sandbox);
    assert_eq!(again.engine.component, entry);
}

/// A store-installed engine resolves offline and the deployment grants
/// exactly the three well-known mounts.
fn deployment_mounts(sandbox: &Sandbox) -> Deployment {
    deployment(sandbox.prepare(&["registry", "validate"]))
}

#[test]
fn mounts_are_the_three_well_known_locations() {
    let sandbox = Sandbox::new();
    sandbox.seed_engine();
    let deployment = deployment_mounts(&sandbox);

    let names: Vec<&str> = deployment.mounts.iter().map(|mount| mount.name.as_str()).collect();
    assert_eq!(names, [".", "/specify-cache", "/specify-store"]);
    assert!(deployment.mounts.iter().all(|mount| mount.writable));
    assert_eq!(deployment.mounts[0].path, sandbox.root);
    assert_eq!(deployment.mounts[2].path, sandbox.store);
    // The cache mount is created pre-run so the guest's preopen exists.
    assert!(deployment.mounts[1].path.is_dir());
}

#[test]
fn hydration_failure_fails_closed() {
    let sandbox = Sandbox::new();
    let (stderr, code) = exit(sandbox.prepare(&["registry", "validate"]));
    assert_eq!(code, 1);
    assert!(stderr.contains("adapter-hydrate-failed"), "{stderr}");
}

#[test]
fn missing_sidecar_is_refused() {
    let sandbox = Sandbox::new();
    std::fs::write(
        sandbox.store.join(format!("engine@{ENGINE_VERSION}.wasm")),
        b"engine without provenance",
    )
    .expect("write unverifiable engine entry");

    let (stderr, code) = exit(sandbox.prepare(&["registry", "validate"]));
    assert_eq!(code, 1);
    assert!(stderr.contains("adapter-sidecar-missing"), "{stderr}");
}

/// The fail-closed sidecar gate covers store-resolved adapters too,
/// not just the engine entry: `locate`'s verify-on-read is fail-open
/// for legacy sidecar-less entries, so hydration adds the refusal.
#[test]
fn store_adapter_missing_sidecar_is_refused() {
    let sandbox = Sandbox::with_project("specify:mock@1.0.0");
    sandbox.seed_engine();
    std::fs::write(sandbox.store.join("mock@1.0.0.wasm"), b"mock without provenance")
        .expect("write unverifiable adapter entry");

    let (stderr, code) = exit(sandbox.prepare(&["slice", "build", "s1"]));
    assert_eq!(code, 1);
    assert!(stderr.contains("adapter-sidecar-missing"), "{stderr}");
}

#[test]
fn digest_drift_is_refused() {
    let sandbox = Sandbox::new();
    sandbox.seed_engine();
    std::fs::write(
        sandbox.store.join(format!("engine@{ENGINE_VERSION}.wasm")),
        b"tampered engine bytes",
    )
    .expect("tamper with engine entry");

    let (stderr, code) = exit(sandbox.prepare(&["registry", "validate"]));
    assert_eq!(code, 1);
    assert!(stderr.contains("adapter-digest-mismatch"), "{stderr}");
}

#[test]
fn failure_envelope_honours_format_json() {
    let sandbox = Sandbox::new();
    let (stderr, code) = exit(sandbox.prepare(&["--format", "json", "registry", "validate"]));
    assert_eq!(code, 1);
    let body: serde_json::Value = serde_json::from_str(&stderr).expect("JSON failure envelope");
    assert_eq!(body["error"], "adapter-hydrate-failed");
    assert_eq!(body["exit-code"], 1);
}

#[test]
fn project_target_joins_closure() {
    let sandbox = Sandbox::with_project("mock");
    sandbox.seed_engine();
    let cached = sandbox.seed_cached_component("mock");

    let deployment = deployment(sandbox.prepare(&["slice", "build", "s1"]));
    let ids: Vec<&str> = deployment.adapters.iter().map(|guest| guest.id.as_str()).collect();
    assert_eq!(ids, ["target:mock"]);
    assert_eq!(deployment.adapters[0].component, cached);
    assert!(deployment.adapters[0].links.is_empty());
}

#[test]
fn plan_sources_join_closure() {
    let sandbox = Sandbox::with_project("mock");
    sandbox.seed_engine();
    let target_cached = sandbox.seed_cached_component("mock");
    let source_cached = sandbox.seed_cached_component("mock-code");
    sandbox.write_plan();

    // `slice refine` extracts per bound source and its synthesis reads
    // the target's guidance, so both legs join.
    let deployment = deployment(sandbox.prepare(&["slice", "refine", "s1"]));
    let mut guests: Vec<(&str, &Path)> = deployment
        .adapters
        .iter()
        .map(|guest| (guest.id.as_str(), guest.component.as_path()))
        .collect();
    guests.sort_unstable();
    assert_eq!(
        guests,
        [("source:mock-code", source_cached.as_path()), ("target:mock", target_cached.as_path())]
    );
}

/// Read-only verbs derive an engine-only closure: no state-derived
/// adapter joins, nothing needs staging, and — with the refusing
/// transport — no registry fetch happens even though the project binds
/// a target and the plan binds a source that were never provisioned.
#[test]
fn read_only_verbs_deploy_the_engine_alone() {
    let sandbox = Sandbox::with_project("mock");
    sandbox.seed_engine();
    sandbox.write_plan();

    for verb in [&["plan", "status"][..], &["journal", "show"][..], &["slice", "list"][..]] {
        let deployment = deployment(sandbox.prepare(verb));
        assert_eq!(deployment.engine.id, "specify", "{verb:?}");
        assert!(deployment.adapters.is_empty(), "{verb:?}: {:?}", deployment.adapters);
    }
}

#[test]
fn argv_component_selector_mirrors_into_cache() {
    let sandbox = Sandbox::new();
    sandbox.seed_engine();
    std::fs::write(sandbox.root.join("mock.wasm"), b"operator-supplied component")
        .expect("write local component");

    let deployment = deployment(sandbox.prepare(&["init", "./mock.wasm"]));
    let ids: Vec<&str> = deployment.adapters.iter().map(|guest| guest.id.as_str()).collect();
    assert_eq!(ids, ["target:mock"]);
    let mirrored = &deployment.adapters[0].component;
    assert!(mirrored.ends_with("components/mock.wasm"), "{}", mirrored.display());
    assert!(mirrored.is_file());
}

#[test]
fn adapter_add_completes_host_side() {
    // The operator's component may live anywhere on the host —
    // outside the guest's mounts — and the copy is deterministic
    // engine-free work, so the launcher seeds and reports without
    // starting the runtime (the untouched store proves no deployment
    // was assembled).
    let sandbox = Sandbox::new();
    let elsewhere = sandbox.root.parent().expect("sandbox base").join("built/demo.wasm");
    std::fs::create_dir_all(elsewhere.parent().expect("parent")).expect("mkdir build dir");
    std::fs::write(&elsewhere, b"freshly built component").expect("write built component");

    let stdout = done(sandbox.prepare(&["adapter", "add", &elsewhere.display().to_string()]));
    assert!(stdout.contains("Seeded `demo`"), "{stdout}");
    let entry = project::handler::ExecutionPaths::new(&sandbox.root, sandbox.locations.clone())
        .cache_dir()
        .join("components/demo.wasm");
    assert!(entry.is_file(), "seeded without a runtime");
    assert_eq!(std::fs::read(&entry).expect("read entry"), b"freshly built component");
    assert!(
        !sandbox.store.join(format!("engine@{ENGINE_VERSION}.wasm")).exists(),
        "no deployment was assembled"
    );
}

#[test]
fn adapter_add_missing_component_fails_closed() {
    let sandbox = Sandbox::new();
    let (stderr, code) = exit(sandbox.prepare(&["adapter", "add", "./no-such.wasm"]));
    assert_eq!(code, 1);
    assert!(stderr.contains("adapter-component-missing"), "{stderr}");
}

/// A stale cache entry must not mask a typo: re-adding a name whose
/// operator file no longer exists fails instead of reporting the old
/// entry (the persisted-mirror fallback belongs to the component
/// selector's re-ensure, not to the explicit seed verb).
#[test]
fn adapter_add_stale_path_with_cached_name_fails() {
    let sandbox = Sandbox::new();
    sandbox.seed_cached_component("demo");
    let (stderr, code) = exit(sandbox.prepare(&["adapter", "add", "./demo.wasm"]));
    assert_eq!(code, 1);
    assert!(stderr.contains("adapter-component-missing"), "{stderr}");
}

#[test]
fn help_renders_host_side() {
    // Displays never assemble a deployment: the shared grammar renders
    // them, so no hydration (the refusing transport plus the untouched
    // store prove it).
    let sandbox = Sandbox::new();
    for display in [&["--help"][..], &["plan", "--help"][..], &["--version"][..]] {
        let stdout = done(sandbox.prepare(display));
        assert!(!stdout.is_empty(), "{display:?}");
    }
    assert!(!sandbox.store.join(format!("engine@{ENGINE_VERSION}.wasm")).exists());
}

#[test]
fn duplicate_requirements_collapse() {
    let sandbox = Sandbox::with_project("mock");
    sandbox.seed_engine();
    sandbox.seed_cached_component("mock");

    // `init mock` joins both the argv selector and (for `--upgrade`
    // re-entry) the recorded project adapter — the same identity once.
    let deployment = deployment(sandbox.prepare(&["init", "mock"]));
    let ids: Vec<&str> = deployment.adapters.iter().map(|guest| guest.id.as_str()).collect();
    assert_eq!(ids, ["target:mock"]);
}

#[test]
fn conflicting_components_fail_closed() {
    // Genuinely distinct paths for one identity: the recorded project
    // adapter is a pinned store install while argv supplies a local
    // component that mirrors into the project cache — same name, two
    // component files.
    let sandbox = Sandbox::with_project("specify:mock@1.0.0");
    sandbox.seed_engine();
    sandbox.seed_store_adapter("mock", "1.0.0");
    std::fs::write(sandbox.root.join("mock.wasm"), b"a different mock component")
        .expect("write local component");

    let (stderr, code) = exit(sandbox.prepare(&["init", "./mock.wasm"]));
    assert_eq!(code, 1);
    assert!(stderr.contains("deployment-adapter-conflict"), "{stderr}");
}

#[test]
fn slot_target_joins_plan_validate_closure() {
    let sandbox = Sandbox::with_workspace(&["billing"]);
    sandbox.seed_engine();
    let entry = sandbox.seed_store_adapter("mock", "1.0.0");
    sandbox.materialise_slot("billing", "specify:mock@1.0.0");

    let deployment = deployment(sandbox.prepare(&["plan", "validate"]));
    let ids: Vec<&str> = deployment.adapters.iter().map(|guest| guest.id.as_str()).collect();
    assert_eq!(ids, ["target:mock"]);
    assert_eq!(deployment.adapters[0].component, entry);
}

/// The derived `.specify/topology.lock` is never a closure input: a
/// stale lock naming `mock@1.0.0` must not shadow the slot's own
/// `project.yaml` pin — the guest re-derives topology from the slot
/// config, so the launcher enumerates the same fresh identity.
#[test]
fn stale_topology_lock_is_ignored() {
    let sandbox = Sandbox::with_workspace(&["billing"]);
    sandbox.seed_engine();
    let fresh = sandbox.seed_store_adapter("mock", "2.0.0");
    sandbox.materialise_slot("billing", "specify:mock@2.0.0");
    std::fs::write(
        sandbox.root.join(".specify/topology.lock"),
        "version: 1\nprojects:\n  - name: billing\n    target: mock@1.0.0\n",
    )
    .expect("write stale topology.lock");

    // `mock@1.0.0` is neither store-installed nor fetchable (the
    // refusing transport), so consulting the lock would fail the
    // launch; the fresh pin resolves offline.
    let deployment = deployment(sandbox.prepare(&["plan", "validate"]));
    let ids: Vec<&str> = deployment.adapters.iter().map(|guest| guest.id.as_str()).collect();
    assert_eq!(ids, ["target:mock"]);
    assert_eq!(deployment.adapters[0].component, fresh);
}

#[test]
fn unmaterialised_slot_contributes_nothing() {
    let sandbox = Sandbox::with_workspace(&["billing"]);
    sandbox.seed_engine();

    let deployment = deployment(sandbox.prepare(&["plan", "validate"]));
    assert!(deployment.adapters.is_empty(), "{:?}", deployment.adapters);
}

/// A materialised slot bound to a bare name with an empty slot cache
/// has no resolvable artifact; the guest degrades that slot to a
/// `workspace-slot-config-unreadable` finding without dispatching it,
/// so the launcher skips it rather than failing the launch.
#[test]
fn unresolvable_slot_binding_is_skipped() {
    let sandbox = Sandbox::with_workspace(&["billing"]);
    sandbox.seed_engine();
    sandbox.materialise_slot("billing", "mock");

    let deployment = deployment(sandbox.prepare(&["plan", "validate"]));
    assert!(deployment.adapters.is_empty(), "{:?}", deployment.adapters);
}

#[test]
fn anchors_at_the_project_root_ancestor() {
    let sandbox = Sandbox::with_project("mock");
    sandbox.seed_engine();
    sandbox.seed_cached_component("mock");
    let nested = sandbox.root.join("src/deeply/nested");
    std::fs::create_dir_all(&nested).expect("mkdir nested dir");

    let deployment = deployment(launcher::prepare_with(
        &nested,
        &argv(&["slice", "build", "s1"]),
        ENGINE,
        sandbox.locations.clone(),
        refuse,
    ));
    assert_eq!(deployment.mounts[0].path, sandbox.root);
    let ids: Vec<&str> = deployment.adapters.iter().map(|guest| guest.id.as_str()).collect();
    assert_eq!(ids, ["target:mock"]);
}
