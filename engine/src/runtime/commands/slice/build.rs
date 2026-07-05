//! `specify slice build <slice> [--phase prepare|finalize]` handler —
//! target build envelope owner.
//!
//! Mirrors the shipped source two-phase agent contract
//! (`specify source survey` / `extract`). The verb resolves the target
//! from the slice's bound `metadata.yaml`, then owns request assembly,
//! report validation, the four `target-build-*` aborts, the
//! `slice.build.*` events, and the `built` transition gate; the target
//! `build` brief owns only code generation.
//!
//! - `execution: tool`: single-phase. Assemble + schema-validate the
//!   request, then dispatch the declared build tool. No first-party
//!   build tool exists yet, so the dispatch itself is a clear
//!   unsupported seam ([`dispatch_build_tool`]); the request-side
//!   aborts still fire so a future tool slots in behind the same flow.
//! - `execution: agent` (default): two-phase, and the CLI never blocks
//!   on agent work.
//!   - `--phase prepare` (default): assemble + schema-validate the
//!     request, write `.specify/slices/<slice>/build/request.yaml`, emit
//!     `target.execution.agent`, and print the handoff envelope. Control
//!     returns to the agent, which runs the `build` brief and writes
//!     `build/report.yaml`.
//!   - `--phase finalize`: frame entry with `slice.build.started`,
//!     validate the agent-produced report against
//!     `schemas/target/build-report.schema.json`, reject a `success`
//!     report carrying a blocking finding, gate the `Refined → Built`
//!     transition, and journal `slice.build.succeeded` /
//!     `slice.build.failed`. The `slice.build.*` pair brackets finalize
//!     (mirroring the `slice merge run` idiom), so a prepare-time abort
//!     never leaves a dangling `started`.
//!
//! Journal posture (mirroring `slice merge`): every
//! `slice.build.*` / `target.execution.agent` append is best-effort — a
//! journal-write failure is logged and swallowed so it can never change
//! the verb's exit code. A genuine build failure (failed report, schema
//! abort, gate rejection) still propagates so the exit stays non-zero.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use specify_diagnostics::Diagnostic;
use specify_dispatch::commands::slice::bracket;
use specify_dispatch::commands::source::cli::Phase;
use specify_error::{Error, Result};
use specify_registry::host::CapturedOutput;
use specify_workflow::adapter::{
    BuildInputDeclaration, Execution, ResolvedTargetAdapter, TargetAdapter, TargetOperation,
    extension_run_name,
};
use specify_workflow::init::adapter_ref_from_value;
use specify_workflow::journal::{self, EventKind};
use specify_workflow::schema::{validate_build_report_json, validate_build_request_json};
use specify_workflow::slice::{
    BuildReport, BuildRequest, BuildStatus, LifecycleStatus, SliceMetadata,
    actions as slice_actions, build_request, enforce_report_no_blocking_on_success,
    enforce_report_outputs_exist, evaluate_ui_surface_coherence, run_native_build_hook,
};

use crate::runtime::commands::extension;
use crate::runtime::context::Ctx;

/// Handoff envelope printed by the agent `prepare` phase. The agent
/// runs the `build` brief against `request`, then writes `report`
/// before calling back with `--phase finalize`.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct BuildHandoff {
    slice: String,
    target: String,
    /// Assembled, schema-validated build request the brief consumes.
    request: PathBuf,
    /// Expected output path the brief writes its build report to.
    report: PathBuf,
    /// Directory holding the target adapter's brief markdown.
    briefs_dir: PathBuf,
    /// The `build` brief the agent runs.
    build_brief: PathBuf,
    execution: &'static str,
}

/// Result of a completed `finalize` (or a future single-phase tool
/// run): the validated report's slice / target / status plus the
/// finding count and any non-blocking A4 coherence warnings.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct BuildResult {
    slice: String,
    target: String,
    status: BuildStatus,
    findings: usize,
    /// Non-blocking UI-surface coherence warnings (A4). Never alters the
    /// verb's exit code; surfaced in both the text and JSON output.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<Diagnostic>,
}

