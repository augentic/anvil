//! Per-project component cache and codex distribution at init time.
//!
//! `cache_adapter` resolves the `<adapter>` argument (see
//! [`AdapterUri`]) and, for a local `.wasm` component outside the
//! resolver's probe set, mirrors it into the out-of-tree component
//! cache at `<project-cache>/components/<name>.wasm` — the project-local
//! leg the bare-name resolver probes first. Store entries (package
//! references) and development release builds are read in place and
//! never mirrored. Provenance is stamped in [`ComponentMeta`].
//!
//! `cache_codex` materializes the shared codex packs compiled into the
//! binary into the project codex cache at `<project-cache>/codex/`,
//! pinned to the binary version. The codex resolver's rules-root probe
//! finds that tree without a co-located framework checkout or a manual
//! `--rules-root`. Provenance is stamped in [`CodexMeta`].

use std::fs;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use specify_error::Error;

use crate::adapter::component_cache_entry;
use crate::config::Layout;
use crate::init::adapter_uri::{AdapterOrigin, AdapterUri};

/// Provenance for the mirrored component under
/// `<project-cache>/components/`. The structural twin of [`CodexMeta`]:
/// each cache tenant carries its own metadata inside its own tree.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ComponentMeta {
    /// The adapter source value (a `file://` component URI) the
    /// component cache was populated from.
    pub source: String,
    /// ISO 8601 timestamp of when the component was last mirrored.
    pub fetched_at: String,
}

impl ComponentMeta {
    /// Absolute path to `component-meta.yaml` inside the out-of-tree
    /// `<project-cache>/components/` tenant.
    #[must_use]
    pub fn path(project_dir: &Path) -> PathBuf {
        Layout::new(project_dir).cache_dir().join("components").join("component-meta.yaml")
    }
}

/// Resolve the `<adapter>` argument and mirror a local component into
/// the project component cache when needed. Returns the resolved
/// [`AdapterUri`] so the caller can record `project.yaml.adapter`
/// (`source.adapter_value`).
///
/// Store entries (package references) resolve from the global
/// content-addressed store in place; development release builds resolve
/// live from `target/wasm32-wasip2/release/` so the adapter dev loop
/// (rebuild, re-run) never reads a stale mirror. Only an operator's own
/// local `.wasm` file is copied — into
/// `<project-cache>/components/<name>.wasm`, the project-local probe leg
/// of the bare-name resolver.
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
    if source.origin == AdapterOrigin::Local {
        let entry = component_cache_entry(project_dir, &source.adapter_name);
        if let Some(parent) = entry.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source.component, &entry)?;
        write_component_meta(project_dir, &source.adapter_value, now)?;
    }

    Ok(source)
}

/// Canonical codex layout inside the codex cache (and a framework
/// source checkout): the universal shared-rules pack. The codex
/// resolver joins this same relative path onto its rules root, so
/// mirroring it under the cache keeps the probe free of special-casing.
const UNIVERSAL_RULES_REL: &str = "codex/rules/universal";

/// Absolute path to the project codex cache root,
/// `<project-cache>/codex/` (out-of-tree). The shared pack lands
/// beneath it mirroring `codex/rules/universal/`.
#[must_use]
pub fn codex_cache_root(project_dir: &Path) -> PathBuf {
    Layout::new(project_dir).cache_dir().join("codex")
}

/// Provenance for the materialized shared codex tree.
///
/// Stamped beside the cached rules so a consumer (and CI) can prove
/// which binary version the codex was materialized from. The codex
/// resolver never reads it; `cache_codex` compares it against the
/// running binary to decide whether to re-materialize.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CodexMeta {
    /// The `specify` binary version the embedded packs were
    /// materialized from (the codex's only version knob).
    pub source: String,
    /// ISO 8601 timestamp of the codex materialization.
    pub fetched_at: String,
}

impl CodexMeta {
    /// Absolute path to `codex-meta.yaml` inside the out-of-tree
    /// `<project-cache>/codex/` tenant.
    #[must_use]
    pub fn path(project_dir: &Path) -> PathBuf {
        codex_cache_root(project_dir).join("codex-meta.yaml")
    }

    /// The recorded stamp, or `None` when the codex was never
    /// materialized (or the stamp is unreadable — the next
    /// materialization rewrites it).
    #[must_use]
    pub fn load(project_dir: &Path) -> Option<Self> {
        read_codex_meta(project_dir)
    }
}

/// Shared codex pack compiled into the binary: sorted
/// `(cache-relative path, contents)` entries generated by `build.rs`
/// from the adapters checkout's `codex/rules/universal/`.
static CODEX_PACK_FILES: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/codex_packs.rs"));

/// Materialize the embedded shared codex pack into the project codex
/// cache and stamp [`CodexMeta`].
///
/// Writes `universal/` under the canonical `codex/rules/universal`
/// layout the codex resolver probes. A cache already stamped by this
/// binary version is left untouched; any mismatch (an older binary's
/// stamp, a missing tree) wipes and re-materializes — the bytes are
/// already in memory, so a refresh is cheap. Returns the stamped
/// [`CodexMeta`].
pub(super) fn cache_codex(project_dir: &Path, now: Timestamp) -> Result<CodexMeta, Error> {
    let version = super::resolve_version();
    let codex_root = codex_cache_root(project_dir);
    if let Some(existing) = read_codex_meta(project_dir)
        && existing.source == version
        && codex_root.join(UNIVERSAL_RULES_REL).is_dir()
    {
        return Ok(existing);
    }

    if codex_root.exists() {
        fs::remove_dir_all(&codex_root)?;
    }
    fs::create_dir_all(&codex_root)?;

    for (rel, contents) in CODEX_PACK_FILES {
        let path = codex_root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents)?;
    }

    write_codex_meta(project_dir, &version, now)
}

/// Parse the recorded [`CodexMeta`], treating a missing or unreadable
/// stamp as "not materialized" so [`cache_codex`] rewrites the cache.
fn read_codex_meta(project_dir: &Path) -> Option<CodexMeta> {
    let raw = fs::read_to_string(CodexMeta::path(project_dir)).ok()?;
    serde_saphyr::from_str(&raw).ok()
}

fn write_codex_meta(project_dir: &Path, source: &str, now: Timestamp) -> Result<CodexMeta, Error> {
    let meta = CodexMeta {
        source: source.to_string(),
        fetched_at: now.strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };
    let serialised = serde_saphyr::to_string(&meta)?;
    fs::write(CodexMeta::path(project_dir), serialised)?;
    Ok(meta)
}

fn write_component_meta(
    project_dir: &Path, adapter_value: &str, now: Timestamp,
) -> Result<(), Error> {
    let meta = ComponentMeta {
        source: adapter_value.to_string(),
        fetched_at: now.strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };
    let serialised = serde_saphyr::to_string(&meta)?;
    fs::write(ComponentMeta::path(project_dir), serialised)?;
    Ok(())
}
