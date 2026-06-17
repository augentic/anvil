//! `specify plugins {doctor, refresh}` — Cursor plugin-cache inspection
//! and invalidation.
//!
//! Bootstrap verbs: they operate on the Cursor plugin cache and the
//! marketplace manifest, never a `.specify/` project, so they use the
//! project-context-free [`dispatch`](super::dispatch) path and never
//! call [`ProjectConfig::load`]. `doctor` is read-only and only fails
//! on filesystem / marketplace-parse errors — drift is a finding, not
//! an error. `refresh` deletes the marketplace-scoped cache root after
//! `--yes`, journals `plugins.refreshed` into the discoverable project
//! root (if any), and prints a restart instruction.

pub mod cli;

use std::io::Write;
use std::path::Path;

use jiff::Timestamp;
use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::cmd::real_cmd;
use specify_workflow::config::{Layout, ProjectConfig};
use specify_workflow::journal::{self, Event, EventKind};
use specify_workflow::plugins::{self, DoctorReport, RefreshOutcome};

use crate::runtime::cli::Format;
use crate::runtime::commands::plugins::cli::PluginsAction;
use crate::runtime::output;

/// Restart instruction printed after a successful refresh.
const RESTART_NOTICE: &str =
    "Plugin cache cleared. Restart Cursor to repopulate from the marketplace.";

/// Dispatch the `specify plugins {doctor, refresh}` family.
pub(super) fn run(format: Format, action: PluginsAction) -> Result<()> {
    match action {
        PluginsAction::Doctor {
            project_dir,
            marketplace,
        } => doctor(format, &project_dir, marketplace.as_deref()),
        PluginsAction::Refresh {
            project_dir,
            marketplace,
            yes,
        } => refresh(format, &project_dir, marketplace.as_deref(), yes),
    }
}

/// Build and emit the read-only drift report.
fn doctor(format: Format, project_dir: &Path, marketplace: Option<&Path>) -> Result<()> {
    let marketplace_path = plugins::discover_marketplace(marketplace, project_dir)?;
    let manifest = plugins::load_marketplace(&marketplace_path)?;
    let cache_root = plugins::cache_root(&plugins::cursor_home()?, &manifest.name);
    let report = plugins::build_report(&marketplace_path, &manifest, &cache_root, &real_cmd)?;
    output::emit(&mut std::io::stdout().lock(), format, &report, write_doctor_text)?;
    Ok(())
}

/// Delete the marketplace-scoped cache, journal the deletion, and print
/// the restart notice.
fn refresh(
    format: Format, project_dir: &Path, marketplace: Option<&Path>, yes: bool,
) -> Result<()> {
    let marketplace_path = plugins::discover_marketplace(marketplace, project_dir)?;
    let manifest = plugins::load_marketplace(&marketplace_path)?;
    let cache_root = plugins::cache_root(&plugins::cursor_home()?, &manifest.name);

    if !yes {
        return Err(Error::Diag {
            code: "plugins-refresh-consent-required",
            detail: "refusing to clear the plugin cache without consent; pass --yes to apply"
                .to_string(),
        });
    }

    let outcome = plugins::refresh(&marketplace_path, &cache_root)?;
    let journaled = journal_refresh(&outcome)?;
    output::emit(
        &mut std::io::stdout().lock(),
        format,
        &RefreshBody::new(&outcome, journaled),
        write_refresh_text,
    )?;
    Ok(())
}

/// Journal `plugins.refreshed` into the CWD project, if one exists.
///
/// Appends the event when a `.specify/` root is discoverable from the
/// CWD and returns whether one was written. Mirrors `upgrade`'s
/// skip-silently-if-no-root rule.
fn journal_refresh(outcome: &RefreshOutcome) -> Result<bool> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let Some(root) = ProjectConfig::find_root(&cwd) else {
        return Ok(false);
    };
    let event = Event::new(
        Timestamp::now(),
        EventKind::PluginsRefreshed {
            deleted_paths: outcome.deleted_paths.iter().map(|p| p.display().to_string()).collect(),
            marketplace: outcome.marketplace.display().to_string(),
        },
    );
    journal::append_batch(Layout::new(&root), std::slice::from_ref(&event))?;
    Ok(true)
}

/// Wire-stable `specify plugins refresh` envelope (text + JSON).
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct RefreshBody {
    /// Schema marker; `1` for this shape.
    version: u32,
    /// Resolved marketplace file path.
    marketplace: String,
    /// Cache root that was deleted (or would have been).
    cache_root: String,
    /// Cache directories actually removed; empty when nothing existed.
    deleted_paths: Vec<String>,
    /// `true` when a `plugins.refreshed` journal event was written.
    journaled: bool,
    /// Operator-facing restart instruction.
    message: &'static str,
}

impl RefreshBody {
    fn new(outcome: &RefreshOutcome, journaled: bool) -> Self {
        Self {
            version: 1,
            marketplace: outcome.marketplace.display().to_string(),
            cache_root: outcome.cache_root.display().to_string(),
            deleted_paths: outcome.deleted_paths.iter().map(|p| p.display().to_string()).collect(),
            journaled,
            message: RESTART_NOTICE,
        }
    }
}

fn write_doctor_text(w: &mut dyn Write, report: &DoctorReport) -> std::io::Result<()> {
    writeln!(w, "marketplace: {}", report.marketplace.display())?;
    writeln!(w, "cache-root: {}", report.cache_root.display())?;
    for plugin in &report.plugins {
        let expected = plugin.expected_sha.as_deref().unwrap_or("null");
        let cached = plugin.cached_sha.as_deref().unwrap_or("null");
        writeln!(
            w,
            "  {} [{}] expected={expected} cached={cached}",
            plugin.name,
            plugin.status.as_str()
        )?;
    }
    let s = &report.summary;
    writeln!(
        w,
        "summary: ok={} drifted={} present={} missing={} extra={}",
        s.ok, s.drifted, s.present, s.missing, s.extra
    )?;
    Ok(())
}

fn write_refresh_text(w: &mut dyn Write, body: &RefreshBody) -> std::io::Result<()> {
    for path in &body.deleted_paths {
        writeln!(w, "removed {path}")?;
    }
    writeln!(w, "{}", body.message)
}
