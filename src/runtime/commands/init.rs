use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::Serialize;
use specify_error::{Error, Result};
use specify_registry::store;
use specify_workflow::config::{ProjectConfig, is_slot};
use specify_workflow::hydrate;
use specify_workflow::init::{AdapterPackage, InitOptions, InitResult, init, recognize_package};
use specify_workflow::platform::parse_platforms_csv;
use specify_workflow::registry::Registry;
use specify_workflow::registry::workspace::{regenerate_topology_lock, sync_projects};

use crate::runtime::cli::Format;
use crate::runtime::commands::{agents, deploy};
use crate::runtime::context::Ctx;
use crate::runtime::output;

/// Display a path as the canonical absolute form when it exists; fall back
/// to the lossy display when it does not (e.g. a path we just deleted).
fn canonical(p: &Path) -> String {
    std::fs::canonicalize(p).map_or_else(|_| p.display().to_string(), |c| c.display().to_string())
}

/// Clap-mapped inputs for `specify init` (format-only handler).
pub(super) struct Args<'a> {
    pub format: Format,
    pub adapter: Option<&'a str>,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub workspace: bool,
    pub include_framework: bool,
    pub platforms: Option<&'a str>,
    pub upgrade: bool,
}

pub(super) fn run(args: &Args<'_>) -> Result<()> {
    let project_dir = PathBuf::from(".");

    let parsed_platforms =
        args.platforms.map(parse_platforms_csv).transpose().map_err(|e| Error::Argument {
            flag: "--platforms",
            detail: e,
        })?;

    hydrate_declared(args.adapter, &project_dir)?;

    let opts = InitOptions {
        project_dir: &project_dir,
        adapter: args.adapter,
        name: args.name,
        description: args.description,
        workspace: args.workspace,
        include_framework: args.include_framework,
        platforms: parsed_platforms.as_deref(),
        upgrade: args.upgrade,
    };

    let result = init(opts, Timestamp::now())?;
    // Regenerate the deployment manifest from the freshly hydrated +
    // scaffolded declared set (RFC-65: init is a manifest trigger).
    deploy::regenerate(&project_dir)?;
    let current_dir = std::env::current_dir().map_err(Error::Io)?;
    let context_skip_reason = generate_initial_context(args.format, &current_dir)?;

    let workspace_sync_message = if args.workspace && !args.upgrade {
        Some(run_workspace_sync(&project_dir)?)
    } else {
        None
    };

    emit_init_result(args.format, &result, context_skip_reason, workspace_sync_message)
}

/// Run the RFC-65 hydration kernel over the project's declared pinned
/// identities before the workflow init flow resolves them: the
/// positional `<adapter>` when it is a package reference (or the
/// versioned `<name>@<semver>` shorthand), plus — on re-entry, when
/// `.specify/project.yaml` already exists — the recorded `adapter:`
/// pin and the `project.yaml.adapters:` prefetch list. Non-package
/// forms (bare development names, local component paths) are left to
/// the workflow flow, so this is a no-op for a purely local project.
///
/// The fetch leg is `store::install_tofu` (trust-on-first-use through
/// the wasm-pkg transport, honouring `.specify/wasm-pkg.toml`); a warm
/// store makes the whole call a no-op probe per identity. Each resolved
/// entry is pinned in (and verified against) the committed
/// `.specify/adapters.lock` by the kernel.
fn hydrate_declared(adapter: Option<&str>, project_dir: &Path) -> Result<()> {
    let mut refs: Vec<AdapterPackage> = Vec::new();
    if let Some(adapter) = adapter
        && let Some(package) = recognize_package(adapter)
    {
        refs.push(package?);
    }
    match ProjectConfig::load(project_dir) {
        Ok(config) => refs.extend(hydrate::config_refs(&config)?),
        Err(Error::NotInitialized) => {}
        Err(err) => return Err(err),
    }
    let fetch = |package: &AdapterPackage| {
        let version = package.version.to_string();
        store::install_tofu(&package.namespace, &package.name, &version, project_dir)
            .map_err(Error::from)
    };
    hydrate::hydrate(project_dir, &refs, false, &fetch)?;
    Ok(())
}

