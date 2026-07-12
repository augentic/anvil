//! Clap argument types for `specify source *`. Each `*Args` type mirrors
//! its command's workflow wire input.

use std::path::PathBuf;

use clap::Args;

/// Arguments for `source resolve`.
#[derive(Debug, Args)]
pub struct ResolveArgs {
    /// Kebab-case source-adapter name (e.g. `intent`,
    /// `documentation`, `typescript`, `screenshots`).
    #[arg(value_name = "NAME")]
    pub value: String,
    /// Project directory containing `.specify/` (defaults to the
    /// current directory).
    #[arg(long, default_value = ".")]
    pub project_dir: Option<PathBuf>,
}

/// Arguments for `source survey`.
#[derive(Debug, Args)]
pub struct SurveyArgs {
    /// Source key from `plan.yaml.sources.<key>`.
    pub source: String,
    /// Plan name guard. When set, must match `plan.yaml.name`.
    #[arg(long)]
    pub plan: Option<String>,
}

/// Arguments for `source extract`.
#[derive(Debug, Args)]
pub struct ExtractArgs {
    /// Source key from `plan.yaml.sources.<key>`.
    pub source: String,
    /// Lead id (from `discovery.md`) the Evidence is bound to.
    pub lead: String,
    /// Slice the Evidence is extracted into; keys the
    /// `.specify/slices/<slice>/evidence/` target.
    #[arg(long)]
    pub slice: String,
}
