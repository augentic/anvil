//! [`Locations`] — well-known on-disk roots (adapter store, component
//! cache). Kernels and handlers receive the value through
//! [`super::ExecutionPaths`] and never read `std::env` themselves.

use std::path::{Path, PathBuf};

/// Guest-visible preopen name of the per-project derived cache inside
/// the engine guest's WASI sandbox.
///
/// The generated deployment manifest mounts the host's resolved
/// [`Locations::project_cache_dir`] under this name; the guest
/// constructs its `Locations` over the preopen directly — one project
/// per deployment, so no project-id keying is needed in-guest.
pub const GUEST_CACHE_MOUNT: &str = "/emery-cache";

/// Nominal store root inside the engine guest's layout.
///
/// The global adapter store is host-owned and gets **no** guest
/// mount: package pins dispatch by routed id, the host resolver
/// installs a missing pin during that dispatch, and the guest never
/// opens a store file. The constant survives as the guest's nominal
/// [`Locations::store_root`] — pure path math feeding origin display,
/// never I/O.
pub const GUEST_STORE_MOUNT: &str = "/emery-store";

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
/// identities plus digest sidecars) and the project component cache
/// (operator-seeded local components) — and the layout formulas over
/// them. Methods are pure path math; the roots are fixed at
/// construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locations {
    store_root: PathBuf,
    cache: CachePlacement,
}

impl Locations {
    /// Explicit layout — no environment reads. Sandboxed sessions and
    /// tests pass [`CachePlacement::Parent`]; the wasm32 engine guest
    /// passes [`CachePlacement::Project`] for its already-resolved
    /// preopen.
    #[must_use]
    pub const fn explicit(store_root: PathBuf, cache: CachePlacement) -> Self {
        Self { store_root, cache }
    }

    /// The engine guest's layout: the writable cache preopen the
    /// deployment manifest grants plus the nominal (never-opened)
    /// store root — no environment, no project-id suffix below the
    /// cache mount.
    #[must_use]
    pub fn guest() -> Self {
        Self {
            store_root: PathBuf::from(GUEST_STORE_MOUNT),
            cache: CachePlacement::Project(PathBuf::from(GUEST_CACHE_MOUNT)),
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
    /// (`emery_diagnostics::cache::project_id`), so each checkout gets its
    /// own collision-free cache; [`CachePlacement::Project`] returns
    /// the carried, already-resolved root directly.
    #[must_use]
    pub fn project_cache_dir(&self, project_root: &Path) -> PathBuf {
        match &self.cache {
            CachePlacement::Parent(parent) => {
                parent.join(emery_diagnostics::cache::project_id(project_root))
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
}
