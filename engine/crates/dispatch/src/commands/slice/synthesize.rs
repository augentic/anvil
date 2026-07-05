//! `specify slice synthesize <slice>` handler — slice synthesis engine.
//!
//! The CLI cannot run the agent reconciliation step, so synthesis
//! splits into the same two mutually-exclusive modes the shipped
//! `specify plan propose` precedent uses:
//!
//! - `--dry-run` is read-only. It reads the slice's bound
//!   `evidence/<source>.yaml` and the resolved target `shape` brief and
//!   emits the `kind: inputs` envelope ([`SynthesisInputs`]) for the
//!   agent synthesis step. `--format json` prints the envelope verbatim;
//!   nothing is written. It emits the `slice.synthesize.agent` journal
//!   event — synthesis is always agent-dispatched, and the journal
//!   records the handoff.
//! - `--from <response.json>` is the only writer. It schema-gates the
//!   raw response bytes, deserialises the agent's
//!   [`SynthesisResponse`], resolves authority from the on-disk
//!   Evidence and any per-slice override, projects the kernel-owned
//!   fields into the single `model.yaml` ([`project`]), renders
//!   provenance lines into `specs/<domain>/spec.md` ([`render_spec_files`]),
//!   and persists the staged artifacts atomically. It emits
//!   `slice.synthesize.started` first, then `slice.synthesize.completed`
//!   on success, or `slice.synthesize.failed` on any error before the
//!   write commits.
//!
//! Passing neither mode fails with `slice-synthesize-mode-required`
//! (exit 2); the clap layer rejects passing both. Everything is computed
//! and validated in memory before the first write, so prior artifacts
//! stay intact on failure.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::adapter::{TargetAdapter, TargetOperation};
use specify_workflow::change::{Entry, Plan, resolve_topology};
use specify_workflow::config::ProjectConfig;
use specify_workflow::init::adapter_ref_from_value;
use specify_workflow::journal::{self, EventKind};
use specify_workflow::merge::MergeStrategy;
use specify_workflow::registry::Surface;
use specify_workflow::schema::validate_synthesis_json;
use specify_workflow::slice::{
    BaselineDomainDetail, BaselineIndex, ProjectionHeader, SliceMetadata, SynthesisInputs,
    SynthesisResponse, build_synthesis_inputs, persist_synthesized, project, read_evidence_index,
    read_source_inputs, synthesize_failure_reason,
};

use crate::context::Ctx;

/// Run `specify slice synthesize <slice> --dry-run | --from <response.json>`.
///
/// # Errors
///
/// - `slice-synthesize-mode-required` (exit 2) when neither `--dry-run`
///   nor `--from` is set.
/// - propagates every projection-kernel abort, the `synthesis-schema`
///   gate failure, response read / parse failures, and Evidence /
///   adapter resolution errors.
pub(super) fn run(ctx: &Ctx, name: &str, dry_run: bool, from: Option<&Path>) -> Result<()> {
    match (dry_run, from) {
        (true, None) => dry_run_inputs(ctx, name),
        (false, Some(path)) => from_response(ctx, name, path),
        // The clap `conflicts_with` guard makes `(true, Some(_))`
        // unreachable; return the mode error rather than risk a panic.
        (false, None) | (true, Some(_)) => Err(Error::validation_failed(
            "slice-synthesize-mode-required",
            "synthesize requires exactly one of --dry-run or --from",
            "pass exactly one of --dry-run or --from",
        )),
    }
}

/// `--dry-run`: assemble and emit the `kind: inputs` envelope. Reads
/// each bound source's Evidence and the target shape brief; writes
/// nothing and emits `slice.synthesize.agent`.
fn dry_run_inputs(ctx: &Ctx, name: &str) -> Result<()> {
    let slice_dir = ctx.slices_dir().join(name);
    let entry = load_entry(ctx, name)?;
    let sources = read_source_inputs(&slice_dir, &entry)?;
    let shape_brief = resolve_shape_brief(ctx, &slice_dir)?;
    let baseline_specs_dir = resolve_baseline_specs_dir(ctx, &slice_dir);
    let baseline_index = BaselineIndex::build(&baseline_specs_dir)?;
    let baseline = baseline_surface(ctx, &entry)?;
    let baseline_detail: Vec<BaselineDomainDetail> = (&baseline_index).into();
    let inputs = build_synthesis_inputs(name, &sources, &shape_brief, &baseline, &baseline_detail);

    // Synthesis is always agent-dispatched — record the handoff.
    emit(
        ctx,
        EventKind::SliceSynthesizeAgent {
            slice_name: name.into(),
        },
    );
    ctx.write(&inputs, write_inputs_text)
}

