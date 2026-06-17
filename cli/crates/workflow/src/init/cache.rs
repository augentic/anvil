//! Adapter cache management plus the on-disk
//! `manifest-meta.yaml` representation inside the out-of-tree cache.
//!
//! `cache_adapter` copies a resolved source into the manifest cache at
//! `<project-cache>/manifests/targets/<name>/` and stamps
//! `manifest-meta.yaml` inside the `manifests/` tree (mirroring
//! `codex/codex-meta.yaml` — each cache tenant is self-describing).
//! The agent owns writes to the manifest cache; the CLI reads
//! [`ManifestMeta`]'s path only to decide whether the cache is
//! populated.
//!
//! `cache_codex` distributes the shared codex packs that ship beside
//! the target adapter in its source repo
//! (`adapters/shared/rules/{universal,core}/`) into the project codex
//! cache at `<project-cache>/codex/`, pinned to the same source/ref as
//! the adapter. The codex resolver's rules-root probe finds that tree
//! without a co-located framework checkout or a manual `--rules-root`
//! (RM-07). Provenance is stamped in [`CodexMeta`].

use std::fs;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use specify_error::Error;

use crate::adapter::{Axis, cache_dir as adapter_cache_dir, check_axis_unique_for_name};
use crate::config::Layout;
use crate::init::adapter_uri::AdapterUri;

/// Provenance for the adapter manifest mirror under
/// `<project-cache>/manifests/`. The structural twin of [`CodexMeta`]:
/// each cache tenant carries its own metadata inside its own tree.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ManifestMeta {
    /// The adapter source value (a `file://` or `https://…@ref` URI)
    /// the manifest mirror was populated from.
    pub source: String,
    /// ISO 8601 timestamp of when the mirror was last fetched.
    pub fetched_at: String,
}

impl ManifestMeta {
    /// Absolute path to `manifest-meta.yaml` inside the out-of-tree
    /// `<project-cache>/manifests/` tenant.
    #[must_use]
    pub fn path(project_dir: &Path) -> PathBuf {
        Layout::new(project_dir).cache_dir().join("manifests").join("manifest-meta.yaml")
    }
}

/// Copy the resolved adapter source into the project's source/target adapter split
/// axis-aware cache and stamp `manifest-meta.yaml`. Returns the resolved
/// [`AdapterUri`] so the caller can record `project.yaml.adapter`
/// (`source.adapter_value`) and reuse the same resolved checkout for
/// codex distribution ([`cache_codex`]) without re-cloning.
pub(super) fn cache_adapter(
    adapter: &str, project_dir: &Path, now: Timestamp,
) -> Result<AdapterUri, Error> {
    if adapter.trim().is_empty() || adapter != adapter.trim() {
        return Err(Error::Diag {
            code: "adapter-arg-malformed",
            detail: "<adapter> must be non-empty and must not have leading or trailing whitespace"
                .to_string(),
        });
    }

    let source = AdapterUri::parse(adapter, project_dir)?;
    // Cross-axis uniqueness: a target adapter being cached must not
    // collide with an in-repo `adapters/sources/<name>/` (or its
    // cached mirror). See DECISIONS.md §"Adapter name uniqueness".
    // Probing here gives the operator a clean diagnostic before the
    // cache directory is rewritten and ahead of the downstream
    // `TargetAdapter::resolve` call in `init/regular.rs`.
    check_axis_unique_for_name(Axis::Target, &source.adapter_name, project_dir)?;
    // RFC-48 D5 / L3: a package-ref source already lives in the global
    // content-addressed store, which the resolver probes first by pinned
    // `(name, version)`. Skip the per-project manifest-cache copy and its
    // provenance stamp — there is nothing to mirror, and the store entry
    // is read in place. A non-store source is mirrored into the cache as
    // before.
    if !source.from_store {
        let target = adapter_cache_dir(project_dir, Axis::Target, &source.adapter_name);
        refresh_cached_adapter(&source.source_dir, &target)?;
        write_manifest_meta(project_dir, &source.adapter_value, now)?;
    }

    Ok(source)
}

/// Project-relative path to the universal shared-rules pack inside a
/// framework source tree. The codex resolver joins this same relative
/// path onto its rules root, so mirroring it under the cache keeps the
/// probe free of special-casing.
const UNIVERSAL_RULES_REL: &str = "adapters/shared/rules/universal";
/// Project-relative path to the framework `core/` pack (distributed
/// only under `--include-framework`).
const CORE_RULES_REL: &str = "adapters/shared/rules/core";

/// Absolute path to the project codex cache root,
/// `<project-cache>/codex/` (out-of-tree). Shared/core packs land
/// beneath it mirroring `adapters/shared/rules/{universal,core}/`.
#[must_use]
pub fn codex_cache_root(project_dir: &Path) -> PathBuf {
    Layout::new(project_dir).cache_dir().join("codex")
}

