//! `specify adapters sync` — the explicit RFC-65 hydration trigger.
//!
//! Provisioning verb: it shares the hydration kernel with `specify
//! init` (`specify_workflow::hydrate`) but collects the *full*
//! declared set — the core guest `specify:core@<the binary's own
//! version>` when no development override resolves (RFC-65 move 4),
//! the `project.yaml.adapter` pin, the `adapters:` prefetch list, and
//! `plan.yaml` source pins via `hydrate::collect_refs` — then hydrates
//! it against the global store with the `store::install_tofu` fetch
//! leg and regenerates the deployment manifest from the resolved set
//! (`commands::deploy`). `--frozen` turns any would-be fetch into the
//! typed `adapter-not-installed` (exit 2).

use std::cell::Cell;
use std::io::Write;

use serde::Serialize;
use specify_error::{Error, Result};
use specify_registry::store;
use specify_workflow::hydrate::{self, ResolvedAdapter};
use specify_workflow::init::{AdapterPackage, CodexMeta, sync_codex};
use specify_workflow::journal::{self, Event, EventKind};

use crate::runtime::commands::deploy;
use crate::runtime::context::Ctx;

pub(super) fn sync(ctx: &Ctx, frozen: bool) -> Result<()> {
    let mut refs = Vec::new();
    if deploy::dev_core(&ctx.project_dir)?.is_none() {
        refs.push(deploy::core_package());
    }
    refs.extend(hydrate::collect_refs(&ctx.project_dir)?);
    let fetched = Cell::new(0_usize);
    let fetch = |package: &AdapterPackage| {
        fetched.set(fetched.get() + 1);
        let version = package.version.to_string();
        store::install_tofu(&package.namespace, &package.name, &version, &ctx.project_dir)
            .map_err(Error::from)
    };
    let resolved = hydrate::hydrate(&ctx.project_dir, &refs, frozen, &fetch)?;
    // The absorbed `rules sync` leg: re-materialize the embedded codex
    // packs into the per-project cache, preserving the recorded
    // `--include-framework` choice (a warm, version-matched cache is a
    // no-op probe).
    let include_framework =
        CodexMeta::load(&ctx.project_dir).is_some_and(|meta| meta.include_framework);
    sync_codex(&ctx.project_dir, include_framework, ctx.now())?;
    let manifest = deploy::regenerate(&ctx.project_dir)?;

    let fetched = fetched.get();
    if !resolved.is_empty() {
        let event = Event::new(
            ctx.now(),
            EventKind::AdaptersSynced {
                resolved: resolved.len(),
                fetched,
            },
        );
        journal::append_batch(ctx.layout(), std::slice::from_ref(&event))?;
    }
    ctx.write(&Body::new(frozen, fetched, &resolved, &manifest), write_text)
}

/// Wire-stable `specify adapters sync` envelope (text + JSON).
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct Body {
    /// Schema marker; `1` for this shape.
    version: u32,
    /// `true` when the sync ran with `--frozen` (nothing was fetched).
    frozen: bool,
    /// Count of identities resolved and digest-verified.
    resolved: usize,
    /// Count of identities pulled into the store on a miss.
    fetched: usize,
    /// Count of identities already present in the store.
    already_present: usize,
    /// The resolved set, one row per pinned identity, in declaration
    /// order.
    adapters: Vec<AdapterRow>,
    /// Path of the regenerated deployment manifest in the per-project
    /// cache.
    manifest: String,
}

/// One resolved identity in the [`Body::adapters`] set.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct AdapterRow {
    /// Kebab-case adapter name.
    name: String,
    /// Exact semver pin.
    version: String,
    /// The single-file store entry path.
    path: String,
    /// `sha256:<hex>` content digest of the entry.
    digest: String,
}

impl Body {
    fn new(
        frozen: bool, fetched: usize, resolved: &[ResolvedAdapter], manifest: &std::path::Path,
    ) -> Self {
        Self {
            version: 1,
            frozen,
            resolved: resolved.len(),
            fetched,
            already_present: resolved.len() - fetched,
            adapters: resolved.iter().map(AdapterRow::from).collect(),
            manifest: manifest.display().to_string(),
        }
    }
}

impl From<&ResolvedAdapter> for AdapterRow {
    fn from(adapter: &ResolvedAdapter) -> Self {
        Self {
            name: adapter.name.clone(),
            version: adapter.version.to_string(),
            path: adapter.path.display().to_string(),
            digest: adapter.digest.clone(),
        }
    }
}

fn write_text(w: &mut dyn Write, body: &Body) -> std::io::Result<()> {
    if body.adapters.is_empty() {
        writeln!(w, "No pinned adapter identities declared; nothing to hydrate.")?;
        return writeln!(w, "Deployment manifest: {}", body.manifest);
    }
    if body.fetched == 0 {
        writeln!(
            w,
            "Adapter store warm: {} identit{} verified, nothing fetched (no-op probe)",
            body.resolved,
            if body.resolved == 1 { "y" } else { "ies" },
        )?;
    } else {
        writeln!(
            w,
            "Synced {} identit{} ({} fetched, {} already present)",
            body.resolved,
            if body.resolved == 1 { "y" } else { "ies" },
            body.fetched,
            body.already_present,
        )?;
    }
    for adapter in &body.adapters {
        writeln!(
            w,
            "  {}@{}  {}  {}",
            adapter.name, adapter.version, adapter.path, adapter.digest
        )?;
    }
    writeln!(w, "Deployment manifest: {}", body.manifest)
}
