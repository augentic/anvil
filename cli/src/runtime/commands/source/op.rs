//! Shared source-operation kernel for `survey` and `extract`.
//!
//! `survey.rs` and `extract.rs` run the same two-phase agent flow
//! around an adapter-declared brief: resolve the sandbox scratch path,
//! read the staged artifact, validate-before-visible, and commit. The
//! only axis of variation is the [`SourceOperation`] (`survey` vs
//! `extract`), so these helpers are parameterised by it and preserve
//! each operation's wire-stable diagnostic codes via a match on the op.
//!
//! [`run`] drives the whole flow. Source extraction is agent-only —
//! every operation splits into `prepare` / `finalize`, and the CLI
//! never blocks on agent work. Each operation supplies its handoff
//! envelope, commit step (lead-set merge / Evidence persist), and
//! completion event through the [`Flow`] trait, while [`Common`]
//! carries the shared inputs.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::adapter::SourceOperation;
use specify_workflow::change::SourceBinding;
use specify_workflow::journal::{self, Event, EventKind};

use crate::runtime::commands::source::cli::Phase;
use crate::runtime::commands::source::prep;
use crate::runtime::context::Ctx;

/// The `$SCRATCH_DIR` host path the prep mounted for this operation.
///
/// `survey` / `extract` prep always mounts a scratch root; a `None` is
/// an unreachable prep-invariant violation surfaced as
/// a diagnostic rather than a panic.
///
/// # Errors
///
/// Returns `source-scratch-missing` when the prep mounted no scratch
/// root (an unreachable prep-invariant violation).
pub(super) fn scratch_path(prepared: &prep::SourcePrep, op: SourceOperation) -> Result<PathBuf> {
    prepared.layout.scratch.path.clone().ok_or_else(|| Error::Diag {
        code: "source-scratch-missing",
        detail: format!("{op} prep mounted no $SCRATCH_DIR host path"),
    })
}

/// Read the staged artifact (`leads.md` / `evidence.yaml`), mapping a
/// missing file to the operation's wire-stable "must write into
/// `$SCRATCH_DIR` before finalize" diagnostic.
///
/// # Errors
///
/// Returns the operation's `*-missing` diagnostic when the artifact is
/// absent, or [`Error::Io`] on any other read failure.
pub(super) fn read_artifact(path: &Path, op: SourceOperation) -> Result<String> {
    std::fs::read_to_string(path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            let (code, what, writer) = match op {
                SourceOperation::Survey => {
                    ("survey-leads-missing", "leads.md", "the survey must write the lead set")
                }
                SourceOperation::Extract => (
                    "extract-evidence-missing",
                    "evidence.yaml",
                    "the extract must write the Evidence",
                ),
            };
            Error::Diag {
                code,
                detail: format!(
                    "no `{what}` at {}; {writer} into $SCRATCH_DIR before finalize",
                    path.display()
                ),
            }
        } else {
            Error::Io(err)
        }
    })
}

/// Per-invocation inputs shared by every source-operation flow
/// function. Survey and extract differ only in their operation-specific
/// behaviour (the [`Flow`] methods); this bundle carries the rest, so
/// the kernel functions stay under the argument-count budget.
pub(super) struct Common<'a> {
    pub ctx: &'a Ctx,
    /// Plan source key (`plan.yaml.sources.<source>`).
    pub source: &'a str,
    pub prepared: &'a prep::SourcePrep,
    /// Bound `$SOURCE_DIR` host path; `None` for value-bound sources.
    pub source_path: Option<&'a Path>,
    pub binding: &'a SourceBinding,
    /// Operation this flow runs (`survey` / `extract`).
    pub operation: SourceOperation,
}

