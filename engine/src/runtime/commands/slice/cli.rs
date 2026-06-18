//! Clap derive surface for `specify slice *` and its nested verbs.
//! The umbrella `cli.rs` re-exports the action enums.

use std::path::PathBuf;

use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{ArgAction, Subcommand};
use specify_workflow::slice::{CreateIfExists, LifecycleStatus};

use crate::runtime::commands::source::cli::Phase;

/// Clap value parser for a workflow enum carrying strum's kebab-case
/// `EnumString` + `VariantNames` derives. `specify-workflow` is
/// clap-free, so the possible-values surface is reconstructed here at
/// the CLI boundary from the strum metadata.
fn kebab_enum<T>() -> impl TypedValueParser<Value = T>
where
    T: std::str::FromStr + strum::VariantNames + Clone + Send + Sync + 'static,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    PossibleValuesParser::new(T::VARIANTS).try_map(|value| value.parse::<T>())
}

#[derive(Subcommand)]
pub enum SliceAction {
    /// Create a new slice directory with an initial `metadata.yaml`
    Create {
        /// Kebab-case slice name
        name: String,
        /// Target-adapter identifier; defaults to the value in `.specify/project.yaml`
        #[arg(long)]
        target: Option<String>,
        /// Behaviour when `<slices_dir>/<name>/` already exists
        #[arg(long, value_parser = kebab_enum::<CreateIfExists>(), default_value = "fail")]
        if_exists: CreateIfExists,
    },
    /// Validate a slice's artifacts against adapter validation rules
    Validate {
        /// Slice name (under `.specify/slices/`)
        name: String,
    },
    /// Project the audit-only provenance view from the slice's
    /// `model.yaml`. Provenance is
    /// carried inline in `model.yaml`; this reshapes it on demand and
    /// never reads or writes a `provenance.yaml` file.
    Provenance {
        /// Slice name (under `.specify/slices/`)
        name: String,
    },
    /// Read-only viewer over a slice's `model.yaml`
    Model {
        #[command(subcommand)]
        action: SliceModelAction,
    },
    /// Synthesise a slice — assemble the agent INPUTS envelope, or
    /// project an agent response into `model.yaml` + Markdown artifacts.
    ///
    /// Exactly one mode is required — the parser rejects passing both:
    ///
    /// - `--dry-run` is read-only. It reads the slice's bound
    ///   `evidence/<source>.yaml` and the target `shape` brief and
    ///   emits the `kind: inputs` envelope for the agent synthesis
    ///   step. Writes nothing; emits the `slice.synthesize.agent`
    ///   journal event (synthesis is always agent-dispatched).
    /// - `--from <response.json>` is the only writer. It schema-gates
    ///   the agent response, resolves authority from the on-disk
    ///   Evidence and any per-slice override, projects the kernel-owned
    ///   fields into `model.yaml`, renders provenance into
    ///   `specs/<domain>/spec.md`, and persists the staged artifacts
    ///   atomically — emitting `slice.synthesize.started` then
    ///   `slice.synthesize.completed` (or `slice.synthesize.failed` on
    ///   error).
    ///
    /// Passing neither mode fails with `slice-synthesize-mode-required`
    /// (exit 2).
    Synthesize {
        /// Slice name (under `.specify/slices/`)
        name: String,
        /// Assemble and emit the agent INPUTS envelope. Writes nothing.
        #[arg(long = "dry-run", action = ArgAction::SetTrue)]
        dry_run: bool,
        /// Apply the agent's synthesis response, project it, and persist the artifacts. The only writer.
        #[arg(long = "from", value_name = "RESPONSE_JSON", conflicts_with = "dry_run")]
        from: Option<PathBuf>,
    },
    /// Build a slice through its bound target adapter's `build`
    /// operation and gate the `built` transition.
    ///
    /// Resolves the target from the slice's `metadata.yaml`, then
    /// owns the build envelopes: request assembly, report validation,
    /// the `target-build-*` aborts, the `slice.build.*` events, and the
    /// `built` transition gate. The target brief owns only code
    /// generation.
    ///
    /// For `execution: tool` adapters the single call runs the whole
    /// operation. For `execution: agent` adapters the operation is
    /// two-phase: `--phase prepare` (the default) assembles and
    /// schema-validates the request, writes
    /// `.specify/slices/<slice>/build/request.yaml`, emits
    /// `target.execution.agent`, prints the handoff envelope, and
    /// returns control to the agent; `--phase finalize` validates the
    /// agent-produced `build/report.yaml`, gates the `built`
    /// transition, and journals `slice.build.succeeded` /
    /// `slice.build.failed`.
    Build {
        /// Slice name (under `.specify/slices/`)
        name: String,
        /// Phase to run (`prepare` | `finalize`); `tool` adapters run
        /// the whole operation regardless.
        #[arg(long, value_enum, default_value_t = Phase::Prepare)]
        phase: Phase,
    },
    /// Spec-merge operations for a slice
    Merge {
        #[command(subcommand)]
        action: SliceMergeAction,
    },
    /// Tasks-list operations for a slice
    Task {
        #[command(subcommand)]
        action: SliceTaskAction,
    },
    /// Transition a slice to a new lifecycle status. Note: `merged` is
    /// not a valid target — the only legal writer of `Merged` is
    /// `specify slice merge run`, which performs the spec merge,
    /// status transition, and archive move atomically.
    Transition {
        /// Slice name
        name: String,
        /// Target status (`refining`, `refined`, `built`, or `dropped`).
        /// `merged` is reserved for `specify slice merge run` and is
        /// rejected with exit 2 if passed here.
        #[arg(value_parser = kebab_enum::<LifecycleStatus>())]
        target: LifecycleStatus,
    },
    /// Scan or overwrite `touched_specs` on `metadata.yaml`
    TouchedSpecs {
        /// Slice name
        name: String,
        /// Scan `specs/` subdirs and classify each as new or modified
        #[arg(long, conflicts_with = "set")]
        scan: bool,
        /// Replace `touched_specs` with the listed adapters (each `<name>:new|modified`)
        #[arg(long, value_delimiter = ',')]
        set: Vec<String>,
    },
    /// Report overlapping `touched_specs` with other active slices
    Overlap {
        /// Slice name
        name: String,
    },
    /// Transition a slice to `dropped` and archive it
    Drop {
        /// Slice name
        name: String,
        /// Free-text reason; surfaced in `metadata.yaml.drop_reason` and the archive path
        #[arg(long)]
        reason: Option<String>,
    },
}