/// Provenance for the distributed shared codex tree.
///
/// Stamped beside the cached rules so a consumer (and CI) can prove
/// which adapter source/ref the codex was pinned to. Audit-only: the
/// codex resolver never reads it.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CodexMeta {
    /// The adapter source value (a `file://` or `https://…@ref` URI)
    /// the codex was copied from. Pins the codex to the same source and
    /// ref as the project's target adapter.
    pub source: String,
    /// Whether the framework `core/` pack was distributed alongside the
    /// shared `universal/` pack (`--include-framework`).
    pub include_framework: bool,
    /// ISO 8601 timestamp of the codex fetch.
    pub fetched_at: String,
}

impl CodexMeta {
    /// Absolute path to `codex-meta.yaml` inside the out-of-tree
    /// `<project-cache>/codex/` tenant.
    #[must_use]
    pub fn path(project_dir: &Path) -> PathBuf {
        codex_cache_root(project_dir).join("codex-meta.yaml")
    }
}

/// Copy the shared codex packs from the resolved adapter's source repo
/// into the project codex cache and stamp [`CodexMeta`].
///
/// The shared codex lives at a sibling path in the same source tree the
/// target adapter resolves from. This walks up from the adapter's
/// `source_dir` to the nearest ancestor that carries the shared
/// `universal/` pack, replaces the out-of-tree `<project-cache>/codex/` with a fresh
/// copy of that pack (and, when `include_framework`, the framework
/// `core/` pack), and records provenance pinned to `source.adapter_value`.
///
/// Returns `Ok(true)` when the codex was distributed, `Ok(false)` when
/// the source tree carries no shared `universal/` pack — a fail-soft
/// path so the adapter cache still succeeds; the consumer then falls
/// back to `--rules-root` or a monorepo checkout.
pub(super) fn cache_codex(
    project_dir: &Path, source: &AdapterUri, include_framework: bool, now: Timestamp,
) -> Result<bool, Error> {
    let Some(repo_root) = repo_root_with_codex(&source.source_dir) else {
        return Ok(false);
    };

    let codex_root = codex_cache_root(project_dir);
    if codex_root.exists() {
        fs::remove_dir_all(&codex_root)?;
    }
    fs::create_dir_all(&codex_root)?;

    copy_dir_recursive(
        &repo_root.join(UNIVERSAL_RULES_REL),
        &codex_root.join(UNIVERSAL_RULES_REL),
    )?;

    let core_src = repo_root.join(CORE_RULES_REL);
    if include_framework && core_src.is_dir() {
        copy_dir_recursive(&core_src, &codex_root.join(CORE_RULES_REL))?;
    }

    write_codex_meta(project_dir, &source.adapter_value, include_framework, now)?;
    Ok(true)
}

/// Walk up from a resolved adapter `source_dir` to the nearest ancestor
/// that carries the shared `universal/` pack. The same walk works for
/// local sources (canonicalised adapter dir under a repo checkout) and
/// for git sources (the sparse checkout temp dir, which now also fetches
/// `adapters/shared/rules/` — see `init/git.rs`).
fn repo_root_with_codex(source_dir: &Path) -> Option<PathBuf> {
    source_dir.ancestors().find(|dir| dir.join(UNIVERSAL_RULES_REL).is_dir()).map(Path::to_path_buf)
}

fn write_codex_meta(
    project_dir: &Path, source: &str, include_framework: bool, now: Timestamp,
) -> Result<(), Error> {
    let meta = CodexMeta {
        source: source.to_string(),
        include_framework,
        fetched_at: now.strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };
    let serialised = serde_saphyr::to_string(&meta)?;
    fs::write(CodexMeta::path(project_dir), serialised)?;
    Ok(())
}

fn refresh_cached_adapter(source: &Path, target: &Path) -> Result<(), Error> {
    if target.exists() {
        fs::remove_dir_all(target)?;
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    // `copy_dir_recursive` dereferences the adapter's own symlinks into
    // real bytes. The former `vendor_spec_runtime` ancestor-walk is
    // retired (RFC-48 D9/D12): published adapters are self-contained, so
    // the consumer cache vendors no shared content of its own.
    copy_dir_recursive(source, target)
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<(), Error> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        // Follow directory symlinks (e.g. an adapter's `references/spec-runtime`
        // symlink into the shared bundle) and dereference file symlinks so the
        // cached adapter is self-contained with real bytes.
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}

fn write_manifest_meta(
    project_dir: &Path, adapter_value: &str, now: Timestamp,
) -> Result<(), Error> {
    let meta = ManifestMeta {
        source: adapter_value.to_string(),
        fetched_at: now.strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };
    let meta_path = ManifestMeta::path(project_dir);
    let serialised = serde_saphyr::to_string(&meta)?;
    fs::write(meta_path, serialised)?;
    Ok(())
}