/// Operation-specific seam for the shared survey/extract two-phase
/// flow. The kernel ([`run`]) owns resolution, the journal events, and
/// rendering; each operation supplies its handoff envelope, commit
/// step (lead-set merge / Evidence persist), and completion event
/// here.
pub(super) trait Flow<'a> {
    /// Agent `prepare` handoff envelope body.
    type Handoff: Serialize;
    /// Completed-operation result body (agent `finalize`).
    type Outcome: Serialize;

    /// The shared per-invocation inputs.
    fn common(&self) -> &Common<'a>;

    /// Build the agent handoff envelope for the mounted `scratch` root.
    fn handoff(&self, scratch: PathBuf) -> Result<Self::Handoff>;
    /// Render the handoff envelope text body.
    fn write_handoff(w: &mut dyn Write, body: &Self::Handoff) -> std::io::Result<()>;

    /// Validate the staged artifact and commit it — merge the lead set
    /// into `discovery.md` (survey) or persist the Evidence (extract) —
    /// returning the result body. `artifact_source` is the on-disk path
    /// `raw` was read from (the Evidence schema error references it).
    /// Called before the completion event, so a validation failure
    /// leaves nothing visible.
    fn commit(&self, raw: &str, artifact_source: &Path) -> Result<Self::Outcome>;
    /// Render the result text body.
    fn write_outcome(w: &mut dyn Write, body: &Self::Outcome) -> std::io::Result<()>;

    /// The completion journal event kind for this operation
    /// (`source.survey.completed` / `slice.extract.completed`).
    fn completed_event(&self) -> EventKind;
}

/// Drive the shared two-phase source-operation flow. Source extraction
/// is agent-only: `prepare` builds the sandbox and prints the handoff
/// envelope, `finalize` validates and commits the staged artifact. The
/// CLI never blocks on agent work.
///
/// # Errors
///
/// Propagates the flow's resolution, validation, and commit failures.
pub(super) fn run<'a, F: Flow<'a>>(flow: &F, phase: Phase) -> Result<()> {
    match phase {
        Phase::Prepare => prepare(flow),
        Phase::Finalize => finalize(flow),
    }
}

/// Recreate the scratch lane empty, dropping any artifact a prior run
/// left behind so `finalize` can only ever stage what this run
/// produced.
fn reset_scratch(scratch: &Path) -> Result<()> {
    match std::fs::remove_dir_all(scratch) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(Error::Io(err)),
    }
    std::fs::create_dir_all(scratch).map_err(Error::Io)
}

/// Agent `prepare`: build scratch, emit `source.execution.agent`, and
/// print the handoff envelope. Control returns to the agent.
fn prepare<'a, F: Flow<'a>>(flow: &F) -> Result<()> {
    let c = flow.common();
    let scratch = scratch_path(c.prepared, c.operation)?;
    reset_scratch(&scratch)?;
    emit_execution_agent(c)?;
    let handoff = flow.handoff(scratch)?;
    c.ctx.write(&handoff, F::write_handoff)
}

/// Agent `finalize`: read the staged artifact,
/// validate-before-visible, commit, then record the completion event.
fn finalize<'a, F: Flow<'a>>(flow: &F) -> Result<()> {
    let c = flow.common();
    let scratch = scratch_path(c.prepared, c.operation)?;
    let artifact_source = scratch.join(c.operation.artifact_name());
    let raw = read_artifact(&artifact_source, c.operation)?;

    let outcome = flow.commit(&raw, &artifact_source)?;
    emit_completed(c, flow)?;
    c.ctx.write(&outcome, F::write_outcome)
}

/// Emit the `source.execution.agent` event marking the agent `prepare`
/// handoff.
fn emit_execution_agent(c: &Common<'_>) -> Result<()> {
    let event = Event::new(
        c.ctx.now(),
        EventKind::SourceExecutionAgent {
            source: c.source.to_string(),
            adapter: c.prepared.manifest.name.clone(),
            operation: c.operation,
        },
    );
    journal::append_batch(c.ctx.layout(), std::slice::from_ref(&event))
}

/// Emit the operation's completion journal event.
fn emit_completed<'a, F: Flow<'a>>(c: &Common<'a>, flow: &F) -> Result<()> {
    let event = Event::new(c.ctx.now(), flow.completed_event());
    journal::append_batch(c.ctx.layout(), std::slice::from_ref(&event))
}
