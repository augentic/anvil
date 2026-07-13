//! Clap argument types for `specify slice *`. Each `*Args` type mirrors
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
    /// Slice name (under `.specify/slices/`)
    pub name: String,
}

/// Arguments for `slice provenance`.
#[derive(Debug, clap::Args)]
pub struct ProvenanceArgs {
    /// Slice name (under `.specify/slices/`)
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
    /// Slice name (under `.specify/slices/`)
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
    /// Slice name (under `.specify/slices/`)
    pub name: String,
}

/// Arguments for `slice merge run`.
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

/// Arguments for `slice merge preview`.
#[derive(Debug, clap::Args)]
pub struct MergePreviewArgs {
    /// Slice name
    pub name: String,
}

/// Arguments for `slice merge conflict-check`.
#[derive(Debug, clap::Args)]
pub struct ConflictCheckArgs {
    /// Slice name
    pub name: String,
}
