//! Workspace-slot binding reach under ID-only guest resolution: the
//! routed adapter id carries no slot, so `cache_staleness` flags a
//! slot binding that resolves slot-locally but not at the deployment
//! root (`workspace-slot-binding-unresolvable`). Exact package pins
//! are exempt — the host resolver installs a store miss during
//! dispatch (pull-on-miss).

use std::path::PathBuf;

use error::Error;
use project::adapter::{
    AdapterSelector, Origin, ResolvedSource, ResolvedTarget, Resolver, TargetAdapter,
};
use project::config::Layout;
use project::handler::{CachePlacement, ExecutionPaths, Locations};
use project::registry::Registry;
use project::registry::topology::cache_staleness;

const UNRESOLVABLE: &str = "workspace-slot-binding-unresolvable";

/// A component-deployment-shaped resolver: a package pin resolves as a
/// store identity; a bare or component selector resolves only when the
/// probe root's component cache holds the name — the path-dependence
/// that makes a slot-local cache entry invisible at the deployment
/// root.
struct CacheResolver;

impl Resolver for CacheResolver {
    fn resolve_source(
        &self, _selector: &AdapterSelector, _paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        unreachable!("topology resolves targets only")
    }

    fn resolve_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        let name = selector.name()?;
        let (version, reference) = if let AdapterSelector::Package { version, .. } = selector {
            (Some(version.clone()), format!("store:{name}@{version}"))
        } else {
            let entry = paths.cache_dir().join("components").join(format!("{name}.wasm"));
            if !entry.is_file() {
                return Err(Error::Diag {
                    code: "adapter-not-found",
                    detail: format!("no component cache entry at {}", entry.display()),
                });
            }
            (None, entry.display().to_string())
        };
        Ok(ResolvedTarget {
            manifest: TargetAdapter {
                name,
                version,
                requires_specify: None,
                inputs: Vec::new(),
                platforms: None,
            },
            origin: Origin {
                label: "cache".to_string(),
                reference,
            },
        })
    }
}

/// One workspace sandbox: registry root, one materialised slot, and
/// explicit locations per tree so slot and deployment caches diverge.
struct Workspace {
    root: PathBuf,
    locations: Locations,
    _tmp: tempfile::TempDir,
}

impl Workspace {
    fn new(slot_adapter: &str) -> Self {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let base = tmp.path().canonicalize().expect("canonical tempdir");
        let root = base.join("platform");
        let specify = root.join(".specify");
        std::fs::create_dir_all(&specify).expect("mkdir .specify");
        std::fs::write(
            specify.join("project.yaml"),
            format!(
                "name: platform\nspecify: {}\nrules: {{}}\nworkspace: true\n",
                env!("CARGO_PKG_VERSION")
            ),
        )
        .expect("write project.yaml");
        std::fs::write(
            root.join("registry.yaml"),
            "version: 1\nprojects:\n  - name: billing\n    url: .\n",
        )
        .expect("write registry.yaml");

        let slot = root.join("workspace").join("billing").join(".specify");
        std::fs::create_dir_all(&slot).expect("mkdir slot .specify");
        std::fs::write(
            slot.join("project.yaml"),
            format!(
                "name: billing\nadapter: {slot_adapter}\nspecify: {}\nrules: {{}}\n",
                env!("CARGO_PKG_VERSION")
            ),
        )
        .expect("write slot project.yaml");

        let locations =
            Locations::explicit(base.join("store"), CachePlacement::Parent(base.join("cache")));
        Self {
            root,
            locations,
            _tmp: tmp,
        }
    }

    fn paths(&self) -> ExecutionPaths {
        ExecutionPaths::new(&self.root, self.locations.clone())
    }

    /// Seed a cache component under `paths`'s component cache.
    fn seed_cache(paths: &ExecutionPaths, name: &str) {
        let components = paths.cache_dir().join("components");
        std::fs::create_dir_all(&components).expect("mkdir components");
        std::fs::write(components.join(format!("{name}.wasm")), b"component bytes")
            .expect("write component");
    }

    fn staleness(&self) -> Vec<diagnostics::Diagnostic> {
        let paths = self.paths();
        let registry = Registry::load(&self.root).expect("load registry").expect("registry.yaml");
        cache_staleness(
            &CacheResolver,
            &registry,
            &paths,
            &Layout::new(&self.root).topology_lock_path(),
        )
    }
}

fn codes(diagnostics: &[diagnostics::Diagnostic]) -> Vec<&str> {
    diagnostics.iter().filter_map(|d| d.rule_id.as_deref()).collect()
}

// A slot-local cache-only binding derives topology fine (the slot's
// own cache resolves it) but cannot dispatch by routed id — flagged.
#[test]
fn slot_local_cache_binding_is_flagged() {
    let workspace = Workspace::new("mock");
    let slot_paths = workspace.paths().with_root(workspace.root.join("workspace/billing"));
    Workspace::seed_cache(&slot_paths, "mock");

    let diagnostics = workspace.staleness();
    assert!(codes(&diagnostics).contains(&UNRESOLVABLE), "{diagnostics:?}");
    let unresolvable = diagnostics
        .iter()
        .find(|d| d.rule_id.as_deref() == Some(UNRESOLVABLE))
        .expect("unresolvable finding");
    // The finding names the binding and both supported forms.
    assert!(unresolvable.title.contains("`mock`"), "{}", unresolvable.title);
    assert!(unresolvable.title.contains("specify:mock@<semver>"), "{}", unresolvable.title);
    assert!(unresolvable.title.contains("adapter add"), "{}", unresolvable.title);
}

// A binding seeded into the deployment project's cache dispatches by
// its unversioned routed id — not flagged.
#[test]
fn deployment_cache_binding_is_resolvable() {
    let workspace = Workspace::new("mock");
    let slot_paths = workspace.paths().with_root(workspace.root.join("workspace/billing"));
    Workspace::seed_cache(&slot_paths, "mock");
    Workspace::seed_cache(&workspace.paths(), "mock");

    let diagnostics = workspace.staleness();
    assert!(!codes(&diagnostics).contains(&UNRESOLVABLE), "{diagnostics:?}");
}

// An exact package pin installs by id (the host resolver pulls a
// store miss during dispatch) — exempt.
#[test]
fn package_pin_binding_is_exempt() {
    let workspace = Workspace::new("specify:mock@1.0.0");
    let diagnostics = workspace.staleness();
    assert!(!codes(&diagnostics).contains(&UNRESOLVABLE), "{diagnostics:?}");
}
