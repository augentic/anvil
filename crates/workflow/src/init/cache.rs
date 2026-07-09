//! Per-project component cache at init time.
//!
//! `cache_adapter` resolves the `<adapter>` argument (see
//! [`AdapterUri`]) and, for a local `.wasm` component outside the
//! resolver's probe set, mirrors it into the out-of-tree component
//! cache at `<project-cache>/components/<name>.wasm` — the project-local
//! leg the bare-name resolver probes first. Store entries (package
//! references) and development release builds are read in place and
//! never mirrored. Provenance is stamped in [`ComponentMeta`].

use std::fs;
use std::path::{Path, PathBuf};

use error::Error;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::adapter::component_cache_entry;
use crate::config::Layout;
use crate::init::adapter_uri::{AdapterOrigin, AdapterUri};

/// Provenance for the mirrored component under
/// `<project-cache>/components/`: the cache tenant carries its own
/// metadata inside its own tree.
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
