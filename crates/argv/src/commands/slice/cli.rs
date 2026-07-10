//! Clap derive surface for `specify slice *` and its nested verbs.
//! The umbrella `cli.rs` re-exports the action enums.

use clap::Subcommand;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use serde::Serialize;
use workflow::slice::{CreateIfExists, LifecycleStatus};

/// Clap value parser for a workflow enum carrying strum's kebab-case
/// `EnumString` + `VariantNames` derives. `workflow` is
/// clap-free, so the possible-values surface is reconstructed here at
/// the CLI boundary from the strum metadata.
fn kebab_enum<T>() -> impl TypedValueParser<Value = T>
where
    T: std::str::FromStr + strum::VariantNames + Clone + Send + Sync + 'static,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    PossibleValuesParser::new(T::VARIANTS).try_map(|value| value.parse::<T>())
}

/// Verbs under `specify slice`.
#[derive(Debug, Subcommand)]
pub enum SliceAction {
    /// Create a new slice directory with an initial `metadata.yaml`
    Create(CreateArgs),
    /// Validate a slice's artifacts against adapter validation rules
    Validate(ValidateArgs),
    /// Project the audit-only provenance view from the slice's
    /// `model.yaml`. Provenance is
    /// carried inline in `model.yaml`; this reshapes it on demand and
    /// never reads or writes a `provenance.yaml` file.
    Provenance(ProvenanceArgs),
    /// Read-only viewer over a slice's `model.yaml`
    Model {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: SliceModelAction,
    },
    /// Refine one named plan entry's slice to `refined` in the
    /// workflow guest: slice create (re-entry safe), per-binding
    /// extract fan-out, the synthesis judgment leg, the persist tail,
    /// validate, and the `refined` transition — the `/spec:refine`
    /// breakout outside the execute loop.
    ///
    /// Acts on the named slice directly against a `pending` or
    /// `in-progress` plan entry (the standalone `slice build <name>`
    /// posture); never advances per-entry status, and refuses a `done`
    /// entry.
    ///
    /// Guest-only. The native binary refuses this verb — natively the
    /// phase is driven by the `/spec:refine` skill.
    Refine(RefineArgs),
    /// Build a slice through its bound target adapter's `build`
    /// operation and gate the `built` transition.
    ///
    /// Resolves the target from the slice's `metadata.yaml`, then
    /// drives the collapsed build orchestration in the workflow guest:
    /// request assembly and schema gate, the target-seam dispatch, the
    /// report gates (`target-build-*` aborts), the `slice.build.*`
    /// events, and the `Refined → Built` transition. The target guest
    /// owns only code generation.
    Build(BuildArgs),
    /// Spec-merge operations for a slice
    Merge {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: SliceMergeAction,
    },
    /// Tasks-list operations for a slice
    Task {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: SliceTaskAction,
    },
    /// Transition a slice to a new lifecycle status. Note: `merged` is
    /// not a valid target — the only legal writer of `Merged` is
    /// `specify slice merge run`, which performs the spec merge,
    /// status transition, and archive move atomically.
    Transition(TransitionArgs),
    /// Scan or overwrite `touched_specs` on `metadata.yaml`
    TouchedSpecs(TouchedSpecsArgs),
    /// Report overlapping `touched_specs` with other active slices
    Overlap(OverlapArgs),
    /// Transition a slice to `dropped` and archive it
    Drop(DropArgs),
}

/// Argv mirror of `slice create`'s wire input
/// (`workflow::slice::handlers::CreateInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CreateArgs {
    /// Kebab-case slice name
    pub name: String,
    /// Target-adapter identifier; defaults to the value in `.specify/project.yaml`
    #[arg(long)]
    pub target: Option<String>,
    /// Behaviour when `<slices_dir>/<name>/` already exists
    #[arg(long, value_parser = kebab_enum::<CreateIfExists>(), default_value = "fail")]
    pub if_exists: CreateIfExists,
}

/// Argv mirror of `slice validate`'s wire input
/// (`workflow::slice::handlers::ValidateInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ValidateArgs {
    /// Slice name (under `.specify/slices/`)
    pub name: String,
}

/// Argv mirror of `slice provenance`'s wire input
/// (`workflow::slice::handlers::ProvenanceInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProvenanceArgs {
    /// Slice name (under `.specify/slices/`)
    pub name: String,
}

