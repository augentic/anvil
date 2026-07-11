//! Clap argument types for `specify slice *`.

use clap::builder::{PossibleValuesParser, TypedValueParser};
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

/// Argv mirror of `slice create`'s wire input
/// (`workflow::slice::handlers::CreateInput`).
#[derive(Debug, clap::Args)]
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
#[derive(Debug, clap::Args)]
pub struct ValidateArgs {
    /// Slice name (under `.specify/slices/`)
    pub name: String,
}

/// Argv mirror of `slice provenance`'s wire input
/// (`workflow::slice::handlers::ProvenanceInput`).
#[derive(Debug, clap::Args)]
pub struct ProvenanceArgs {
    /// Slice name (under `.specify/slices/`)
    pub name: String,
}

/// Argv mirror of `slice refine`'s wire input
/// (`workflow::slice::handlers::RefineInput`).
#[derive(Debug, clap::Args)]
pub struct RefineArgs {
    /// Slice name (a `plan.yaml.slices[]` entry)
    pub name: String,
}

/// Argv mirror of `slice build`'s wire input
/// (`workflow::slice::handlers::BuildInput`).
#[derive(Debug, clap::Args)]
pub struct BuildArgs {
    /// Slice name (under `.specify/slices/`)
    pub name: String,
}

/// Argv mirror of `slice transition`'s wire input
/// (`workflow::slice::handlers::TransitionInput`).
#[derive(Debug, clap::Args)]
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
#[derive(Debug, clap::Args)]
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
#[derive(Debug, clap::Args)]
pub struct OverlapArgs {
    /// Slice name
    pub name: String,
}

/// Argv mirror of `slice drop`'s wire input
/// (`workflow::slice::handlers::DropInput`).
#[derive(Debug, clap::Args)]
pub struct DropArgs {
    /// Slice name
    pub name: String,
    /// Free-text reason; surfaced in `metadata.yaml.drop_reason` and the archive path
    #[arg(long)]
    pub reason: Option<String>,
}

/// Argv mirror of `slice model show`'s wire input
/// (`workflow::slice::handlers::ModelShowInput`).
#[derive(Debug, clap::Args)]
pub struct ModelShowArgs {
    /// Slice name (under `.specify/slices/`)
    pub name: String,
}

/// Argv mirror of `slice merge run`'s wire input
/// (`workflow::slice::handlers::MergeRunInput`).
#[derive(Debug, clap::Args)]
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
#[derive(Debug, clap::Args)]
pub struct MergePreviewArgs {
    /// Slice name
    pub name: String,
}

/// Argv mirror of `slice merge conflict-check`'s wire input
/// (`workflow::slice::handlers::ConflictCheckInput`).
#[derive(Debug, clap::Args)]
pub struct ConflictCheckArgs {
    /// Slice name
    pub name: String,
}

/// Argv mirror of `slice task progress`' wire input
/// (`workflow::slice::handlers::TaskProgressInput`).
#[derive(Debug, clap::Args)]
pub struct TaskProgressArgs {
    /// Slice name
    pub name: String,
}

/// Argv mirror of `slice task mark`'s wire input
/// (`workflow::slice::handlers::TaskMarkInput`).
#[derive(Debug, clap::Args)]
pub struct TaskMarkArgs {
    /// Slice name
    pub name: String,
    /// Task number (e.g. `1.1`)
    pub task_number: String,
}
