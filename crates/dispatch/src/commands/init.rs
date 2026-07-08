//! `specify init --scaffold-only` — the guest-invocable scaffold leg
//! (guest routing).
//!
//! The scaffold writes project-scoped state only — `.specify/`,
//! `project.yaml`, `registry.yaml` (workspace mode), `.gitignore`
//! lines, and the per-project derived cache tenants (component mirror,
//! codex packs) — so it runs on both sides of the seam. Everything
//! the full native `init` adds around it (hydration, deployment-
//! manifest generation, `AGENTS.md` context generation, the workspace
//! sync chain) stays native: the provisioning front invokes this leg
//! through the host form after hydration. The flag is hidden from
//! operator-facing help; the forwarding form (stage C.3) calls it
//! verbatim.

use std::io::Write;
use std::path::Path;

use jiff::Timestamp;
use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow_lib::init::{InitOptions, InitResult, init};
use specify_workflow_lib::platform::parse_platforms_csv;

use crate::output::{self, Format};

/// Clap-mapped inputs for the scaffold leg — the `init` argument
/// surface minus nothing: the same flags parse, only the provisioning
/// legs are absent.
#[derive(Debug)]
pub struct ScaffoldArgs<'a> {
    /// Output format for the scaffold envelope.
    pub format: Format,
    /// Adapter identifier recorded on `project.yaml.adapter`.
    pub adapter: Option<&'a str>,
    /// Project name override.
    pub name: Option<&'a str>,
    /// Project description.
    pub description: Option<&'a str>,
    /// Scaffold a registry-only workspace.
    pub workspace: bool,
    /// Also materialize the framework `core/` codex pack.
    pub include_framework: bool,
    /// Raw `--platforms` CSV.
    pub platforms: Option<&'a str>,
}

/// Run the scaffold leg against `project_dir` (`"."` on both sides:
/// the guest's mount preopen, the native process CWD).
///
/// # Errors
///
/// Propagates argument-shape failures (`--platforms` parse), the
/// workflow init errors (adapter resolution, platform validation,
/// filesystem), and stdout serialisation errors.
pub fn scaffold(project_dir: &Path, args: &ScaffoldArgs<'_>) -> Result<()> {
    let parsed_platforms =
        args.platforms.map(parse_platforms_csv).transpose().map_err(|e| Error::Argument {
            flag: "--platforms",
            detail: e,
        })?;

    let opts = InitOptions {
        project_dir,
        adapter: args.adapter,
        name: args.name,
        description: args.description,
        workspace: args.workspace,
        include_framework: args.include_framework,
        platforms: parsed_platforms.as_deref(),
        upgrade: false,
    };
    let result = init(opts, Timestamp::now())?;
    emit_scaffold_result(args.format, &result)
}

/// Display a path as the canonical absolute form when it exists; fall
/// back to the lossy display when it does not.
fn canonical(p: &Path) -> String {
    std::fs::canonicalize(p).map_or_else(|_| p.display().to_string(), |c| c.display().to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct Body {
    config_path: String,
    /// Resolved adapter name (or `"workspace"` for workspace init).
    adapter_name: String,
    cache_present: bool,
    codex_present: bool,
    directories_created: Vec<String>,
    scaffolded_rule_keys: Vec<String>,
    specify_version: String,
    wasm_pkg_config_written: bool,
}

fn write_text(w: &mut dyn Write, body: &Body) -> std::io::Result<()> {
    if body.adapter_name == "workspace" {
        writeln!(w, "Scaffolded .specify/ as a registry-only workspace")?;
    } else {
        writeln!(w, "Scaffolded .specify/")?;
    }
    writeln!(w, "  adapter: {}", body.adapter_name)?;
    writeln!(w, "  config: {}", body.config_path)?;
    writeln!(w, "  specify: {}", body.specify_version)?;
    Ok(())
}

fn emit_scaffold_result(format: Format, result: &InitResult) -> Result<()> {
    let body = Body {
        config_path: canonical(&result.config_path),
        adapter_name: result.adapter_name.clone(),
        cache_present: result.cache_present,
        codex_present: result.codex_present,
        directories_created: result.directories_created.iter().map(|p| canonical(p)).collect(),
        scaffolded_rule_keys: result.scaffolded_rule_keys.clone(),
        specify_version: result.specify_version.clone(),
        wasm_pkg_config_written: result.wasm_pkg_config_written,
    };
    output::emit(&mut std::io::stdout().lock(), format, &body, write_text)?;
    Ok(())
}