/// Argv mirror of `slice refine`'s wire input
/// (`workflow::orchestrate::handlers::RefineInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefineArgs {
    /// Slice name (a `plan.yaml.slices[]` entry)
    pub name: String,
}

/// Argv mirror of `slice build`'s wire input
/// (`workflow::orchestrate::handlers::BuildInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildArgs {
    /// Slice name (under `.specify/slices/`)
    pub name: String,
}

/// Argv mirror of `slice transition`'s wire input
/// (`workflow::slice::handlers::TransitionInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TransitionArgs {
    /// Slice name
    pub name: String,
    /// Target status (`refining`, `refined`, `built`, or `dropped`).
    /// `merged` is reserved for `specify slice merge run` and is
    /// rejected with exit 2 if passed here.
    #[arg(value_parser = kebab_enum::<LifecycleStatus>())]
    pub target: LifecycleStatus,
}

/// Argv mirror of `slice touched-specs`' wire input
/// (`workflow::slice::handlers::TouchedSpecsInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TouchedSpecsArgs {
    /// Slice name
    pub name: String,
    /// Scan `specs/` subdirs and classify each as new or modified
    #[arg(long, conflicts_with = "set")]
    pub scan: bool,
    /// Replace `touched_specs` with the listed adapters (each `<name>:new|modified`)
    #[arg(long, value_delimiter = ',')]
    pub set: Vec<String>,
}

/// Argv mirror of `slice overlap`'s wire input
/// (`workflow::slice::handlers::OverlapInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct OverlapArgs {
    /// Slice name
    pub name: String,
}

/// Argv mirror of `slice drop`'s wire input
/// (`workflow::slice::handlers::DropInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DropArgs {
    /// Slice name
    pub name: String,
    /// Free-text reason; surfaced in `metadata.yaml.drop_reason` and the archive path
    #[arg(long)]
    pub reason: Option<String>,
}

/// Read-only model-viewer subcommands grouped under `slice model`.
#[derive(Debug, Subcommand)]
pub enum SliceModelAction {
    /// Render the persisted `model.yaml` — concise text view, or the
    /// model serialised verbatim under `--format json`
    Show(ModelShowArgs),
}

/// Argv mirror of `slice model show`'s wire input
/// (`workflow::slice::handlers::ModelShowInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ModelShowArgs {
    /// Slice name (under `.specify/slices/`)
    pub name: String,
}

/// Spec-merge subcommands grouped under `slice merge`.
#[derive(Debug, Subcommand)]
pub enum SliceMergeAction {
    /// Merge all delta specs for the slice into baseline and archive the slice
    Run(MergeRunArgs),
    /// Show the merge operations that would be applied, without writing
    Preview(MergePreviewArgs),
    /// Report `type: modified` baselines modified after this slice's `defined_at`
    ConflictCheck(ConflictCheckArgs),
}

/// Argv mirror of `slice merge run`'s wire input
/// (`workflow::orchestrate::handlers::MergeRunInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MergeRunArgs {
    /// Slice name
    pub name: String,
    /// Authorise a whole-document (`screens:`) slice composition to
    /// overwrite a non-empty baseline. Reserved for intentional
    /// full-baseline rewrites (e.g. a dedicated refactoring slice);
    /// routine per-screen edits flow through `delta:` and never need it.
    #[arg(long)]
    pub allow_composition_replace: bool,
}

/// Argv mirror of `slice merge preview`'s wire input
/// (`workflow::slice::handlers::PreviewInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MergePreviewArgs {
    /// Slice name
    pub name: String,
}

/// Argv mirror of `slice merge conflict-check`'s wire input
/// (`workflow::slice::handlers::ConflictCheckInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConflictCheckArgs {
    /// Slice name
    pub name: String,
}

/// Task-list subcommands grouped under `slice task`.
#[derive(Debug, Subcommand)]
pub enum SliceTaskAction {
    /// Report task completion counts (total, complete, pending)
    Progress(TaskProgressArgs),
    /// Mark a task complete (idempotent — no-op if already complete)
    Mark(TaskMarkArgs),
}

/// Argv mirror of `slice task progress`' wire input
/// (`workflow::slice::handlers::TaskProgressInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TaskProgressArgs {
    /// Slice name
    pub name: String,
}

/// Argv mirror of `slice task mark`'s wire input
/// (`workflow::slice::handlers::TaskMarkInput`).
#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TaskMarkArgs {
    /// Slice name
    pub name: String,
    /// Task number (e.g. `1.1`)
    pub task_number: String,
}
