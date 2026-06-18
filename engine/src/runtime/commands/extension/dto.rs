//! Response DTOs and row builders for `specify tool *` handlers.

use std::io::Write;

use serde::Serialize;
use specify_error::Result;
use specify_registry::cache::{self, Status as CacheStatus};
use specify_registry::manifest::{Extension, ExtensionScope, ExtensionScopeKind};

#[derive(Debug, Clone)]
pub struct ScopedTool {
    pub(super) scope: ExtensionScope,
    pub(super) tool: Extension,
}

impl ScopedTool {
    /// Borrow the resolved scope (project / plugin) the tool was
    /// declared in. Used by `commands::lint` to invoke a declared
    /// WASI tool with the right capability root.
    pub const fn scope(&self) -> &ExtensionScope {
        &self.scope
    }

    /// Borrow the tool record (name, version, source, permissions).
    pub const fn tool(&self) -> &Extension {
        &self.tool
    }
}

#[derive(Debug)]
pub struct Inventory {
    pub(super) tools: Vec<ScopedTool>,
    pub(super) warnings: Vec<WarningRow>,
    pub(super) scopes: Vec<ExtensionScope>,
}

impl Inventory {
    /// Look up the declared tool with the given `name`, or return
    /// `None` if no declaration matches. Project-scope declarations
    /// shadow plugin-scope declarations per the merge contract in
    /// [`specify_registry::load::merge_scoped`].
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&ScopedTool> {
        self.tools.iter().find(|scoped| scoped.tool.name == name)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) struct WarningRow {
    pub(super) code: &'static str,
    pub(super) name: String,
    pub(super) message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) struct ToolRow {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) source: String,
    pub(super) scope: ExtensionScopeKind,
    pub(super) scope_detail: String,
    pub(super) cache_status: CacheStatus,
    pub(super) cached_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) struct ToolFetchRow {
    #[serde(flatten)]
    pub(super) row: ToolRow,
    pub(super) fetched: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) struct FetchBody {
    pub(super) tools: Vec<ToolFetchRow>,
    pub(super) warnings: Vec<WarningRow>,
}

pub(super) fn write_fetch_text(w: &mut dyn Write, body: &FetchBody) -> std::io::Result<()> {
    if body.tools.is_empty() {
        writeln!(w, "No declared tools to fetch.")?;
    } else {
        for row in &body.tools {
            let action = if row.fetched { "fetched" } else { "cached" };
            writeln!(
                w,
                "{action}: {} {} [{}:{}] {}",
                row.row.name,
                row.row.version,
                row.row.scope,
                row.row.scope_detail,
                row.row.cached_path
            )?;
        }
    }
    write_warnings(w, &body.warnings)
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) struct GcBody {
    pub(super) removed: Vec<String>,
    pub(super) warnings: Vec<WarningRow>,
}

pub(super) fn write_gc_text(w: &mut dyn Write, body: &GcBody) -> std::io::Result<()> {
    writeln!(
        w,
        "Removed {} tool cache entrie(s) from current-project scopes.",
        body.removed.len()
    )?;
    for path in &body.removed {
        writeln!(w, "  {path}")?;
    }
    write_warnings(w, &body.warnings)
}

/// Render the `tool-name-collision` warning rows to the text writer.
/// JSON carries the same rows on each body's `warnings` field, so both
/// formats surface the notice through the renderer rather than a
/// format-branched stderr side channel.
pub(super) fn write_warnings(w: &mut dyn Write, warnings: &[WarningRow]) -> std::io::Result<()> {
    for warning in warnings {
        writeln!(w, "warning: {}: {}", warning.code, warning.message)?;
    }
    Ok(())
}

pub(super) fn row_for(scoped: &ScopedTool) -> Result<ToolRow> {
    let source = scoped.tool.source.to_wire_string().into_owned();
    let cache_status = cache_status_for(scoped)?;
    let cached_path = cache::module_path(&scoped.scope, &scoped.tool.name, &scoped.tool.version)?;
    let (scope, scope_detail) = scope_labels(&scoped.scope);
    Ok(ToolRow {
        name: scoped.tool.name.clone(),
        version: scoped.tool.version.clone(),
        source,
        scope,
        scope_detail,
        cache_status,
        cached_path: cached_path.display().to_string(),
    })
}

pub(super) fn cache_status_for(scoped: &ScopedTool) -> Result<CacheStatus> {
    Ok(cache::status(
        &scoped.scope,
        &scoped.tool.name,
        &scoped.tool.version,
        scoped.tool.source.to_wire_string().as_ref(),
        scoped.tool.sha256.as_deref(),
    )?)
}

fn scope_labels(scope: &ExtensionScope) -> (ExtensionScopeKind, String) {
    match scope {
        ExtensionScope::Project { project_name } => {
            (ExtensionScopeKind::Project, project_name.clone())
        }
        ExtensionScope::Plugin { plugin_slug, .. } => {
            (ExtensionScopeKind::Plugin, plugin_slug.clone())
        }
    }
}

pub(super) fn warning_row(name: String) -> WarningRow {
    WarningRow {
        code: "tool-name-collision",
        message: format!(
            "project-scope declaration for `{name}` overrides the plugin-scope declaration"
        ),
        name,
    }
}
