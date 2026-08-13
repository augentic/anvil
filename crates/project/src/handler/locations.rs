//! [`Locations`] — well-known on-disk roots (adapter store, component
//! cache, snapshots, workspaces). Kernels and handlers receive the value
//! through [`super::ExecutionPaths`] and never read `std::env` themselves.

use std::path::{Path, PathBuf};

/// Guest-visible preopen name of the per-project derived cache inside
/// the engine guest's WASI sandbox.
///
/// The generated deployment manifest mounts the host's resolved
/// [`Locations::project_cache_dir`] under this name (guest routing:
/// the guest runs init's scaffold leg, which writes cache tenants);
/// the guest constructs its `Locations` over the preopen directly —
/// one project per deployment, so no project-id keying is needed
/// in-guest.
pub const GUEST_CACHE_MOUNT: &str = "/emery-cache";

/// Nominal store root inside the engine guest's layout.
///
/// The global adapter store is host-owned and gets **no** guest
/// mount: package pins dispatch by routed id, the host resolver
/// installs a missing pin during that dispatch (pull-on-miss), and
/// the guest never opens a store file. The constant survives as the
/// guest's nominal [`Locations::store_root`] — pure path math feeding
/// origin display, never I/O.
pub const GUEST_STORE_MOUNT: &str = "/emery-store";

/// Nominal snapshot-store root inside the engine guest's layout.
///
/// The content-addressed snapshot store is host-owned and gets **no**
/// guest mount: the guest drives `prepare` / `capture` / `discard`
/// through the workspace capability and never opens a snapshot
/// object. The constant is the guest's nominal
/// [`Locations::snapshots_root`] — pure path math, never I/O.
pub const GUEST_SNAPSHOTS_MOUNT: &str = "/emery-snapshots";

/// Guest-visible preopen name of the host's workspaces root inside
/// the deployment's WASI sandbox.
///
/// The deployment mounts the host's resolved
/// [`Locations::workspaces_root`] under this name so every guest
/// (engine and adapters) can open a prepared private workspace by its
/// deployment-local path.
pub const GUEST_WORKSPACES_MOUNT: &str = "/emery-workspaces";

/// Environment key carrying the host-absolute project root into the
/// engine guest.
///
/// Guests inherit the host environment; the in-guest kernel derives
/// the agent-visible artifact root from this key, so a spawned agent
/// whose working directory is a lent workspace can still read
/// change-tree artifacts.
pub const PROJECT_ROOT_ENV: &str = "EMERY_PROJECT_ROOT";

/// How the cache root carried by [`Locations`] is interpreted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachePlacement {
    /// Host parent holding every project's cache: the per-project
    /// directory is created beneath it, keyed by the canonical
    /// project-id digest.
    Parent(PathBuf),
    /// Already-resolved per-project cache root, such as the engine
    /// guest's preopen — one project per deployment, no keying.
    Project(PathBuf),
}

/// Well-known on-disk locations for resolvable artifacts.
///
/// Owns the production roots — the global adapter store (pinned
/// identities plus digest sidecars), the project component cache
/// (operator-seeded local components), the content-addressed snapshot
/// store, and the private-workspace root — and the layout formulas
/// over them. Methods are pure path math; the roots are fixed at
/// construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locations {
    store_root: PathBuf,
    cache: CachePlacement,
    snapshots_root: PathBuf,
    workspaces_root: PathBuf,
}