/// Run `specify slice build <slice> [--phase prepare|finalize]`.
///
/// # Errors
///
/// - propagates `metadata.yaml` load and `TargetAdapter::resolve`
///   failures.
/// - `target-build-input-missing` / `target-build-request-schema` from
///   request assembly + validation (both phases of the tool path and
///   the agent `prepare` phase).
/// - `target-build-report-missing` / `target-build-report-schema` /
///   `target-build-success-with-blocking-finding` /
///   `target-build-output-missing` /
///   `target-build-report-slice-mismatch` / `target-build-failed` and
///   the `lifecycle` gate error from the agent `finalize` phase.
/// - `target-build-host-prereq-missing` from a manifest-declared `host_prereq` hook at prepare.
/// - `target-build-materialize-failed` / `plan-bootstrap-app-icon-missing` from the manifest `prepare` extension hook dispatch.
/// - `target-build-verify-gate-failed` from a manifest-declared `finalize_verify` hook.
/// - `target-build-tool-unsupported` from the `execution: tool` seam.
/// - `adapter-execution-mode-required` when the manifest omits
///   `execution` (a shrunk post-cutover manifest carries no native
///   build machinery).
pub fn run(ctx: &Ctx, name: &str, phase: Phase) -> Result<()> {
    let slice_dir = ctx.slices_dir().join(name);
    let metadata = SliceMetadata::load(&slice_dir)?;
    let target_ref = adapter_ref_from_value(&metadata.target);
    let resolved = TargetAdapter::resolve(&target_ref, &ctx.project_dir)?;

    match resolved.manifest.execution {
        Some(Execution::Tool) => run_tool(ctx, name, &slice_dir, &resolved.manifest),
        Some(Execution::Agent) => match phase {
            Phase::Prepare => prepare(ctx, name, &slice_dir, &resolved),
            Phase::Finalize => finalize(ctx, name, &slice_dir, &resolved),
        },
        None => Err(Error::validation_failed(
            "adapter-execution-mode-required",
            "the native `slice build` dispatch requires the manifest's `execution` mode",
            format!(
                "target adapter `{}` omits `execution`; a shrunk post-cutover manifest carries \
                 no native build machinery, so run the build through the workflow guest instead",
                resolved.manifest.name
            ),
        )),
    }
}

/// Agent `prepare` phase: assemble + validate + persist the request,
/// emit `target.execution.agent`, and print the handoff envelope.
/// Returns without blocking on the agent's `build` run. The
/// `slice.build.started` frame is owned by `finalize`, not prepare, so
/// a prepare-time abort never leaves a dangling `started`.
fn prepare(
    ctx: &Ctx, name: &str, slice_dir: &Path, resolved: &ResolvedTargetAdapter,
) -> Result<()> {
    let manifest = &resolved.manifest;
    // Resolved up front so a briefless manifest aborts before the
    // request write, the hook dispatches, and the journal emit.
    let build_brief = build_brief_path(resolved)?;
    let request_path = assemble_and_write_request(ctx, name, slice_dir, &manifest.inputs)?;

    dispatch_host_prereq_hook(resolved, &ctx.project_dir, slice_dir)?;

    dispatch_prepare_hook(ctx, manifest, slice_dir)?;

    journal::emit_best_effort(
        ctx.layout(),
        ctx.now(),
        EventKind::TargetExecutionAgent {
            slice: name.into(),
            target: manifest.name.clone(),
        },
        "slice.build",
    );

    let briefs_dir =
        build_brief.parent().map_or_else(|| resolved.location.path().clone(), Path::to_path_buf);
    let handoff = BuildHandoff {
        slice: name.to_string(),
        target: manifest.name.clone(),
        request: request_path,
        report: report_path(slice_dir),
        briefs_dir,
        build_brief,
        execution: "agent",
    };
    ctx.write(&handoff, write_handoff_text)
}

/// Manifest-declared `host_prereq` script at prepare.
fn dispatch_host_prereq_hook(
    resolved: &ResolvedTargetAdapter, project_dir: &Path, slice_dir: &Path,
) -> Result<()> {
    let Some(hook) = &resolved.manifest.host_prereq else {
        return Ok(());
    };
    run_native_build_hook(
        resolved.location.path(),
        hook,
        project_dir,
        slice_dir,
        "target-build-host-prereq-missing",
        "host toolchain prerequisites are satisfied for declared platforms",
    )
}

