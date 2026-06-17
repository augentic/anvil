//! Out-of-tree per-project cache root resolution.
//!
//! The adapter manifest mirror and the distributed codex pack are
//! regenerable, machine-owned state — never committed, never authored.
//! Rather than scatter them through the repository under
//! `.specify/cache/`, they live in a per-project directory inside the
//! user's OS cache, keyed by a stable digest of the canonicalised
//! project path. Each checkout — including each materialised workspace
//! slot — gets its own collision-free cache that survives `git clean`
//! and never pollutes the working tree.
//!
//! Lives on the `specify-schema` leaf so both `specify-workflow` (which
//! populates the cache at init/sync) and `specify-standards` (which
//! reads it during rule resolution) resolve the same root without a
//! cross-layer dependency.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::digest::{Hasher, sha256_hex};

/// Environment override for the per-project cache parent. When set to
/// an absolute path, per-project directories are created directly
/// beneath it (the `specify/projects` suffix is *not* appended).
const CACHE_ENV: &str = "SPECIFY_PROJECT_CACHE";

/// Environment override for the persistent Git mirror parent. When set
/// to an absolute path, per-URL bare-ish mirror directories are created
/// directly beneath it (the `specify/mirrors` suffix is *not* appended).
const MIRROR_ENV: &str = "SPECIFY_MIRROR_CACHE";

/// Absolute path to the persistent Git mirror for `url` —
/// `<mirrors-root>/<url-id>.git`.
///
/// `<url-id>` is the lowercase SHA-256 hex of the registry URL, so a
/// peer's object store is shared across changes and checkouts: the slot
/// at `workspace/<peer>/` becomes a `git worktree` of this mirror rather
/// than a fresh full clone each time. Infallible for the same reasons
/// as [`project_cache_dir`].
#[must_use]
pub fn mirror_dir(url: &str) -> PathBuf {
    mirrors_root().join(format!("{}.git", sha256_hex(url.as_bytes())))
}

/// Resolve the parent directory that holds every URL's Git mirror.
///
/// Precedence: `$SPECIFY_MIRROR_CACHE`, then
/// `$XDG_CACHE_HOME/specify/mirrors`, then
/// `$HOME/.cache/specify/mirrors`, then `<temp>/specify/mirrors`.
fn mirrors_root() -> PathBuf {
    if let Some(root) = env::var_os(MIRROR_ENV).and_then(absolute) {
        return root;
    }
    if let Some(root) = env::var_os("XDG_CACHE_HOME").and_then(absolute) {
        return root.join("specify").join("mirrors");
    }
    if let Some(home) = env::var_os("HOME").and_then(absolute) {
        return home.join(".cache").join("specify").join("mirrors");
    }
    env::temp_dir().join("specify").join("mirrors")
}

/// Absolute path to the out-of-tree cache directory for `project_dir` —
/// `<projects-root>/<project-id>/`.
///
/// `<project-id>` is the lowercase SHA-256 hex of the canonicalised
/// project path, so the root is stable across invocations and unique
/// per checkout. Tenants (`manifests/`, `codex/`, …) are created by the
/// caller beneath the returned directory.
///
/// Infallible by design: cache path helpers across the workflow and
/// standards layers are infallible, and a regenerable cache must never
/// fall back into the working tree. When no environment anchor is
/// available the OS temp directory is used as a last resort.
#[must_use]
pub fn project_cache_dir(project_dir: &Path) -> PathBuf {
    project_cache_dir_in(&projects_root(), project_dir)
}