/// `--from`: schema-gate, project, render, and persist the agent
/// response, framed by the paired `started` / `completed` / `failed`
/// journal events.
fn from_response(ctx: &Ctx, name: &str, response_path: &Path) -> Result<()> {
    emit(
        ctx,
        EventKind::SliceSynthesizeStarted {
            slice_name: name.into(),
        },
    );
    match synthesize_from(ctx, name, response_path) {
        Ok(written) => {
            emit(
                ctx,
                EventKind::SliceSynthesizeCompleted {
                    slice_name: name.into(),
                    artifacts: written.clone(),
                },
            );
            let summary = SynthesizeSummary {
                slice: name.to_string(),
                artifacts: written,
            };
            ctx.write(&summary, write_summary_text)
        }
        Err(err) => {
            // The failed event is best-effort so a journal hiccup can
            // never shadow the synthesis error itself.
            emit(
                ctx,
                EventKind::SliceSynthesizeFailed {
                    slice_name: name.into(),
                    reason: synthesize_failure_reason(&err),
                },
            );
            Err(err)
        }
    }
}

/// The schema-gate → project → render → persist pipeline, returning the
/// relative paths written (in write order). Every step runs in memory
/// before the first write, so a failure leaves prior artifacts intact.
fn synthesize_from(ctx: &Ctx, name: &str, response_path: &Path) -> Result<Vec<String>> {
    let slice_dir = ctx.slices_dir().join(name);
    let entry = load_entry(ctx, name)?;

    // Step 1 — schema-gate the raw bytes (the schema enforces the
    // kebab/const/`$ref` constraints the typed DTO does not), then
    // deserialise.
    let raw = read_response_file(response_path)?;
    validate_synthesis_json(&raw)?;
    let response: SynthesisResponse = serde_saphyr::from_str(&raw).map_err(|err| {
        Error::validation_failed(
            "slice-synthesize-response-parse",
            "the --from response deserialises as a synthesis response",
            format!("failed to parse synthesis response: {err}"),
        )
    })?;

    // Step 2 — resolve authority from on-disk Evidence and the per-slice
    // override, then project the kernel-owned fields.
    let (authority, evidence_claims) = read_evidence_index(&slice_dir, &entry)?;
    let overrides = entry.authority_override.by_kind.clone();
    let baseline_specs_dir = resolve_baseline_specs_dir(ctx, &slice_dir);
    let baseline_index = BaselineIndex::build(&baseline_specs_dir)?;
    let header = ProjectionHeader {
        version: 1,
        slice: name.to_string(),
        project: entry.project,
    };
    let projected =
        project(response.model, header, &authority, &overrides, &evidence_claims, &baseline_index)?;

    // Steps 3–5 — re-validate, render, stage, and atomically persist
    // through the shared tail (also driven by the guest refine
    // orchestrator).
    persist_synthesized(&slice_dir, response.artifacts, &projected, &baseline_index)
}

/// `--from` success summary. `--format json` emits this verbatim.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct SynthesizeSummary {
    slice: String,
    artifacts: Vec<String>,
}

/// Load the named slice's plan entry — the binding that carries the
/// slice's bound `sources[]`, `project`, and per-slice
/// `authority-override`.
fn load_entry(ctx: &Ctx, name: &str) -> Result<Entry> {
    let plan_path = ctx.layout().plan_path();
    if !plan_path.exists() {
        return Err(Error::validation_failed(
            "slice-synthesize-plan-missing",
            "synthesize reads the slice's bound sources from plan.yaml",
            format!(
                "no plan.yaml at {}; synthesize binds a slice's sources through its plan entry",
                plan_path.display()
            ),
        ));
    }
    let plan = Plan::load(&plan_path)?;
    plan.entries.into_iter().find(|e| e.name == name).ok_or_else(|| {
        Error::validation_failed(
            "slice-synthesize-entry-missing",
            "the slice has a matching plan entry",
            format!("plan.yaml has no entry named `{name}`"),
        )
    })
}

