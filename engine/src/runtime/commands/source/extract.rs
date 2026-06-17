//! `specify source extract` handler — slice-time Evidence extraction.
//!
//! Resolves `<source>` against `plan.yaml.sources.<key>`, runs the
//! shared [`prep`] seam ([`prep::SourceOp::Extract`]) for adapter
//! resolution, brief directory, the four-root sandbox (scratch at
//! `.specify/scratch/<adapter>/<slice>/`), and the
//! `evidence/` output target under `.specify/slices/<slice>/`. Source
//! extraction is agent-only and two-phase; the CLI never blocks on
//! agent work:
//!
//! - `--phase prepare` (default): build scratch + the `evidence/`
//!   target, emit `source.execution.agent`, and print the extract
//!   handoff envelope (`{ adapter, version, briefs-dir, source-dir? |
//!   value-inline?, scratch-dir, evidence-dir, leads:[<lead>],
//!   execution }`). For value-bound sources (e.g. `intent`)
//!   `source-dir` is absent and `value-inline` carries the literal
//!   binding body. Control returns to the agent.
//! - `--phase finalize`: validate the agent-produced Evidence
//!   against `schemas/evidence.schema.json` *before* it becomes
//!   visible to synthesis, persist it to
//!   `.specify/slices/<slice>/evidence/<source>.yaml`, and emit
//!   `slice.extract.completed`. A validation failure returns early —
//!   no Evidence is persisted and the slice stays `refining`.
//!
//! The agent writes its Evidence to `$SCRATCH_DIR/evidence.yaml` (the
//! write-only sandbox root, mirroring how `survey` writes
//! `leads.md`); the CLI is the only writer of the visible
//! `.specify/slices/<slice>/evidence/<source>.yaml`, so an invalid
//! document never lands on the persisted path.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use specify_error::{Error, Result};
use specify_model::atomic::bytes_write;
use specify_workflow::adapter::SourceOperation;
use specify_workflow::change::Plan;
use specify_workflow::journal::EventKind;
use specify_workflow::schema;

use crate::runtime::commands::source::cli::Phase;
use crate::runtime::commands::source::{op, prep};
use crate::runtime::context::Ctx;

/// Extract handoff envelope printed by the agent `prepare` phase.
/// Carries `evidence-dir` (the slice's `evidence/`
/// target) and exactly one source representation: `source-dir` for
/// path bindings, `value-inline` for value bindings (e.g. `intent`).
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct ExtractHandoff {
    adapter: String,
    version: semver::Version,
    briefs_dir: PathBuf,
    /// `$SOURCE_DIR` — present for path bindings, absent for
    /// value-bound sources.
    #[serde(skip_serializing_if = "Option::is_none")]
    source_dir: Option<PathBuf>,
    /// Literal `value:` body — present for value-bound sources, absent
    /// for path bindings. The value half of the minimal two-field
    /// source request.
    #[serde(skip_serializing_if = "Option::is_none")]
    value_inline: Option<String>,
    scratch_dir: PathBuf,
    /// `.specify/slices/<slice>/evidence/` — where the CLI persists
    /// the validated `<source>.yaml` in finalize.
    evidence_dir: PathBuf,
    /// The single lead being extracted.
    leads: Vec<String>,
    execution: &'static str,
}

/// Result of a completed extract (agent `finalize`): the persisted
/// Evidence path.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct ExtractResult {
    adapter: String,
    source: String,
    slice: String,
    lead: String,
    /// Persisted `.specify/slices/<slice>/evidence/<source>.yaml`.
    evidence: PathBuf,
}

/// Run `specify source extract <source> <lead> --slice <slice>
/// [--phase prepare|finalize]`.
///
/// # Errors
///
/// - `source-unknown` when `<source>` is not a
///   `plan.yaml.sources` key.
/// - propagates adapter-resolution, schema-validation, and persist
///   failures.
pub fn run(ctx: &Ctx, source: &str, lead: &str, slice: &str, phase: Phase) -> Result<()> {
    let plan = Plan::load(&ctx.layout().plan_path())?;
    let binding = plan.sources.get(source).ok_or_else(|| Error::Diag {
        code: "source-unknown",
        detail: format!(
            "no source `{source}` in plan.yaml.sources; `specify source extract` resolves \
             its argument against the plan's source keys, not the adapter name"
        ),
    })?;

    let source_path =
        binding.path.as_deref().map(|raw| prep::resolve_source_path(ctx.layout().plan_dir(), raw));

    let slice_dir = ctx.slices_dir().join(slice);
    let leads = [lead.to_string()];
    let prepared = prep::prepare(&prep::PrepRequest {
        adapter: &binding.adapter,
        version: binding.version.clone(),
        project_dir: &ctx.project_dir,
        op: prep::SourceOp::Extract {
            slice: slice.to_string(),
        },
        source: source_path.as_deref(),
        leads: &leads,
        evidence_root: Some(&slice_dir),
    })?;

    let flow = ExtractFlow {
        common: op::Common {
            ctx,
            source,
            prepared: &prepared,
            source_path: source_path.as_deref(),
            binding,
            operation: SourceOperation::Extract,
        },
        lead,
        slice,
    };
    op::run(&flow, phase)
}