/// Manifest-driven prepare hook: dispatch `extension run <tool> <argv…> <slice>`.
fn dispatch_prepare_hook(ctx: &Ctx, manifest: &TargetAdapter, slice_dir: &Path) -> Result<()> {
    let Some(prepare) = &manifest.prepare else {
        return Ok(());
    };
    let tool_name = extension_run_name(manifest).ok_or_else(|| {
        Error::validation_failed(
            "adapter-prepare-without-extension",
            "a target manifest with `prepare` must also declare `extension`",
            format!("target adapter `{}` declares `prepare` but omits `extension`", manifest.name),
        )
    })?;
    let rel = slice_dir.strip_prefix(&ctx.project_dir).map_or_else(
        |_| slice_dir.to_string_lossy().into_owned(),
        |p| p.to_string_lossy().into_owned(),
    );
    let mut args = prepare.argv.clone();
    args.push(rel);
    let captured = extension::run_captured(ctx, &tool_name, args)?;
    map_prepare_exit_code(&captured, &tool_name)
}

/// Map a non-zero prepare-hook guest outcome to host abort codes.
fn map_prepare_exit_code(captured: &CapturedOutput, tool_name: &str) -> Result<()> {
    if captured.exit_code == 0 {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&captured.stderr);
    let stdout = String::from_utf8_lossy(&captured.stdout);
    if let Ok(value) = serde_json::from_slice::<Value>(&captured.stdout) {
        if prepare_has_bootstrap_missing(&value) {
            return Err(Error::validation_failed(
                "plan-bootstrap-app-icon-missing",
                "prepare hook bootstrap app-icon gate passes before the build brief handoff",
                prepare_failure_detail(
                    &value,
                    tool_name,
                    captured.exit_code,
                    stderr.as_ref(),
                    stdout.as_ref(),
                ),
            ));
        }
        if prepare_has_materialize_errors(&value) {
            return Err(Error::validation_failed(
                "target-build-materialize-failed",
                "prepare hook materialize completes successfully before the build brief handoff",
                prepare_failure_detail(
                    &value,
                    tool_name,
                    captured.exit_code,
                    stderr.as_ref(),
                    stdout.as_ref(),
                ),
            ));
        }
    }

    let detail = if stderr.trim().is_empty() {
        stdout.trim().to_string()
    } else {
        stderr.trim().to_string()
    };
    Err(Error::validation_failed(
        "target-build-materialize-failed",
        "prepare hook completes successfully before the build brief handoff",
        format!("{tool_name} prepare hook exited with code {}: {detail}", captured.exit_code),
    ))
}

fn prepare_has_bootstrap_missing(value: &Value) -> bool {
    value
        .get("bootstrap_app_icon")
        .and_then(|b| b.get("findings"))
        .and_then(Value::as_array)
        .is_some_and(|findings| {
            findings.iter().any(|f| {
                f.get("id").and_then(Value::as_str) == Some("plan-bootstrap-app-icon-missing")
            })
        })
}

fn prepare_has_materialize_errors(value: &Value) -> bool {
    value
        .get("materialized")
        .and_then(|m| m.get("errors"))
        .and_then(Value::as_array)
        .is_some_and(|errors| !errors.is_empty())
}
fn prepare_failure_detail(
    value: &Value, tool_name: &str, exit_code: i32, stderr: &str, stdout: &str,
) -> String {
    if let Some(findings) = value.get("bootstrap_app_icon").and_then(|b| b.get("findings"))
        && let Some(messages) = finding_messages(findings)
    {
        return format!("{tool_name} prepare hook exited with code {exit_code}: {messages}");
    }
    if let Some(errors) = value.get("materialized").and_then(|m| m.get("errors"))
        && let Some(messages) = error_messages(errors)
    {
        return format!("{tool_name} prepare hook exited with code {exit_code}: {messages}");
    }
    let fallback = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
    format!("{tool_name} prepare hook exited with code {exit_code}: {fallback}")
}

fn finding_messages(findings: &Value) -> Option<String> {
    let arr = findings.as_array()?;
    let messages: Vec<_> =
        arr.iter().filter_map(|f| f.get("message").and_then(Value::as_str)).collect();
    (!messages.is_empty()).then(|| messages.join("; "))
}

