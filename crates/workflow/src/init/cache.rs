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
//! `cache_codex` distributes the shared codex packs that ship beside a
//! *development* adapter build in its source checkout into the project
//! codex cache at `<project-cache>/codex/`, pinned to the same adapter
//! value. The codex resolver's rules-root probe finds that tree without
//! a co-located framework checkout or a manual `--rules-root` (RM-07).
//! A registry-installed component carries no prose tree, so codex
//! distribution is fail-soft `false` for store entries. Provenance is
//! stamped in [`CodexMeta`].

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
/// (`source.adapter_value`) and reuse the same resolved component for
/// codex distribution ([`cache_codex`]).
///
/// Store entries (package references) resolve from the global
/// content-addressed store in place; development release builds resolve
/// live from `target/wasm32-wasip2/release/` so the RFC-62 dev loop
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

/// Canonical codex layout inside the codex cache (and an engine-style
/// source checkout): the universal shared-rules pack. The codex
/// resolver joins this same relative path onto its rules root, so
/// mirroring it under the cache keeps the probe free of special-casing.
const UNIVERSAL_RULES_REL: &str = "adapters/shared/prose/rules/universal";
/// Canonical codex layout for the framework `core/` pack (distributed
/// only under `--include-framework`).
const CORE_RULES_REL: &str = "adapters/shared/prose/rules/core";

/// Absolute path to the project codex cache root,
/// `<project-cache>/codex/` (out-of-tree). Shared/core packs land
/// beneath it mirroring `adapters/shared/prose/rules/{universal,core}/`.
#[must_use]
pub fn codex_cache_root(project_dir: &Path) -> PathBuf {
    Layout::new(project_dir).cache_dir().join("codex")
}

/// Provenance for the distributed shared codex tree.
///
/// Stamped beside the cached rules so a consumer (and CI) can prove
/// which adapter source the codex was pinned to. Audit-only: the
/// codex resolver never reads it.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CodexMeta {
    /// The adapter source value the codex was copied from. Pins the
    /// codex to the same source as the project's target adapter.
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

/// Copy the shared codex packs from the checkout that built the
/// resolved adapter component into the project codex cache and stamp
/// [`CodexMeta`].
///
/// A development component lives inside its source checkout
/// (`<repo>/target/wasm32-wasip2/release/…`), so the shared codex is
/// found by walking the component's ancestors for a prose tree — the
/// engine-repo layout (`adapters/shared/prose/rules/`) or the adapters
/// repo layout (`shared/prose/rules/`). Whichever is found is mirrored
/// into the out-of-tree `<project-cache>/codex/` under the canonical
/// `adapters/shared/prose/rules/{universal,core}` layout the codex
/// resolver probes.
///
/// Returns `Ok(true)` when the codex was distributed, `Ok(false)` when
/// no ancestor carries a shared `universal/` pack — a fail-soft path so
/// init still succeeds. A registry-installed store entry has no source
/// checkout, so it always takes the `false` path; the consumer then
/// relies on `--rules-root` or a monorepo checkout (RFC-64: prose
/// distribution beyond the component is deferred).
pub(super) fn cache_codex(
    project_dir: &Path, source: &AdapterUri, include_framework: bool, now: Timestamp,
) -> Result<bool, Error> {
    let Some(rules_root) = rules_root_for_component(&source.component) else {
        return Ok(false);
    };

    let codex_root = codex_cache_root(project_dir);
    if codex_root.exists() {
        fs::remove_dir_all(&codex_root)?;
    }
    fs::create_dir_all(&codex_root)?;

    copy_dir_recursive(&rules_root.join("universal"), &codex_root.join(UNIVERSAL_RULES_REL))?;

    let core_src = rules_root.join("core");
    if include_framework && core_src.is_dir() {
        copy_dir_recursive(&core_src, &codex_root.join(CORE_RULES_REL))?;
    }

    write_codex_meta(project_dir, &source.adapter_value, include_framework, now)?;
    Ok(true)
}

/// Locate the shared-rules root (`…/prose/rules/`, carrying a
/// `universal/` pack) for a component file by walking its ancestors.
///
/// Probes each ancestor for the engine-repo layout
/// (`<base>/adapters/shared/prose/rules/`) and the adapters-repo layout
/// (`<base>/shared/prose/rules/`). The walk anchors on the component's
/// own checkout, so an adapter nested inside an unrelated outer repo
/// never adopts that repo's rules tree unless no inner match exists.
fn rules_root_for_component(component: &Path) -> Option<PathBuf> {
    for base in component.ancestors() {
        for candidate in [
            base.join("adapters").join("shared").join("prose").join("rules"),
            base.join("shared").join("prose").join("rules"),
        ] {
            if candidate.join("universal").is_dir() {
                return Some(candidate);
            }
        }
    }
    None
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

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<(), Error> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        // Follow directory symlinks and dereference file symlinks so the
        // cached codex is self-contained with real bytes.
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
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