/// Read-only model-viewer subcommands grouped under `slice model`.
#[derive(Subcommand)]
pub enum SliceModelAction {
    /// Render the persisted `model.yaml` — concise text view, or the
    /// model serialised verbatim under `--format json`
    Show {
        /// Slice name (under `.specify/slices/`)
        name: String,
    },
}

/// Spec-merge subcommands grouped under `slice merge`.
#[derive(Subcommand)]
pub enum SliceMergeAction {
    /// Merge all delta specs for the slice into baseline and archive the slice
    Run {
        /// Slice name
        name: String,
        /// Authorise a whole-document (`screens:`) slice composition to
        /// overwrite a non-empty baseline. Reserved for intentional
        /// full-baseline rewrites (e.g. a dedicated refactoring slice);
        /// routine per-screen edits flow through `delta:` and never need it.
        #[arg(long)]
        allow_composition_replace: bool,
    },
    /// Show the merge operations that would be applied, without writing
    Preview {
        /// Slice name
        name: String,
    },
    /// Report `type: modified` baselines modified after this slice's `defined_at`
    ConflictCheck {
        /// Slice name
        name: String,
    },
}

/// Task-list subcommands grouped under `slice task`.
#[derive(Subcommand)]
pub enum SliceTaskAction {
    /// Report task completion counts (total, complete, pending)
    Progress {
        /// Slice name
        name: String,
    },
    /// Mark a task complete (idempotent — no-op if already complete)
    Mark {
        /// Slice name
        name: String,
        /// Task number (e.g. `1.1`)
        task_number: String,
    },
}