fn error_messages(errors: &Value) -> Option<String> {
    let arr = errors.as_array()?;
    let messages: Vec<_> =
        arr.iter().filter_map(|e| e.get("message").and_then(Value::as_str)).collect();
    (!messages.is_empty()).then(|| messages.join("; "))
}

/// Agent `finalize` phase: validate the agent-produced report, gate the
/// `built` transition, and bracket the outcome with
/// `slice.build.succeeded` / `slice.build.failed`. Mirrors the
/// `slice merge run` lifecycle-pair idiom.
fn finalize(
    ctx: &Ctx, name: &str, slice_dir: &Path, resolved: &ResolvedTargetAdapter,
) -> Result<()> {
    let body = bracket(
        ctx,
        "slice.build",
        EventKind::SliceBuildStarted {
            slice_name: name.into(),
        },
        EventKind::SliceBuildSucceeded {
            slice_name: name.into(),
        },
        |reason| EventKind::SliceBuildFailed {
            slice_name: name.into(),
            reason,
        },
        || finalize_report(ctx, name, slice_dir, resolved),
    )?;
    ctx.write(&body, write_result_text)
}

/// Validate the report, enforce the success-blocking gate and the
/// output-existence gate, reject a failed report, and gate the
/// `Refined → Built` transition. Wrapped by [`finalize`] so the
/// `slice.build.*` pair brackets it.
fn finalize_report(
    ctx: &Ctx, name: &str, slice_dir: &Path, resolved: &ResolvedTargetAdapter,
) -> Result<BuildResult> {
    let project_dir: &Path = &ctx.project_dir;
    let raw = read_report(&report_path(slice_dir))?;
    validate_build_report_json(&raw)?;
    let report: BuildReport = serde_saphyr::from_str(&raw)?;

    if report.slice != name {
        return Err(Error::validation_failed(
            "target-build-report-slice-mismatch",
            "the build report's slice matches the slice being finalized",
            format!("report names slice `{}`, but finalize ran for `{name}`", report.slice),
        ));
    }

    enforce_report_no_blocking_on_success(&report)?;
    enforce_report_outputs_exist(&report, project_dir)?;
    if report.status == BuildStatus::Failure {
        return Err(Error::Diag {
            code: "target-build-failed",
            detail: format!(
                "target `{}` reported a failed build for slice `{name}` ({} finding(s))",
                report.target,
                report.findings.len()
            ),
        });
    }

    dispatch_finalize_verify_hook(resolved, &ctx.project_dir, slice_dir)?;

    slice_actions::transition(slice_dir, LifecycleStatus::Built, ctx.now())?;

    // A4 self-consistency: compare the brief-authored `ui_surface`
    // against the produced composition. These warnings are advisory —
    // they surface agent non-compliance one phase before the A3 merge
    // gate, and never gate the transition (already done) or the exit
    // code.
    let warnings = evaluate_ui_surface_coherence(&report, &slice_dir.join("composition.yaml"));

    Ok(BuildResult {
        slice: name.to_string(),
        target: report.target,
        status: report.status,
        findings: report.findings.len(),
        warnings,
    })
}

/// Manifest-declared `finalize_verify` script before the `built` transition.
fn dispatch_finalize_verify_hook(
    resolved: &ResolvedTargetAdapter, project_dir: &Path, slice_dir: &Path,
) -> Result<()> {
    let Some(hook) = &resolved.manifest.finalize_verify else {
        return Ok(());
    };
    run_native_build_hook(
        resolved.location.path(),
        hook,
        project_dir,
        slice_dir,
        "target-build-verify-gate-failed",
        "finalize host verify hook completes successfully",
    )
}

/// Single-phase `tool` execution: assemble + schema-validate the request
/// (so the request-side aborts still fire), then dispatch the declared
/// build tool. The dispatch itself is the unsupported seam.
fn run_tool(ctx: &Ctx, name: &str, slice_dir: &Path, manifest: &TargetAdapter) -> Result<()> {
    assemble_and_write_request(ctx, name, slice_dir, &manifest.inputs)?;
    dispatch_build_tool(manifest)
}

