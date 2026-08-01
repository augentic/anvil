//! Clap argument types for `emery slice *`. Each `*Args` type mirrors
//! its command's workflow wire input.

/// Arguments for `slice list` — none.
#[derive(Clone, Copy, Debug, clap::Args)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "clap's `Args` derive requires a braced struct"
)]
pub struct ListArgs {}

/// Arguments for `slice validate`.
#[derive(Debug, clap::Args)]
pub struct ValidateArgs {
    /// Slice name (under `.emery/slices/`)
    pub name: String,
}

/// Arguments for `slice provenance`.
#[derive(Debug, clap::Args)]
pub struct ProvenanceArgs {
    /// Slice name (under `.emery/slices/`)
    pub name: String,
}

/// Arguments for `slice refine`.
#[derive(Debug, clap::Args)]
pub struct RefineArgs {
    /// Slice name (a `plan.yaml.slices[]` entry)
    pub name: String,
}

/// Arguments for `slice build`.
#[derive(Debug, clap::Args)]
pub struct BuildArgs {
    /// Slice name (under `.emery/slices/`)
    pub name: String,
}

/// Arguments for `slice drop`.
#[derive(Debug, clap::Args)]
pub struct DropArgs {
    /// Slice name
    pub name: String,
    /// Free-text reason; surfaced in `metadata.yaml.drop_reason` and the archive path
    #[arg(long)]
    pub reason: Option<String>,
}

/// Arguments for `slice model show`.
#[derive(Debug, clap::Args)]
pub struct ModelShowArgs {
    /// Slice name (under `.emery/slices/`)
    pub name: String,
}

/// Arguments for `slice merge`.
#[derive(Debug, clap::Args)]
pub struct MergeArgs {
    /// Slice name
    pub name: String,
    /// Authorise a whole-document (`screens:`) slice composition to
    /// overwrite a non-empty baseline. Reserved for intentional
    /// full-baseline rewrites (e.g. a dedicated refactoring slice);
    /// routine per-screen edits flow through `delta:` and never need it.
    #[arg(long)]
    pub allow_composition_replace: bool,
    /// Show the merge operations that would be applied, without
    /// writing.
    #[arg(long, conflicts_with = "conflict_check")]
    pub preview: bool,
    /// Report `type: modified` baselines modified after this slice's
    /// `defined_at`, without writing.
    #[arg(long)]
    pub conflict_check: bool,
}