/// Resolve the `ThreeWayMerge` baseline `specs/` directory — the same path
/// merge and `slice touched-specs --scan` use.
fn resolve_baseline_specs_dir(ctx: &Ctx, slice_dir: &Path) -> PathBuf {
    let classes = super::artifact_classes(&ctx.project_dir, slice_dir);
    classes.iter().find(|class| matches!(class.strategy, MergeStrategy::ThreeWayMerge)).map_or_else(
        || ctx.layout().specify_dir().join("specs"),
        |class| class.baseline_dir.clone(),
    )
}

/// RFC-46 D5 — project the slice's bound-project baseline surface for
/// the synthesis inputs envelope.
///
/// Resolves the project topology and returns the bound project's
/// `surface` (one entry per `.specify/specs/<domain>/spec.md`). Binding
/// mirrors the kernel: an explicit `entry.project` selects by name, an
/// omitted project auto-binds the sole topology project. Baseline is
/// advisory context, so any resolution miss (multi-project plan with no
/// explicit binding, unknown project) degrades to an empty surface
/// rather than failing the dry-run.
fn baseline_surface(ctx: &Ctx, entry: &Entry) -> Result<Vec<Surface>> {
    let config = ProjectConfig::load(&ctx.project_dir)?;
    let topology = resolve_topology(&config, &ctx.project_dir)?;
    let bound = match entry.project.as_deref() {
        Some(name) => topology.iter().find(|p| p.name == name),
        None if topology.len() == 1 => topology.first(),
        None => None,
    };
    Ok(bound.map(|p| p.surface.clone()).unwrap_or_default())
}

/// Resolve the bound target's `shape` brief body — `TargetAdapter::resolve`
/// keeps target resolution a CLI responsibility.
fn resolve_shape_brief(ctx: &Ctx, slice_dir: &Path) -> Result<String> {
    let metadata = SliceMetadata::load(slice_dir)?;
    let adapter_ref = adapter_ref_from_value(&metadata.target);
    let resolved = TargetAdapter::resolve(&adapter_ref, &ctx.project_dir)?;
    let brief_rel = resolved.manifest.briefs.get(&TargetOperation::Shape).ok_or_else(|| {
        Error::validation_failed(
            "slice-synthesize-shape-brief-missing",
            "the bound target adapter declares a shape brief",
            format!("target adapter `{}` declares no `shape` brief", adapter_ref.name),
        )
    })?;
    let brief_path = resolved.location.path().join(brief_rel);
    std::fs::read_to_string(&brief_path).map_err(|err| Error::Filesystem {
        op: "read",
        path: brief_path,
        source: err,
    })
}

/// Read the `--from` response file, mapping a missing file to an exit-2
/// validation error rather than a generic I/O failure.
fn read_response_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            Error::validation_failed(
                "slice-synthesize-response-not-found",
                "the --from response file must exist",
                format!("no response file at {}", path.display()),
            )
        } else {
            Error::Io(err)
        }
    })
}

/// Best-effort emit of a single `slice.synthesize.*` journal event.
fn emit(ctx: &Ctx, kind: EventKind) {
    journal::emit_best_effort(ctx.layout(), ctx.now(), kind, "slice.synthesize");
}

fn write_inputs_text(w: &mut dyn Write, inputs: &SynthesisInputs) -> std::io::Result<()> {
    writeln!(w, "slice: {}", inputs.slice)?;
    writeln!(w, "sources:")?;
    for source in &inputs.sources {
        writeln!(w, "  - {} ({}): {} claim(s)", source.source, source.lead, source.claims.len())?;
    }
    if !inputs.baseline.is_empty() {
        writeln!(w, "baseline: {} domain(s)", inputs.baseline.len())?;
    }
    writeln!(w, "shape-brief: {} bytes", inputs.shape_brief.len())
}

fn write_summary_text(w: &mut dyn Write, summary: &SynthesizeSummary) -> std::io::Result<()> {
    writeln!(w, "slice: {}", summary.slice)?;
    writeln!(w, "artifacts:")?;
    for artifact in &summary.artifacts {
        writeln!(w, "  - {artifact}")?;
    }
    Ok(())
}
