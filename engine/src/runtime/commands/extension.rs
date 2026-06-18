//! `specify extension *` dispatcher. Hosts the shared inventory-assembly
//! helpers (declared-tool merge, adapter resolution, manifest
//! validation) consumed by every per-subcommand handler.

pub mod cli;
mod dto;
mod fetch;
mod gc;
mod run;

use std::collections::{HashMap, HashSet};

pub(super) use fetch::run as fetch;
pub(super) use gc::run as gc;
pub(super) use run::{run, run_captured};
use specify_error::{Error, Result};
use specify_registry::load::{self};
use specify_registry::manifest::{
    Axis as ToolAxis, Extension, ExtensionManifest, ExtensionScope, ExtensionSource,
};
use specify_workflow::adapter::{ADAPTER_WASM_FILENAME, ResolvedTargetAdapter, TargetAdapter};
use specify_workflow::init::adapter_ref_from_value;

pub(super) use self::dto::{Inventory, ScopedTool};
use self::dto::{WarningRow, warning_row};
use crate::runtime::context::Ctx;

pub fn build_inventory(ctx: &Ctx) -> Result<Inventory> {
    let project_scope = ExtensionScope::Project {
        project_name: ctx.config.name.clone(),
    };
    validate_manifest_tools(&ctx.config.tools, &project_scope)?;
    let project_tools = load::project_tools(ctx.config.name.clone(), ctx.config.tools.clone());

    let mut scopes = vec![project_scope];
    let plugin_tools = resolve_project_adapter(ctx)?
        .map_or_else(Vec::new, |plugin| adapter_extension_scoped(&plugin, &mut scopes));

    let (merged, warnings) = load::merge_scoped(project_tools, plugin_tools);
    Ok(Inventory {
        tools: merged.into_iter().map(|(scope, tool)| ScopedTool { scope, tool }).collect(),
        warnings: warnings.into_iter().map(warning_row).collect(),
        scopes,
    })
}

/// Project the resolved target adapter's singular `extension`
/// declaration (RFC-48 D11) into a plugin-scope [`Extension`], sourcing
/// the WASI component from the committed `adapter.wasm` in the installed
/// adapter tree rather than a retired `tools.yaml` sidecar.
///
/// An adapter with no `extension` contributes nothing. The extension
/// rides the adapter's own semver identity (RFC-47), so its `version`
/// is the manifest version and its run handle defaults to the adapter
/// name when the declaration omits `name`.
fn adapter_extension_scoped(
    plugin: &ResolvedTargetAdapter, scopes: &mut Vec<ExtensionScope>,
) -> Vec<(ExtensionScope, Extension)> {
    let Some(declaration) = &plugin.manifest.extension else {
        return Vec::new();
    };
    let plugin_dir = plugin.location.path();
    let scope = ExtensionScope::Plugin {
        axis: ToolAxis::Target,
        plugin_slug: plugin.manifest.name.clone(),
        capability_dir: plugin_dir.clone(),
    };
    scopes.push(scope.clone());
    let extension = Extension {
        name: declaration.name.clone().unwrap_or_else(|| plugin.manifest.name.clone()),
        version: plugin.manifest.version.to_string(),
        source: ExtensionSource::LocalPath(plugin_dir.join(ADAPTER_WASM_FILENAME)),
        sha256: None,
        permissions: declaration.permissions.clone(),
    };
    vec![(scope, extension)]
}

fn resolve_project_adapter(ctx: &Ctx) -> Result<Option<ResolvedTargetAdapter>> {
    let Some(value) = ctx.config.adapter.as_deref() else {
        return Ok(None);
    };
    let adapter_ref = adapter_ref_from_value(value);
    TargetAdapter::resolve(&adapter_ref, &ctx.project_dir).map(Some)
}

fn validate_manifest_tools(tools: &[Extension], scope: &ExtensionScope) -> Result<()> {
    let manifest = ExtensionManifest {
        tools: tools.to_vec(),
    };
    // `validate_structure` returns one deterministic `violation`
    // diagnostic per failing rule (passing rules emit nothing), so an
    // empty vector means the manifest is structurally valid. Collapse
    // any failures into a single payload-free `Error::Validation` keyed
    // on the first rule id; per-row detail is joined into the message.
    let diagnostics = manifest.validate_structure(scope);
    let Some(first) = diagnostics.first() else {
        return Ok(());
    };
    let code = first.rule_id.clone().unwrap_or_else(|| "tool-manifest-invalid".to_string());
    let detail = diagnostics.iter().map(|d| d.impact.as_str()).collect::<Vec<_>>().join("; ");
    Err(Error::validation_failed(code, "declared extensions must satisfy structural rules", detail))
}

fn find<'a>(inventory: &'a Inventory, name: &str) -> Result<&'a ScopedTool> {
    inventory.tools.iter().find(|scoped| scoped.tool.name == name).ok_or_else(|| {
        Error::validation_failed(
            "tool-not-declared",
            "extension must be declared in project.yaml or the bound adapter",
            format!("tool not declared: {name}"),
        )
    })
}

fn select<'a>(inventory: &'a Inventory, name: Option<&str>) -> Result<Vec<&'a ScopedTool>> {
    match name {
        Some(name) => Ok(vec![find(inventory, name)?]),
        None => Ok(inventory.tools.iter().collect()),
    }
}

fn kept_by_scope(
    inventory: &Inventory,
) -> HashMap<ExtensionScope, HashSet<(String, String, String)>> {
    let mut kept: HashMap<ExtensionScope, HashSet<(String, String, String)>> =
        inventory.scopes.iter().cloned().map(|scope| (scope, HashSet::new())).collect();
    for scoped in &inventory.tools {
        kept.entry(scoped.scope.clone()).or_default().insert((
            scoped.tool.name.clone(),
            scoped.tool.version.clone(),
            scoped.tool.source.to_wire_string().into_owned(),
        ));
    }
    kept
}

fn emit_warnings_to_stderr(warnings: &[WarningRow]) {
    for warning in warnings {
        eprintln!("warning: {}: {}", warning.code, warning.message);
    }
}
