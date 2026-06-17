//! `specify lint project` handler.
//!
//! Composes the standards-layer pipeline:
//!
//! 1. Resolve `--slice` / `--artifact` scope per lint scope resolution.
//! 2. Build the resolved codex (`specify_standards::build_resolved_rules`)
//!    using the same artifact / language filters as the indexer so
//!    the resolved rule set and the scan set agree.
//! 3. Build the project `WorkspaceModel` (`lint::index::build`).
//! 4. Evaluate executable deterministic hints per rule, skipping
//!    `lint-mode: model-assisted` rules.
//! 5. Render the `DiagnosticReport` envelope via `specify_diagnostics::render`.
//! 6. Decide exit: any `critical | important` finding lands the
//!    process on `Exit::ValidationFailed` (code 2) per lint exit mapping.
//!
//! Every failure path routes through a closed `Error` variant so
//! `Exit::from(&Error)` lands on the lint exit mapping exit-code map. The mapping
//! tables live next to the helpers (`map_index_error`,
//! `map_hint_error`, `map_render_error`) and are pinned by table-driven
//! tests in `run_tests.rs`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use jiff::Timestamp;
use specify_diagnostics::{DiagnosticReport, Format as DiagnosticsFormat};
use specify_error::{Error, Result};
use specify_registry::host::{RunContext, WasiRunner};
use specify_registry::manifest::ExtensionScope;
use specify_standards::ResolveInputs;
use specify_standards::lint::ScanProfile;
use specify_standards::lint::eval::tool::{ToolOutput, ToolRunError, ToolRunner};
use specify_standards::lint::runner::PipelineConfig;
use specify_workflow::journal::LintScope;

use crate::output::{self, LintRun};
use crate::runtime::commands::extension::{Inventory, ScopedTool, build_inventory};
use crate::runtime::commands::lint::cli::ProjectArgs;
use crate::runtime::context::Ctx;

/// Handler entry point dispatched from `src/runtime/commands.rs`.
///
/// Assembles the project-surface pipeline config and hands it to the
/// shared [`output::run_lint`] kernel; output, journal, and exit
/// plumbing live there so this handler differs from `specify lint framework`
/// only in the config its `build_report` closure builds.
///
/// # Errors
///
/// Closed mapping per lint exit mapping — `map_index_error` /
/// `map_hint_error` from the pipeline, `map_render_error` /
/// `map_resolve_error` from the shared tail.
pub fn run(ctx: &Ctx, args: &ProjectArgs) -> Result<()> {
    let format: DiagnosticsFormat = args.output_format.into();
    output::run_lint(format, || build_report(ctx, args, format))
}

/// Assemble the project-surface inputs and config, then run the
/// shared pipeline + emit tail. Every `?` here is a pre-emit abort
/// that [`output::run_lint`] turns into the JSON fallback envelope.
fn build_report(
    ctx: &Ctx, args: &ProjectArgs, format: DiagnosticsFormat,
) -> Result<Option<DiagnosticReport>> {
    let started_at = Instant::now();
    let target = args.target.as_str();
    let slice = args.slice.as_deref();
    let artifacts = args.artifacts.as_slice();
    let artifact_set = compose_artifact_set(&ctx.project_dir, slice, artifacts)?;

    let inputs = ResolveInputs {
        project_dir: &ctx.project_dir,
        rules_root: args.rules_root.as_deref(),
        target_adapter: target,
        source_adapters: args.sources.as_slice(),
        artifact_paths: &artifact_set,
        languages: args.languages.as_slice(),
        include_deprecated: false,
        include_unmatched: false,
        include_core: args.include_core,
    };

    let tool_runner = WasiToolRunner::new(ctx)?;
    let cli_contract = crate::runtime::commands::contract::dump::build_contract();
    let config = PipelineConfig {
        profile: ScanProfile::Project,
        dump_model: args.dump_model,
        apply_ignore_directives: true,
        rule_filter: &[],
        tool_runner: &tool_runner,
        cli_contract: Some(&cli_contract),
    };

    let scope = LintScope {
        target: (!target.is_empty()).then(|| target.to_string()),
        slice: slice.map(str::to_string),
        // Populate `artifact` only when the scan was narrowed to
        // exactly one path; multi-artifact and full scans leave the
        // field `null` per the variant doc.
        artifact: (artifacts.len() == 1).then(|| artifacts[0].display().to_string()),
    };
    output::emit_lint_report(LintRun {
        inputs: &inputs,
        config: &config,
        format,
        layout: ctx.layout(),
        now: ctx.now(),
        scope,
        journal: true,
        command_label: "specify lint project",
        started_at,
        trailing_newline: true,
    })
}