/// Materialise registry slots and regenerate topology after workspace init.
/// Returns the human-readable sync outcome for the init envelope.
fn run_workspace_sync(project_dir: &Path) -> Result<String> {
    let registry = Registry::load(project_dir)?;
    let Some(reg) = registry.as_ref() else {
        return Ok("no registry declared at registry.yaml; nothing to sync".to_string());
    };
    let selected = reg.select(&[])?;
    sync_projects(project_dir, &selected)?;
    regenerate_topology_lock(project_dir, reg)?;
    Ok("workspace sync complete".to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "JSON wire DTO: each bool is a stable, independently consumed field on the init envelope."
)]
struct Body {
    config_path: String,
    /// Resolved adapter name (or `"workspace"` for workspace init — both
    /// renderers dispatch on this value).
    adapter_name: String,
    cache_present: bool,
    /// `true` when the shared codex is materialized in the out-of-tree
    /// `<project-cache>/codex/` (RM-07) — always, for regular init, the
    /// packs being embedded in the binary; `false` for workspace init.
    codex_present: bool,
    directories_created: Vec<String>,
    scaffolded_rule_keys: Vec<String>,
    specify_version: String,
    /// `true` when this run wrote `project.yaml.specify` — always
    /// `true` for fresh init and for an `--upgrade` that bumped an older
    /// pin; `false` on an `--upgrade` no-op where the pin already matched.
    /// Change G's re-entry template reads this to distinguish "upgraded"
    /// from "already current".
    specify_version_changed: bool,
    /// `true` when this run scaffolded `.specify/wasm-pkg.toml`. Stays
    /// `false` on re-init so consumers can distinguish a fresh write
    /// from a preserved operator-edited file.
    wasm_pkg_config_written: bool,
    context_generated: bool,
    context_skipped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    context_skip_reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_synced: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_sync_message: Option<String>,
}

fn write_text(w: &mut dyn Write, body: &Body) -> std::io::Result<()> {
    let is_workspace = body.adapter_name == "workspace";
    if is_workspace {
        writeln!(w, "Initialized .specify/ as a registry-only workspace")?;
    } else {
        writeln!(w, "Initialized .specify/")?;
    }
    writeln!(w, "  adapter: {}", body.adapter_name)?;
    writeln!(w, "  config: {}", body.config_path)?;
    writeln!(w, "  cache present: {}", body.cache_present)?;
    if !is_workspace {
        writeln!(w, "  codex present: {}", body.codex_present)?;
    }
    if !body.directories_created.is_empty() {
        writeln!(w, "  directories created: {}", body.directories_created.join(", "))?;
    }
    if body.specify_version_changed {
        writeln!(w, "  specify: {}", body.specify_version)?;
    } else {
        writeln!(w, "  specify: {} (already current)", body.specify_version)?;
    }
    if body.wasm_pkg_config_written {
        writeln!(w, "  wrote .specify/wasm-pkg.toml (edit to add registry mappings)")?;
    }
    if body.context_skipped && body.context_skip_reason == Some("existing-agents-md") {
        writeln!(w, "AGENTS.md already present; skipping context generate")?;
    }
    if let Some(message) = body.workspace_sync_message.as_deref() {
        writeln!(w, "  {message}")?;
    }
    writeln!(w)?;
    if is_workspace {
        writeln!(
            w,
            "Next: run `specify registry add <id> <url>` to declare projects, then `/spec:plan <name>`."
        )?;
    } else {
        writeln!(
            w,
            "Next: run `/spec:plan <name>` (the skill that authors `change.md` + `plan.yaml`), or — for a headless plan — `specify plan create <name>` followed by `specify plan add` and `specify plan transition <name> approved`."
        )?;
    }
    Ok(())
}

fn emit_init_result(
    format: Format, result: &InitResult, context_skip_reason: Option<&'static str>,
    workspace_sync_message: Option<String>,
) -> Result<()> {
    let workspace_synced = workspace_sync_message.as_ref().map(|msg| msg.contains("complete"));
    let body = Body {
        config_path: canonical(&result.config_path),
        adapter_name: result.adapter_name.clone(),
        cache_present: result.cache_present,
        codex_present: result.codex_present,
        directories_created: result.directories_created.iter().map(|p| canonical(p)).collect(),
        scaffolded_rule_keys: result.scaffolded_rule_keys.clone(),
        specify_version: result.specify_version.clone(),
        specify_version_changed: result.specify_version_changed,
        wasm_pkg_config_written: result.wasm_pkg_config_written,
        context_generated: context_skip_reason.is_none(),
        context_skipped: context_skip_reason.is_some(),
        context_skip_reason,
        workspace_synced,
        workspace_sync_message,
    };
    output::emit(&mut std::io::stdout().lock(), format, &body, write_text)?;
    Ok(())
}

/// Returns `None` when initial context generation ran, `Some(reason)` when it was skipped.
fn generate_initial_context(format: Format, project_dir: &Path) -> Result<Option<&'static str>> {
    if is_slot(project_dir) {
        return Ok(Some("workspace-clone"));
    }
    match project_dir.join("AGENTS.md").try_exists() {
        Ok(true) => return Ok(Some("existing-agents-md")),
        Ok(false) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => return Err(Error::Io(err)),
    }

    let config = ProjectConfig::load(project_dir)?;
    let ctx = Ctx {
        format,
        project_dir: project_dir.to_path_buf(),
        config,
        plan_dir: None,
    };
    let outcome = agents::generate_for_init(&ctx)?;
    debug_assert!(
        outcome.changed,
        "init context generation is called only when AGENTS.md is absent"
    );
    debug_assert_eq!(outcome.disposition, "create");
    Ok(None)
}