/// Extract's operation-specific seam onto the shared [`op::run`] flow:
/// the handoff carries `evidence-dir` (and `source-dir` xor
/// `value-inline`), the commit validates then persists the Evidence,
/// and the completion event is `slice.extract.completed`.
struct ExtractFlow<'a> {
    common: op::Common<'a>,
    lead: &'a str,
    slice: &'a str,
}

impl<'a> op::Flow<'a> for ExtractFlow<'a> {
    type Handoff = ExtractHandoff;
    type Outcome = ExtractResult;

    fn common(&self) -> &op::Common<'a> {
        &self.common
    }

    fn handoff(&self, scratch: PathBuf) -> Result<ExtractHandoff> {
        let c = &self.common;
        let (source_dir, value_inline) = c.source_path.map_or_else(
            || (None, c.binding.value.clone()),
            |path| (Some(path.to_path_buf()), None),
        );
        Ok(ExtractHandoff {
            adapter: c.prepared.manifest.name.clone(),
            version: c.prepared.manifest.version.clone(),
            briefs_dir: c.prepared.briefs_dir.clone(),
            source_dir,
            value_inline,
            scratch_dir: scratch,
            evidence_dir: evidence_dir(c.prepared)?,
            leads: vec![self.lead.to_string()],
            execution: "agent",
        })
    }

    fn write_handoff(w: &mut dyn Write, body: &ExtractHandoff) -> std::io::Result<()> {
        write_handoff_text(w, body)
    }

    fn commit(&self, raw: &str, artifact_source: &Path) -> Result<ExtractResult> {
        let c = &self.common;
        schema::validate_evidence(raw, artifact_source)?;
        let path = evidence_dir(c.prepared)?.join(format!("{}.yaml", c.source));
        bytes_write(&path, raw.as_bytes())?;
        Ok(ExtractResult {
            adapter: c.prepared.manifest.name.clone(),
            source: c.source.to_string(),
            slice: self.slice.to_string(),
            lead: self.lead.to_string(),
            evidence: path,
        })
    }

    fn write_outcome(w: &mut dyn Write, body: &ExtractResult) -> std::io::Result<()> {
        write_result_text(w, body)
    }

    fn completed_event(&self) -> EventKind {
        EventKind::SliceExtractCompleted {
            slice_name: self.slice.into(),
            source: self.common.source.to_string(),
        }
    }
}

/// The scaffolded `.specify/slices/<slice>/evidence/` directory. Always
/// `Some` for the extract op (prep was handed `evidence_root: Some`).
/// Fails closed with a diagnostic rather than panicking if that prep
/// invariant ever regresses.
fn evidence_dir(prepared: &prep::SourcePrep) -> Result<PathBuf> {
    let Some(dir) = prepared.evidence_dir.clone() else {
        return Err(Error::Diag {
            code: "source-extract-prep-missing",
            detail: "extract prep did not scaffold the slice evidence/ directory \
                (evidence_root was None)"
                .to_string(),
        });
    };
    Ok(dir)
}

fn write_handoff_text(w: &mut dyn Write, body: &ExtractHandoff) -> std::io::Result<()> {
    writeln!(w, "adapter: {} {}", body.adapter, body.version)?;
    writeln!(w, "execution: {}", body.execution)?;
    writeln!(w, "briefs-dir: {}", body.briefs_dir.display())?;
    if let Some(source_dir) = &body.source_dir {
        writeln!(w, "source-dir: {}", source_dir.display())?;
    }
    if let Some(value_inline) = &body.value_inline {
        writeln!(w, "value-inline: {value_inline}")?;
    }
    writeln!(w, "scratch-dir: {}", body.scratch_dir.display())?;
    writeln!(w, "evidence-dir: {}", body.evidence_dir.display())?;
    writeln!(w, "leads: {}", body.leads.join(", "))?;
    Ok(())
}

fn write_result_text(w: &mut dyn Write, body: &ExtractResult) -> std::io::Result<()> {
    writeln!(w, "adapter: {}", body.adapter)?;
    writeln!(w, "source: {}", body.source)?;
    writeln!(w, "slice: {}", body.slice)?;
    writeln!(w, "lead: {}", body.lead)?;
    writeln!(w, "evidence: {}", body.evidence.display())?;
    Ok(())
}