/// Compose the `artifact_paths` vector handed to both the resolver
/// and the indexer per lint scope resolution.
///
/// - `--slice <name>` contributes every path listed under the slice's
///   `tasks.md` `Touches:` / `Produces:` headings plus the slice
///   directory `.specify/slices/<name>` itself.
/// - `--artifact <path>` entries are appended verbatim.
/// - Empty result means "full scan" — both the resolver and indexer
///   read `&[]` that way.
fn compose_artifact_set(
    project_dir: &Path, slice: Option<&str>, artifacts: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(slice_name) = slice {
        let tasks_path =
            project_dir.join(".specify").join("slices").join(slice_name).join("tasks.md");
        match fs::read_to_string(&tasks_path) {
            Ok(text) => out.extend(parse_slice_tasks_paths(&text)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::validation_failed(
                    "review-slice-tasks-missing",
                    format!("slice {slice_name} has no tasks.md"),
                    tasks_path.display().to_string(),
                ));
            }
            Err(err) => {
                return Err(Error::Filesystem {
                    op: "review-slice-read",
                    path: tasks_path,
                    source: err,
                });
            }
        }
        out.push(PathBuf::from(format!(".specify/slices/{slice_name}")));
    }
    out.extend(artifacts.iter().cloned());
    Ok(out)
}

/// Extract bullet-list paths from the `Touches:` / `Produces:`
/// headings of a slice's `tasks.md`.
///
/// Tolerates `- path` and `* path` bullet markers; ignores blank
/// lines and prose; stops collecting at the next `## ` heading.
fn parse_slice_tasks_paths(text: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("##") {
            let heading = rest.trim_start_matches(' ').trim();
            inside = matches!(heading.to_ascii_lowercase().as_str(), "touches" | "produces");
            continue;
        }
        if !inside {
            continue;
        }
        let bullet =
            trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")).map(str::trim);
        if let Some(path) = bullet
            && !path.is_empty()
        {
            out.push(PathBuf::from(path));
        }
    }
    out
}

/// `ToolRunner` impl bridging `specify-standards`'s standards-layer
/// trait to the runtime's declared WASI tool inventory.
///
/// Owns the inventory built from `project.yaml` + the active target
/// adapter's sidecar manifest so the `kind: tool` evaluator contract `is_declared` / `run`
/// contract resolves without re-walking on every hint. `run`
/// delegates to [`WasiRunner::run_captured`] which mirrors
/// [`WasiRunner::run`] but redirects stdout / stderr into capped
/// memory pipes so the host can fold a tool's `DiagnosticReport`
/// envelope into the scan output.
struct WasiToolRunner {
    project_dir: PathBuf,
    inventory: Inventory,
    runner: WasiRunner,
}

impl WasiToolRunner {
    fn new(ctx: &Ctx) -> Result<Self> {
        let inventory = build_inventory(ctx)?;
        let runner = WasiRunner::new()?;
        Ok(Self {
            project_dir: ctx.project_dir.clone(),
            inventory,
            runner,
        })
    }

    fn lookup<'a>(&'a self, name: &str) -> Option<&'a ScopedTool> {
        self.inventory.find(name)
    }
}

impl ToolRunner for WasiToolRunner {
    fn is_declared(&self, tool_name: &str) -> bool {
        self.lookup(tool_name).is_some()
    }

    fn run(
        &self, tool_name: &str, args: &[String], project_dir: &Path,
    ) -> std::result::Result<ToolOutput, ToolRunError> {
        let Some(scoped) = self.lookup(tool_name) else {
            return Err(ToolRunError::Runtime(format!("tool {tool_name} is not declared")));
        };
        let resolved = specify_registry::resolver::resolve(
            scoped.scope(),
            scoped.tool(),
            Timestamp::now(),
            &self.project_dir,
        )
        .map_err(|err| ToolRunError::Runtime(err.to_string()))?;
        let mut run_ctx = RunContext::new(project_dir, args.to_vec());
        if let ExtensionScope::Plugin { capability_dir, .. } = scoped.scope() {
            run_ctx = run_ctx.with_capability_dir(capability_dir);
        }
        let captured = self
            .runner
            .run_captured(&resolved, &run_ctx)
            .map_err(|err| ToolRunError::Runtime(err.to_string()))?;
        Ok(ToolOutput {
            stdout: captured.stdout,
            stderr: captured.stderr,
            exit_code: captured.exit_code,
        })
    }
}

#[cfg(test)]
#[path = "project_tests.rs"]
mod tests;
