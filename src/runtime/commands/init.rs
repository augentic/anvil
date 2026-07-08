use std::io::{ErrorKind, IsTerminal as _, Write};
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::Serialize;
use specify_error::{Error, Result};
use specify_registry::store;
use specify_workflow::config::{Layout, ProjectConfig, is_slot};
use specify_workflow::hydrate::{self, ResolvedAdapter};
use specify_workflow::init::{AdapterPackage, InitOptions, InitResult, init, recognize_package};
use specify_workflow::platform::{Platform, parse_platforms_csv};
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

    // Idempotent re-entry (RFC-65 §"Operator onboarding"): rerunning
    // the door over an initialized project is never an error — it
    // routes to `--upgrade` instead of re-scaffolding. Workspace init
    // keeps its own typed refusal.
    if !args.upgrade && !args.workspace && Layout::new(&project_dir).config_path().is_file() {
        return emit_reentry(args.format, &project_dir);
    }

    // Elicitation layer (RFC-65 §"Operator onboarding"): flags are the
    // substrate; a TTY prompt fills only the gaps, and every other
    // context gets a typed error naming the missing argument. Nothing
    // at or below the hydration kernel ever prompts.
    let interactive = std::io::stdin().is_terminal();
    let (adapter, adapter_prompted) = elicit_adapter(args, interactive)?;
    let mut platforms_csv: Option<String> = args.platforms.map(str::to_string);
    let mut parsed_platforms = parse_platforms(platforms_csv.as_deref())?;
    let mut platforms_prompted = false;

    let hydrated = hydrate_declared(adapter.as_deref(), &project_dir)?;

    let result = loop {
        let opts = InitOptions {
            project_dir: &project_dir,
            adapter: adapter.as_deref(),
            name: args.name,
            description: args.description,
            workspace: args.workspace,
            include_framework: args.include_framework,
            platforms: parsed_platforms.as_deref(),
            upgrade: args.upgrade,
        };
        match init(opts, Timestamp::now()) {
            Ok(result) => break result,
            // TTY prompt mode for the one requirement only the resolved
            // target can declare: `--platforms`. The prompt fills the
            // gap and the (idempotent) scaffold re-runs once; a second
            // failure propagates typed as always.
            Err(Error::Validation { code, detail })
                if code.as_ref() == "project-platforms-required"
                    && platforms_csv.is_none()
                    && interactive =>
            {
                let answer = prompt_line(&format!(
                    "{detail}\nPlatforms (comma-separated, e.g. `core,ios`): "
                ))?;
                if answer.is_empty() {
                    return Err(Error::Validation { code, detail });
                }
                parsed_platforms = parse_platforms(Some(&answer))?;
                platforms_csv = Some(answer);
                platforms_prompted = true;
            }
            Err(err) => return Err(err),
        }
    };
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

    // Teach the non-interactive form: when a prompt filled a gap, the
    // report ends with the equivalent fully-flagged invocation.
    let equivalent = (adapter_prompted || platforms_prompted)
        .then(|| equivalent_invocation(args, adapter.as_deref(), platforms_csv.as_deref()));

    emit_init_result(
        args.format,
        &result,
        context_skip_reason,
        workspace_sync_message,
        &hydrated,
        equivalent,
    )
}

/// The `--platforms` CSV parsed onto the closed [`Platform`] enum, or
/// the `--platforms` argument error.
fn parse_platforms(csv: Option<&str>) -> Result<Option<Vec<Platform>>> {
    csv.map(parse_platforms_csv).transpose().map_err(|e| Error::Argument {
        flag: "--platforms",
        detail: e,
    })
}

