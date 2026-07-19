//! Out-of-tree per-project cache root resolution.
//!
//! The adapter manifest mirror and component mirror are regenerable,
//! machine-owned state — never committed, never authored. Rather than
//! scatter them through the repository under `.specify/cache/`, they
//! live in a per-project directory inside the user's OS cache, keyed
//! by a stable digest of the canonicalised project path. Each checkout
//! — including each materialised workspace slot — gets its own
//! collision-free cache that survives `git clean` and never pollutes
//! the working tree.
//!
//! Lives on the `diagnostics` leaf so every consumer resolves the same
//! root without a cross-layer dependency.
//!
//! The global adapter store also resolves here, but it is an install
//! store, not an evictable cache: entries are immutable,
//! digest-verified, and load-bearing at runtime, so the store lives in
//! Specify's per-user home (`$HOME/.specify/adapters`) rather than the
//! OS cache.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::digest::{Hasher, sha256_hex};

/// Environment override for the per-project cache parent. When set to
/// an absolute path, per-project directories are created directly
/// beneath it (the `specify/projects` suffix is *not* appended).
const CACHE_ENV: &str = "SPECIFY_PROJECT_CACHE";

/// Guest-visible preopen name of the per-project derived cache inside
/// the engine guest's WASI sandbox.
///
/// The generated deployment manifest mounts the host's
/// [`project_cache_dir`] under this name (guest routing: the guest
/// runs init's scaffold leg, which writes cache tenants), and the
/// wasm32 build of [`project_cache_dir`] resolves to it directly —
/// one project per deployment, so no project-id keying is needed
/// in-guest.
pub const GUEST_CACHE_MOUNT: &str = "/specify-cache";

/// Absolute path to the out-of-tree cache directory for `project_dir` —
/// `<projects-root>/<project-id>/`.
///
/// `<project-id>` is the lowercase SHA-256 hex of the canonicalised
/// project path, so the root is stable across invocations and unique
/// per checkout. Tenants (`manifests/`, `components/`, …) are created
/// by the caller beneath the returned directory.
///
/// Infallible by design: cache path helpers across the workflow and
/// standards layers are infallible, and a regenerable cache must never
/// fall back into the working tree. When no environment anchor is
/// available the OS temp directory is used as a last resort.
///
/// On wasm32 (the engine guest) the cache is the
/// [`GUEST_CACHE_MOUNT`] preopen the deployment manifest grants; a
/// deployment without the mount simply misses every cache probe, the
/// same degradation as an unpopulated cache natively.
#[must_use]
pub fn project_cache_dir(project_dir: &Path) -> PathBuf {
    project_cache_dir_under(None, project_dir)
}

/// [`project_cache_dir`] with an optional explicit cache parent.
///
/// `Some(parent)` places the per-project directory directly beneath
/// `parent` (the execution-context override sandboxed sessions carry);
/// `None` falls through to the process-start environment resolution.
#[must_use]
pub fn project_cache_dir_under(cache_parent: Option<&Path>, project_dir: &Path) -> PathBuf {
    if cfg!(target_arch = "wasm32") {
        return PathBuf::from(GUEST_CACHE_MOUNT);
    }
    cache_parent.map_or_else(
        || project_cache_dir_in(&projects_root(), project_dir),
        |parent| project_cache_dir_in(parent, project_dir),
    )
}

/// Per-project cache directory beneath an explicit `projects_root` —
/// `<projects_root>/<project-id>/`.
///
/// The root-injecting form behind [`project_cache_dir`]. Tests use it
/// to compute the expected location for a chosen temp root without
/// mutating the process environment.
#[must_use]
fn project_cache_dir_in(projects_root: &Path, project_dir: &Path) -> PathBuf {
    projects_root.join(project_id(project_dir))
}