/// Dispatch the declared `build` WASI tool / built-in Rust path.
///
/// No first-party build tool exists yet; the WASI build dispatch
/// protocol is not yet wired (every first-party target
/// declares `execution: agent`). The surrounding control flow — request
/// assembly + schema validation in [`run_tool`], and the
/// validate / gate finalize tail shared with the agent path — is wired
/// correctly, so the only seam left is the actual tool invocation.
fn dispatch_build_tool(manifest: &TargetAdapter) -> Result<()> {
    Err(Error::Diag {
        code: "target-build-tool-unsupported",
        detail: format!(
            "target adapter `{}` declares `execution: tool`, but no `build` tool dispatch is \
             wired; no first-party target declares a build tool",
            manifest.name
        ),
    })
}

/// Assemble the build request from the bound adapter's declared inputs,
/// schema-validate the serialised envelope, and persist it atomically to
/// `.specify/slices/<slice>/build/request.yaml`. Returns the request
/// path. `project-dir` is `ctx.project_dir` (the resolved working tree,
/// the workspace clone in workspace mode); `inputs.root` is the slice
/// tree — both mirror how `slice synthesize` / `slice merge` derive
/// paths from a single `ctx.project_dir`.
fn assemble_and_write_request(
    ctx: &Ctx, name: &str, slice_dir: &Path, inputs: &[BuildInputDeclaration],
) -> Result<PathBuf> {
    let request = build_request(name, inputs, slice_dir, &ctx.project_dir)?;
    let yaml = serialise_request(&request)?;
    validate_build_request_json(&yaml)?;

    let build_dir = slice_dir.join("build");
    std::fs::create_dir_all(&build_dir).map_err(Error::Io)?;
    let request_path = build_dir.join("request.yaml");
    specify_model::atomic::bytes_write(&request_path, yaml.as_bytes())?;
    Ok(request_path)
}

/// Serialise the request to a trailing-newlined YAML document.
fn serialise_request(request: &BuildRequest) -> Result<String> {
    let mut content = serde_saphyr::to_string(request)?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    Ok(content)
}

/// `<slice_dir>/build/report.yaml`.
fn report_path(slice_dir: &Path) -> PathBuf {
    slice_dir.join("build").join("report.yaml")
}

/// Resolve the bound target adapter's `build` brief path.
fn build_brief_path(resolved: &ResolvedTargetAdapter) -> Result<PathBuf> {
    let brief_rel = resolved.manifest.briefs.get(&TargetOperation::Build).ok_or_else(|| {
        Error::validation_failed(
            "target-build-brief-missing",
            "the bound target adapter declares a build brief",
            format!("target adapter `{}` declares no `build` brief", resolved.manifest.name),
        )
    })?;
    Ok(resolved.location.path().join(brief_rel))
}

/// Read the `report.yaml` artifact, mapping a missing file to the
/// `target-build-report-missing` diagnostic.
fn read_report(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            Error::Diag {
                code: "target-build-report-missing",
                detail: format!(
                    "no `report.yaml` at {}; the build must write the report into the slice's \
                     `build/` directory before finalize",
                    path.display()
                ),
            }
        } else {
            Error::Io(err)
        }
    })
}

fn write_handoff_text(w: &mut dyn Write, body: &BuildHandoff) -> std::io::Result<()> {
    writeln!(w, "slice: {}", body.slice)?;
    writeln!(w, "target: {}", body.target)?;
    writeln!(w, "execution: {}", body.execution)?;
    writeln!(w, "request: {}", body.request.display())?;
    writeln!(w, "report: {}", body.report.display())?;
    writeln!(w, "briefs-dir: {}", body.briefs_dir.display())?;
    writeln!(w, "build-brief: {}", body.build_brief.display())
}

fn write_result_text(w: &mut dyn Write, body: &BuildResult) -> std::io::Result<()> {
    let status = match body.status {
        BuildStatus::Success => "success",
        BuildStatus::Failure => "failure",
    };
    writeln!(w, "slice: {}", body.slice)?;
    writeln!(w, "target: {}", body.target)?;
    writeln!(w, "status: {status}")?;
    writeln!(w, "findings: {}", body.findings)?;
    writeln!(w, "warnings: {}", body.warnings.len())?;
    for warning in &body.warnings {
        let code = warning.rule_id.as_deref().unwrap_or(&warning.id);
        writeln!(w, "  - {code}: {}", warning.impact)?;
    }
    Ok(())
}
