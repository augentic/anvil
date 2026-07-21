//! [`Locations`] — well-known on-disk locations for resolvable
//! artifacts (the global adapter store and the per-project component
//! cache).
//!
//! A plain value: construction is the only place layout policy lives.
//! [`Locations::from_env`] is the shipped production layout — defaults
//! in code, `SPECIFY_HOME` the only override, captured once at each
//! composition root. [`Locations::explicit`] is the injection point
//! for sandboxes, tests, and the engine guest's preopens. There is no
//! trait and no alternate implementation; kernels and handlers receive
//! the value through [`super::ExecutionPaths`] and never read
//! `std::env` themselves.

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
pub const GUEST_CACHE_MOUNT: &str = "/specify-cache";

/// Guest-visible preopen name of the global adapter store inside the
/// engine guest's WASI sandbox.
///
/// The generated deployment manifest mounts the host's
/// [`Locations::store_root`] under this name **writable**: forwarded
/// workflow verbs resolve pinned identities in-guest (store probe,
/// verify-on-read against the `.meta` sidecar), and `specify init`
/// hydration installs a missing pin through the same mount (fetch over
/// the provider's `Resolver::ensure_*` leg, write entry + digest
/// sidecar, verify-after-write). Installed entries themselves remain
/// immutable — hydration only ever creates absent `(name, version)`
/// files.
pub const GUEST_STORE_MOUNT: &str = "/specify-store";

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
/// Owns the two production roots — the global adapter store (pinned
/// identities plus digest sidecars) and the project component cache
/// (operator-seeded local components) — and the layout formulas over
/// them. Methods are pure path math; the roots are fixed at
/// construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locations {
    store_root: PathBuf,
    cache: CachePlacement,
}

#[cfg(not(target_arch = "wasm32"))]
mod config {
    //! The relocation override captured from the environment — the
    //! complete override surface for artifact layout.

    #![allow(
        clippy::same_name_method,
        reason = "the FromEnv derive emits inherent methods mirroring its own trait; the \
                  expansion cannot carry a narrower expectation"
    )]

    use std::path::PathBuf;

    use fromenv::FromEnv;

    // `pub` satisfies the derive's visibility requirement; the private
    // `config` module keeps the type crate-internal.
    #[derive(Debug, Clone, FromEnv)]
    pub struct Overrides {
        /// Umbrella Specify root. Store and cache derive together.
        #[env(from = "SPECIFY_HOME")]
        pub home: Option<PathBuf>,
        /// The user home the effective default derives from.
        #[env(from = "HOME")]
        pub user_home: Option<PathBuf>,
    }
}

impl Locations {
    /// Production layout: capture `SPECIFY_HOME` once. A non-empty
    /// absolute override wins; otherwise the effective home is
    /// `$HOME/.specify`, then `<temp>/specify` when `$HOME` is
    /// unavailable. Store and cache derive together as `<home>/store`
    /// and `<home>/cache`.
    ///
    /// Composition-root only — never called from kernels or handlers;
    /// the wasm32 engine guest constructs [`Self::explicit`] over its
    /// preopens instead.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn from_env() -> Self {
        let overrides = config::Overrides::from_env().finalize().ok();
        let (home, user_home) =
            overrides.map_or((None, None), |captured| (captured.home, captured.user_home));
        let home = home
            .filter(|path| absolute(path))
            .or_else(|| user_home.filter(|path| absolute(path)).map(|path| path.join(".specify")))
            .unwrap_or_else(|| std::env::temp_dir().join("specify"));
        Self::explicit(home.join("store"), CachePlacement::Parent(home.join("cache")))
    }

    /// Explicit layout — no environment reads. Sandboxed sessions and
    /// tests pass [`CachePlacement::Parent`]; the wasm32 engine guest
    /// passes [`CachePlacement::Project`] for its already-resolved
    /// preopen.
    #[must_use]
    pub const fn explicit(store_root: PathBuf, cache: CachePlacement) -> Self {
        Self { store_root, cache }
    }

    /// The engine guest's layout: the two writable preopens the
    /// deployment manifest grants — no environment, no project-id
    /// suffix below the cache mount.
    #[must_use]
    pub fn guest() -> Self {
        Self::explicit(
            PathBuf::from(GUEST_STORE_MOUNT),
            CachePlacement::Project(PathBuf::from(GUEST_CACHE_MOUNT)),
        )
    }

    /// Global store root used for hydration and the launcher mount.
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
}

/// Accept an override only when it is a non-empty absolute path;
/// empty or relative values fall through to the effective default.
#[cfg(not(target_arch = "wasm32"))]
fn absolute(path: &Path) -> bool {
    !path.as_os_str().is_empty() && path.is_absolute()
}