/// Resolve the parent directory that holds every project's cache.
///
/// Precedence: `$SPECIFY_PROJECT_CACHE`, then
/// `$XDG_CACHE_HOME/specify/projects`, then
/// `$HOME/.cache/specify/projects`, then `<temp>/specify/projects`.
/// Empty or relative overrides are skipped rather than treated as an
/// error.
fn projects_root() -> PathBuf {
    if let Some(root) = env::var_os(CACHE_ENV).and_then(absolute) {
        return root;
    }
    if let Some(root) = env::var_os("XDG_CACHE_HOME").and_then(absolute) {
        return root.join("specify").join("projects");
    }
    if let Some(home) = env::var_os("HOME").and_then(absolute) {
        return home.join(".cache").join("specify").join("projects");
    }
    env::temp_dir().join("specify").join("projects")
}

/// Environment override for the global adapter store root.
/// When set to an absolute path, store entries are created directly
/// beneath it (no suffix is appended) — the relocation lever for
/// sandboxes and tests.
const ADAPTER_STORE_ENV: &str = "SPECIFY_ADAPTER_STORE";

/// Guest-visible preopen name of the global adapter store inside the
/// engine guest's WASI sandbox.
///
/// The generated deployment manifest mounts the host's
/// [`adapter_store_root`] under this name **writable**: forwarded
/// workflow verbs resolve pinned identities in-guest (store probe,
/// verify-on-read against the `.meta` sidecar), and `specify init`
/// hydration installs a missing pin through the same mount (fetch over
/// the provider's `Resolver::ensure_*` leg, write entry + digest
/// sidecar, verify-after-write). Installed entries themselves remain
/// immutable — hydration only ever creates absent `(name, version)`
/// files.
pub const GUEST_STORE_MOUNT: &str = "/specify-store";

/// Absolute path to the global adapter store entry for an immutable
/// `(name, version)` identity — the single component file
/// `<store>/<name>@<version>.wasm`.
///
/// The store is keyed by the pinned identity, not the project, so two
/// projects pinning the same `(name, version)` resolve to one shared,
/// immutable entry (the Cargo `~/.cargo/registry` model). This is the
/// pure location the install leg (`specify init` hydration) and read
/// paths agree on.
#[must_use]
pub fn adapter_store_entry(name: &str, version: &str) -> PathBuf {
    adapter_store_root().join(format!("{name}@{version}.wasm"))
}

/// Resolve the parent directory that holds every adapter's
/// content-addressed store entry.
///
/// Precedence: `$SPECIFY_ADAPTER_STORE`, then `$HOME/.specify/adapters`
/// — the store lives in Specify's per-user home, not the OS cache.
/// Empty or relative values are skipped rather than treated as an
/// error, mirroring `projects_root`; without a usable `$HOME` the OS
/// temp directory anchors a last-resort root, keeping the helper
/// infallible.
///
/// On wasm32 (the engine guest) the store is the writable
/// [`GUEST_STORE_MOUNT`] preopen the deployment manifest grants — the
/// env override is host-side relocation and never crosses the seam.
#[must_use]
pub fn adapter_store_root() -> PathBuf {
    if cfg!(target_arch = "wasm32") {
        return PathBuf::from(GUEST_STORE_MOUNT);
    }
    if let Some(root) = env::var_os(ADAPTER_STORE_ENV).and_then(absolute) {
        return root;
    }
    if let Some(home) = env::var_os("HOME").and_then(absolute) {
        return home.join(".specify").join("adapters");
    }
    env::temp_dir().join("specify").join("adapters")
}

/// Absolute path to the verify-on-read sidecar for a store entry —
/// `<store>/<name>@<version>.meta`.
///
/// A *sibling* of [`adapter_store_entry`], never the entry itself: the
/// sidecar is a writable provenance record that must not perturb the
/// read-only immutability of the installed component file.
#[must_use]
fn store_meta_path(name: &str, version: &str) -> PathBuf {
    adapter_store_root().join(format!("{name}@{version}.meta"))
}

/// Deterministic content digest of one file, in the `sha256:<hex>`
/// form. A store entry is a single component file, so the entry digest
/// is the file's byte digest.
///
/// Infallible by design, mirroring the other cache helpers — an
/// unreadable file digests as empty rather than poisoning the caller,
/// since a healthy read-only store entry never trips that path.
#[must_use]
pub fn file_content_digest(file: &Path) -> String {
    let bytes = std::fs::read(file).unwrap_or_default();
    let mut hasher = Hasher::new();
    hasher.update(&bytes);
    format!("sha256:{}", hasher.finalize_hex())
}

