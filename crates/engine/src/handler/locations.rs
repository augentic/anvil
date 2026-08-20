//! [`Locations`] — well-known on-disk roots (adapter store, component
//! cache) as fixed layout formulas. Every path is a constant relative
//! to a named preopen; kernels never read `std::env` themselves.

use std::path::{Path, PathBuf};

/// Guest-visible preopen name of the per-project derived cache.
///
/// The deployment manifest mounts the host's CWD-relative
/// `.emery-cache` under the same name, so the string resolves
/// identically against the wasm32 preopen table and the native
/// invocation directory; one project per deployment, so no project-id
/// keying is needed.
pub const GUEST_CACHE_MOUNT: &str = ".emery-cache";

/// Nominal store root inside the engine guest's layout.
///
/// The global adapter store is host-owned and gets **no** guest
/// mount: package pins dispatch by routed id, the host resolver
/// installs a missing pin during that dispatch, and the guest never
/// opens a store file. The constant survives as the guest's nominal
/// [`Locations::store_root`] — pure path math feeding origin display,
/// never I/O.
pub const GUEST_STORE_MOUNT: &str = "/emery-store";

/// Well-known on-disk locations for resolvable artifacts.
///
/// Owns the layout formulas over the two fixed roots — the nominal
/// global adapter store and the project component cache preopen.
/// Methods are pure path math over the mount constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Locations;

impl Locations {
    /// Global store root — host-side installs and verify-and-load;
    /// origin display in-guest.
    #[must_use]
    pub fn store_root(&self) -> &Path {
        Path::new(GUEST_STORE_MOUNT)
    }

    /// Global store entry for an immutable `(name, version)` identity —
    /// the single component file `<store-root>/<name>@<version>.wasm`.
    ///
    /// The store is keyed by the pinned identity, not the project, so
    /// two projects pinning the same `(name, version)` resolve to one
    /// shared, immutable entry (the Cargo `~/.cargo/registry` model).
    #[must_use]
    pub fn store_entry(&self, name: &str, version: &str) -> PathBuf {
        self.store_root().join(format!("{name}@{version}.wasm"))
    }

    /// Verify-on-read digest sidecar sibling of [`Self::store_entry`] —
    /// `<store-root>/<name>@<version>.meta`.
    ///
    /// A *sibling* of the entry, never the entry itself: the sidecar
    /// is a writable provenance record that must not perturb the
    /// read-only immutability of the installed component file.
    #[must_use]
    pub fn store_meta(&self, name: &str, version: &str) -> PathBuf {
        self.store_root().join(format!("{name}@{version}.meta"))
    }

    /// The per-project derived cache root: the cache preopen.
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        PathBuf::from(GUEST_CACHE_MOUNT)
    }

    /// Project component cache entry for `name` under
    /// `<project-cache>/components/<name>.wasm`.
    #[must_use]
    pub fn component(&self, name: &str) -> PathBuf {
        self.cache_dir().join("components").join(format!("{name}.wasm"))
    }
}