/// Resolve the required `<adapter>` argument: the flag when present, a
/// line prompt when stdin is a TTY, the typed `init-adapter-required`
/// (exit 2) everywhere else. Returns the value plus whether a prompt
/// supplied it. `--workspace` and `--upgrade` need no adapter.
fn elicit_adapter(args: &Args<'_>, interactive: bool) -> Result<(Option<String>, bool)> {
    if let Some(adapter) = args.adapter {
        return Ok((Some(adapter.to_string()), false));
    }
    if args.workspace || args.upgrade {
        return Ok((None, false));
    }
    if interactive {
        let answer = prompt_line("Target adapter (e.g. `omnia@1.0.0`, or a local `.wasm` path): ")?;
        if !answer.is_empty() {
            return Ok((Some(answer), true));
        }
    }
    Err(Error::validation_failed(
        "init-adapter-required",
        "specify init requires the target adapter",
        "pass it as the positional argument — `specify init <adapter>`, e.g. `specify init \
         omnia@1.0.0` — or run `specify init --workspace` for a registry-only workspace \
         (`specify init --upgrade` re-enters an initialized project)",
    ))
}

/// One interactive line prompt on the TTY path. The label goes to
/// stderr so stdout stays the clean envelope channel; the answer is
/// the trimmed stdin line.
fn prompt_line(label: &str) -> Result<String> {
    let mut err = std::io::stderr().lock();
    write!(err, "{label}").map_err(Error::Io)?;
    err.flush().map_err(Error::Io)?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).map_err(Error::Io)?;
    Ok(line.trim().to_string())
}

/// The fully-flagged `specify init` invocation equivalent to this run,
/// printed when a TTY prompt filled a missing argument.
fn equivalent_invocation(
    args: &Args<'_>, adapter: Option<&str>, platforms_csv: Option<&str>,
) -> String {
    let mut parts = vec!["specify".to_string(), "init".to_string()];
    if let Some(adapter) = adapter {
        parts.push(shell_word(adapter));
    }
    if let Some(name) = args.name {
        parts.push("--name".to_string());
        parts.push(shell_word(name));
    }
    if let Some(description) = args.description {
        parts.push("--description".to_string());
        parts.push(shell_word(description));
    }
    if let Some(platforms) = platforms_csv {
        parts.push("--platforms".to_string());
        parts.push(shell_word(platforms));
    }
    if args.include_framework {
        parts.push("--include-framework".to_string());
    }
    parts.join(" ")
}

/// Quote one argument for the printed equivalent invocation when it
/// carries whitespace or quotes; plain values pass through verbatim.
fn shell_word(value: &str) -> String {
    if value.chars().any(|c| c.is_whitespace() || c == '\'' || c == '"') {
        format!("'{}'", value.replace('\'', "'\\''"))
    } else {
        value.to_string()
    }
}

/// Run the RFC-65 hydration kernel over the project's declared pinned
/// identities before the workflow init flow resolves them: the core
/// guest `specify:core@<the binary's own version>` when no development
/// override resolves (RFC-65 move 4 — bootstrap order puts the core
/// first), the positional `<adapter>` when it is a package reference
/// (or the versioned `<name>@<semver>` shorthand), plus — on re-entry,
/// when `.specify/project.yaml` already exists (`--upgrade`) — the
/// recorded `adapter:` pin and the `project.yaml.adapters:` prefetch
/// list. Non-package forms (bare development names, local component
/// paths) are left to the workflow flow, so a purely local project
/// hydrates at most the core.
///
/// The fetch leg is `store::install_tofu` (trust-on-first-use through
/// the wasm-pkg transport, honouring `.specify/wasm-pkg.toml`); a warm
/// store makes the whole call a no-op probe per identity. Each resolved
/// entry is pinned in (and verified against) the committed
/// `.specify/adapters.lock` by the kernel. Returns the resolved set
/// for the postflight report.
fn hydrate_declared(adapter: Option<&str>, project_dir: &Path) -> Result<Vec<ResolvedAdapter>> {
    let mut refs: Vec<AdapterPackage> = Vec::new();
    if deploy::dev_core(project_dir)?.is_none() {
        refs.push(deploy::core_package());
    }
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
    hydrate::hydrate(project_dir, &refs, false, &fetch)
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

/// Wire body for the idempotent re-entry path: `specify init` over an
/// already-initialized project changes nothing and routes to
/// `--upgrade`.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct ReentryBody {
    already_initialized: bool,
    config_path: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    specify_version: Option<String>,
    /// The literal re-entry command.
    next: &'static str,
}