/// Verify-on-read sidecar contents. Registry-internal YAML;
/// deliberately *not* an embedded JSON Schema artifact.
#[derive(Debug, Serialize, Deserialize)]
struct StoreMeta {
    /// Deterministic [`file_content_digest`] of the installed component.
    tree_digest: String,
    /// Registry content digest recorded for provenance only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    layer_digest: Option<String>,
}

/// The recorded vs recomputed entry digests when verify-on-read fails.
///
/// Returned by [`verify_store_entry`] when a store entry's current tree
/// content digest no longer matches the digest recorded at install time
/// — the signal that an immutable artifact has drifted (a moved tag, a
/// corrupted store entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestMismatch {
    /// Digest recorded in the sidecar at install time.
    pub recorded: String,
    /// Digest recomputed from the entry's current contents.
    pub actual: String,
}

/// Write the verify-on-read sidecar beside the store entry for
/// `(name, version)`, at install time.
///
/// `tree_digest` is the [`file_content_digest`] of the freshly
/// installed component; `layer_digest` is the registry content digest,
/// recorded for provenance when known. The sidecar is a writable
/// sibling of the read-only entry (`<name>@<version>.meta`).
///
/// # Errors
///
/// Returns the underlying [`std::io::Error`] when the sidecar cannot be
/// serialised or written.
pub fn write_store_meta(
    name: &str, version: &str, tree_digest: &str, layer_digest: Option<&str>,
) -> std::io::Result<()> {
    let meta = StoreMeta {
        tree_digest: tree_digest.to_string(),
        layer_digest: layer_digest.map(ToString::to_string),
    };
    let body =
        serde_saphyr::to_string(&meta).map_err(|err| std::io::Error::other(err.to_string()))?;
    std::fs::write(store_meta_path(name, version), body)
}

/// Read the recorded tree digest from the verify-on-read sidecar for
/// `(name, version)`, or `None` when no sidecar exists or it cannot be
/// parsed.
///
/// `None` is the fail-open signal for a legacy or foreign store entry
/// installed before the sidecar existed — verify-on-read treats it as a
/// pass rather than refusing the entry.
#[must_use]
pub fn read_store_meta(name: &str, version: &str) -> Option<String> {
    let raw = std::fs::read_to_string(store_meta_path(name, version)).ok()?;
    let meta: StoreMeta = serde_saphyr::from_str(&raw).ok()?;
    Some(meta.tree_digest)
}

/// Verify a store entry against its recorded digest (verify-on-read).
///
/// Reads the recorded digest from the sidecar, recomputes
/// [`file_content_digest`] over the component file, and reports a
/// [`DigestMismatch`] when they differ. A missing sidecar is fail-open
/// (`Ok`): legacy and foreign entries predate the sidecar, and the
/// entry's own read-only immutability remains the baseline guarantee.
///
/// # Errors
///
/// Returns [`DigestMismatch`] when the recorded and recomputed digests
/// differ.
pub fn verify_store_entry(name: &str, version: &str) -> Result<(), DigestMismatch> {
    let Some(recorded) = read_store_meta(name, version) else {
        return Ok(());
    };
    let actual = file_content_digest(&adapter_store_entry(name, version));
    if actual == recorded { Ok(()) } else { Err(DigestMismatch { recorded, actual }) }
}

/// Stable per-project identifier — the SHA-256 hex of the canonicalised
/// project path, falling back to the raw path when canonicalisation
/// fails (e.g. the directory does not yet exist).
fn project_id(project_dir: &Path) -> String {
    let canonical =
        std::fs::canonicalize(project_dir).unwrap_or_else(|_| project_dir.to_path_buf());
    sha256_hex(canonical.as_os_str().as_encoded_bytes())
}

/// Accept an environment value only when it is a non-empty absolute path.
fn absolute(value: OsString) -> Option<PathBuf> {
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    path.is_absolute().then_some(path)
}