/// Per-project cache directory beneath an explicit `projects_root` —
/// `<projects_root>/<project-id>/`.
///
/// The root-injecting form behind [`project_cache_dir`]. Tests use it
/// to compute the expected location for a chosen temp root without
/// mutating the process environment.
#[must_use]
pub fn project_cache_dir_in(projects_root: &Path, project_dir: &Path) -> PathBuf {
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

/// Environment override for the global adapter store root (RFC-48 D5).
/// When set to an absolute path, store entries are created directly
/// beneath it (the `specify/adapters` suffix is *not* appended).
const ADAPTER_STORE_ENV: &str = "SPECIFY_ADAPTER_CACHE";

/// Absolute path to the global adapter store entry for an immutable
/// `(name, version)` identity — `<store>/<name>@<version>/` (RFC-48 D5).
///
/// The store is keyed by the pinned identity, not the project, so two
/// projects pinning the same `(name, version)` resolve to one shared,
/// read-only entry (the Cargo `~/.cargo/registry` model). Install
/// orchestration (pull → temp → verify → atomic rename → chmod) lives in
/// the workflow layer; this is the pure location resolver both install
/// and read paths agree on.
#[must_use]
pub fn adapter_store_entry(name: &str, version: &str) -> PathBuf {
    adapter_store_root().join(format!("{name}@{version}"))
}

/// Resolve the parent directory that holds every adapter's
/// content-addressed store entry.
///
/// Precedence: `$SPECIFY_ADAPTER_CACHE`, then
/// `$XDG_CACHE_HOME/specify/adapters`, then
/// `$HOME/.cache/specify/adapters`, then `<temp>/specify/adapters`.
/// Empty or relative overrides are skipped rather than treated as an
/// error, mirroring `projects_root`.
#[must_use]
pub fn adapter_store_root() -> PathBuf {
    if let Some(root) = env::var_os(ADAPTER_STORE_ENV).and_then(absolute) {
        return root;
    }
    if let Some(root) = env::var_os("XDG_CACHE_HOME").and_then(absolute) {
        return root.join("specify").join("adapters");
    }
    if let Some(home) = env::var_os("HOME").and_then(absolute) {
        return home.join(".cache").join("specify").join("adapters");
    }
    env::temp_dir().join("specify").join("adapters")
}

/// Absolute path to the verify-on-read sidecar for a store entry
/// (RFC-48 D4) — `<store>/<name>@<version>.meta`.
///
/// A *sibling* of [`adapter_store_entry`], never a child: the sidecar is
/// a writable provenance record that [`tree_content_digest`] must not
/// walk (it digests only the entry's own tree) and that must not perturb
/// the read-only immutability of the installed entry.
#[must_use]
pub fn store_meta_path(name: &str, version: &str) -> PathBuf {
    adapter_store_root().join(format!("{name}@{version}.meta"))
}

/// Deterministic content digest over an installed store entry tree, in
/// the `sha256:<hex>` form (RFC-48 D4).
///
/// Walks every regular file under `entry` recursively, sorts by the
/// forward-slash relative path so the result is independent of the
/// filesystem's iteration order, and folds each `(relative path, length,
/// file bytes)` triple into a single SHA-256. The framing is internal:
/// the install (record) and resolve (verify) paths both call this one
/// function, so they always agree on a clean tree.
///
/// Infallible by design, mirroring the other cache helpers — a directory
/// that cannot be walked or a file that cannot be read contributes
/// nothing rather than poisoning the digest, since a healthy read-only
/// store entry never trips those paths.
#[must_use]
pub fn tree_content_digest(entry: &Path) -> String {
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    collect_entry_files(entry, entry, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Hasher::new();
    for (rel, path) in &files {
        let bytes = std::fs::read(path).unwrap_or_default();
        hasher.update(rel.as_bytes());
        hasher.update(b"\0");
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    format!("sha256:{}", hasher.finalize_hex())
}

/// Verify-on-read sidecar contents (RFC-48 D4). Registry-internal YAML;
/// deliberately *not* an embedded JSON Schema artifact.
#[derive(Debug, Serialize, Deserialize)]
struct StoreMeta {
    /// Deterministic [`tree_content_digest`] of the installed entry.
    tree_digest: String,
    /// Registry layer content digest recorded for provenance only.
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
/// `(name, version)` (RFC-48 D4 record-on-install).
///
/// `tree_digest` is the [`tree_content_digest`] of the freshly installed
/// entry; `layer_digest` is the registry layer content digest, recorded
/// for provenance when known. The sidecar is a writable sibling of the
/// read-only entry ([`store_meta_path`]).
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

/// Verify a store entry against its recorded tree digest (RFC-48 D4
/// verify-on-read).
///
/// Reads the recorded digest from the sidecar, recomputes
/// [`tree_content_digest`] over the entry, and reports a
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
    let actual = tree_content_digest(&adapter_store_entry(name, version));
    if actual == recorded { Ok(()) } else { Err(DigestMismatch { recorded, actual }) }
}

/// Recursively collect `(relative slash-path, absolute path)` for every
/// regular file under `root`, used by [`tree_content_digest`]. Symlinks
/// are not followed: installed store entries carry only plain files and
/// directories (the pack stage dereferences symlinks at publish time).
fn collect_entry_files(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            collect_entry_files(root, &path, out);
        } else if meta.is_file()
            && let Some(rel) = relative_slash_path(root, &path)
        {
            out.push((rel, path));
        }
    }
}

/// Forward-slash relative path of `path` under `root`, or `None` for a
/// non-UTF-8 component or a path outside `root`.
fn relative_slash_path(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in rel.components() {
        parts.push(component.as_os_str().to_str()?);
    }
    Some(parts.join("/"))
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        adapter_store_entry, adapter_store_root, project_cache_dir, store_meta_path,
        tree_content_digest,
    };

    #[test]
    fn distinct_projects_get_distinct_dirs() {
        let a = project_cache_dir(Path::new("/some/project/a"));
        let b = project_cache_dir(Path::new("/some/project/b"));
        assert_ne!(a, b);
        assert_eq!(a.parent(), b.parent(), "both live under the same projects root");
    }

    #[test]
    fn same_project_is_stable() {
        let first = project_cache_dir(Path::new("/some/project/a"));
        let second = project_cache_dir(Path::new("/some/project/a"));
        assert_eq!(first, second);
    }

    #[test]
    fn store_entry_keys_by_name_and_version() {
        let entry = adapter_store_entry("omnia", "1.2.0");
        assert_eq!(entry.file_name().unwrap(), "omnia@1.2.0");
        assert_eq!(entry.parent().unwrap(), adapter_store_root());
    }

    #[test]
    fn store_entry_distinct_per_pinned_identity() {
        // The store is keyed by the immutable identity, so two versions
        // of one adapter never collide and share no entry.
        assert_ne!(adapter_store_entry("omnia", "1.2.0"), adapter_store_entry("omnia", "1.3.0"));
        assert_eq!(
            adapter_store_entry("omnia", "1.2.0"),
            adapter_store_entry("omnia", "1.2.0"),
            "the same pinned identity is stable across calls"
        );
    }

    #[test]
    fn store_meta_path_is_entry_sibling_not_child() {
        // RFC-48 D4: the verify-on-read sidecar must sit beside the entry,
        // never inside the tree `tree_content_digest` walks, so recording
        // it cannot perturb the entry's own digest.
        let entry = adapter_store_entry("omnia", "1.2.0");
        let meta = store_meta_path("omnia", "1.2.0");
        assert_eq!(meta.parent(), entry.parent(), "the sidecar is a sibling of the entry");
        assert!(
            !meta.starts_with(&entry),
            "the sidecar must not live inside the walked entry tree"
        );
        assert_eq!(meta.file_name().expect("sidecar file name"), "omnia@1.2.0.meta");
    }

    #[test]
    fn tree_digest_is_order_independent_and_content_sensitive() {
        use std::fs;

        // The digest commits to the whole tree deterministically: the
        // same bytes hash identically across calls, and changing any
        // file's bytes changes the digest (RFC-48 D4 verify-on-read).
        let dir = tempfile::tempdir().expect("tempdir");
        let entry = dir.path().join("omnia@1.0.0");
        fs::create_dir_all(entry.join("briefs")).expect("mkdir briefs");
        fs::write(entry.join("adapter.yaml"), b"name: omnia\n").expect("write manifest");
        fs::write(entry.join("briefs/build.md"), b"# build\n").expect("write brief");

        let first = tree_content_digest(&entry);
        assert!(first.starts_with("sha256:"), "digest carries the sha256 prefix: {first}");
        assert_eq!(first, tree_content_digest(&entry), "a stable tree digests identically");

        fs::write(entry.join("briefs/build.md"), b"# build changed\n").expect("rewrite brief");
        assert_ne!(
            first,
            tree_content_digest(&entry),
            "changing a file's bytes must change the tree digest"
        );
    }
}