fn write_reentry_text(w: &mut dyn Write, body: &ReentryBody) -> std::io::Result<()> {
    write!(w, "Already initialized: {} (project '{}'", body.config_path, body.name)?;
    if let Some(version) = body.specify_version.as_deref() {
        write!(w, ", specify {version}")?;
    }
    writeln!(w, ")")?;
    writeln!(
        w,
        "Nothing changed — `specify init` never re-scaffolds an initialized project. Re-entry \
         bumps the `specify` pin and re-runs hydration over the declared set, preserving every \
         operator artifact."
    )?;
    writeln!(w)?;
    writeln!(w, "Next: run `{}`", body.next)?;
    Ok(())
}

/// Detect-and-route for an already-initialized project: report the
/// recorded identity and the literal `specify init --upgrade` re-entry
/// command, exit 0, write nothing.
fn emit_reentry(format: Format, project_dir: &Path) -> Result<()> {
    let config = ProjectConfig::load(project_dir)?;
    let body = ReentryBody {
        already_initialized: true,
        config_path: canonical(&Layout::new(project_dir).config_path()),
        name: config.name,
        specify_version: config.specify_version,
        next: "specify init --upgrade",
    };
    output::emit(&mut std::io::stdout().lock(), format, &body, write_reentry_text)?;
    Ok(())
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
    /// Pinned identities hydrated into the global adapter store by this
    /// run (`<name>@<version>`, bootstrap order — core first). Empty
    /// when every component resolved through a development override or
    /// a project-local file.
    hydrated: Vec<String>,
    /// Root of the global adapter store the hydrated entries live in.
    adapter_store: String,
    /// The literal next command for the operator.
    next: String,
    /// The equivalent fully-flagged invocation, present only when a
    /// TTY prompt filled a missing argument (teaches the
    /// non-interactive form).
    #[serde(skip_serializing_if = "Option::is_none")]
    equivalent: Option<String>,
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
    if body.hydrated.is_empty() {
        writeln!(w, "  hydrated: nothing (components resolved locally)")?;
    } else {
        writeln!(w, "  hydrated: {}", body.hydrated.join(", "))?;
    }
    writeln!(w, "  adapter store: {}", body.adapter_store)?;
    if body.wasm_pkg_config_written {
        writeln!(w, "  wrote .specify/wasm-pkg.toml (edit to add registry mappings)")?;
    }
    if body.context_skipped && body.context_skip_reason == Some("existing-agents-md") {
        writeln!(w, "AGENTS.md already present; skipping context generate")?;
    }
    if let Some(message) = body.workspace_sync_message.as_deref() {
        writeln!(w, "  {message}")?;
    }
    if let Some(equivalent) = body.equivalent.as_deref() {
        writeln!(w, "  rerun non-interactively: {equivalent}")?;
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
    workspace_sync_message: Option<String>, hydrated: &[ResolvedAdapter],
    equivalent: Option<String>,
) -> Result<()> {
    let workspace_synced = workspace_sync_message.as_ref().map(|msg| msg.contains("complete"));
    let is_workspace = result.adapter_name == "workspace";
    let next = if is_workspace {
        "specify registry add <id> <url>".to_string()
    } else {
        "/spec:plan <name>".to_string()
    };
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
        hydrated: hydrated
            .iter()
            .map(|adapter| format!("{}@{}", adapter.name, adapter.version))
            .collect(),
        adapter_store: specify_schema::cache::adapter_store_root().display().to_string(),
        next,
        equivalent,
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