impl Locations {
    /// Production layout: capture `EMERY_HOME` once. A non-empty
    /// absolute override wins; otherwise `$HOME/.emery`, then
    /// `<temp>/emery`. All four roots derive together as
    /// `<home>/{store,cache,snapshots,workspaces}`.
    ///
    /// Composition-root only — never called from kernels or handlers.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn from_env() -> Self {
        let home = env_path("EMERY_HOME")
            .or_else(|| env_path("HOME").map(|user_home| user_home.join(".emery")))
            .unwrap_or_else(|| std::env::temp_dir().join("emery"));
        Self::explicit(home.join("store"), CachePlacement::Parent(home.join("cache")))
            .values_under(&home)
    }

    /// Explicit layout — no environment reads. Sandboxed sessions and
    /// tests pass [`CachePlacement::Parent`]; the wasm32 engine guest
    /// passes [`CachePlacement::Project`] for its already-resolved
    /// preopen. Value roots default beneath the store root's parent;
    /// [`Self::values_under`] re-homes them.
    #[must_use]
    pub fn explicit(store_root: PathBuf, cache: CachePlacement) -> Self {
        let home = store_root.parent().map_or_else(|| store_root.clone(), Path::to_path_buf);
        Self {
            store_root,
            cache,
            snapshots_root: home.join("snapshots"),
            workspaces_root: home.join("workspaces"),
        }
    }

    /// Re-home the value roots (snapshot store and workspaces) as
    /// `<home>/snapshots` and `<home>/workspaces`. Chainable after
    /// [`Self::explicit`] when the value layout diverges from the
    /// store root's parent.
    #[must_use]
    pub fn values_under(mut self, home: &Path) -> Self {
        self.snapshots_root = home.join("snapshots");
        self.workspaces_root = home.join("workspaces");
        self
    }

    /// Re-home a [`CachePlacement::Parent`] onto one shared,
    /// already-resolved `<parent>/<tenant>` directory. `system *`
    /// invocations use this so the cache is never keyed off the
    /// mounted definition home; an already-resolved placement is
    /// unchanged.
    #[must_use]
    pub fn shared_cache(mut self, tenant: &str) -> Self {
        if let CachePlacement::Parent(parent) = &self.cache {
            self.cache = CachePlacement::Project(parent.join(tenant));
        }
        self
    }

    /// The engine guest's layout: the writable cache preopen the
    /// deployment manifest grants plus the nominal (never-opened)
    /// store and snapshot roots — no environment, no project-id
    /// suffix below the cache mount. Prepared workspaces resolve
    /// under the deployment's workspaces preopen.
    #[must_use]
    pub fn guest() -> Self {
        Self {
            store_root: PathBuf::from(GUEST_STORE_MOUNT),
            cache: CachePlacement::Project(PathBuf::from(GUEST_CACHE_MOUNT)),
            snapshots_root: PathBuf::from(GUEST_SNAPSHOTS_MOUNT),
            workspaces_root: PathBuf::from(GUEST_WORKSPACES_MOUNT),
        }
    }

    /// Global store root — host-side installs and verify-and-load;
    /// origin display in-guest.
    #[must_use]
    pub fn store_root(&self) -> &Path {
        &self.store_root
    }

    /// Global store entry for an immutable `(name, version)` identity —
    /// the single component file `<store-root>/<name>@<version>.wasm`.
    ///
    /// The store is keyed by the pinned identity, not the project, so
    /// two projects pinning the same `(name, version)` resolve to one
    /// shared, immutable entry (the Cargo `~/.cargo/registry` model).
    #[must_use]
    pub fn store_entry(&self, name: &str, version: &str) -> PathBuf {
        self.store_root.join(format!("{name}@{version}.wasm"))
    }

    /// Verify-on-read digest sidecar sibling of [`Self::store_entry`] —
    /// `<store-root>/<name>@<version>.meta`.
    ///
    /// A *sibling* of the entry, never the entry itself: the sidecar
    /// is a writable provenance record that must not perturb the
    /// read-only immutability of the installed component file.
    #[must_use]
    pub fn store_meta(&self, name: &str, version: &str) -> PathBuf {
        self.store_root.join(format!("{name}@{version}.meta"))
    }

    /// Resolved per-project cache directory for `project_root`.
    ///
    /// [`CachePlacement::Parent`] appends the stable project-id digest
    /// (`diagnostics::cache::project_id`), so each checkout gets its
    /// own collision-free cache; [`CachePlacement::Project`] returns
    /// the carried, already-resolved root directly.
    #[must_use]
    pub fn project_cache_dir(&self, project_root: &Path) -> PathBuf {
        match &self.cache {
            CachePlacement::Parent(parent) => {
                parent.join(diagnostics::cache::project_id(project_root))
            }
            CachePlacement::Project(dir) => dir.clone(),
        }
    }

    /// Project component cache entry for `name` under
    /// `<project-cache>/components/<name>.wasm`.
    #[must_use]
    pub fn component(&self, project_root: &Path, name: &str) -> PathBuf {
        self.project_cache_dir(project_root).join("components").join(format!("{name}.wasm"))
    }

    /// Content-addressed snapshot store root — host-side snapshot and
    /// materialization; nominal (never opened) in-guest.
    #[must_use]
    pub fn snapshots_root(&self) -> &Path {
        &self.snapshots_root
    }

    /// Private-workspace root: the host prepares each disposable
    /// workspace beneath it; the deployment mounts it so guests open
    /// prepared workspaces by deployment-local path.
    #[must_use]
    pub fn workspaces_root(&self) -> &Path {
        &self.workspaces_root
    }
}

/// Read one environment override, accepting only a non-empty absolute
/// path; empty or relative values fall through to the effective
/// default.
#[cfg(not(target_arch = "wasm32"))]
fn env_path(key: &str) -> Option<PathBuf> {
    let value = std::env::var_os(key)?;
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    path.is_absolute().then_some(path)
}
