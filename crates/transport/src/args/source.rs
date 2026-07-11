//! Clap argument types for `specify source *`.

use std::path::PathBuf;

use clap::Args;

/// Argv mirror of `source resolve`'s wire input
/// (`workflow::adapter::handlers::ResolveInput`).
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

/// Argv mirror of `source survey`'s wire input
/// (`workflow::orchestrate::handlers::SurveyInput`).
#[derive(Debug, Args)]
pub struct SurveyArgs {
    /// Source key from `plan.yaml.sources.<key>`.
    pub source: String,
    /// Plan name guard. When set, must match `plan.yaml.name`.
    #[arg(long)]
    pub plan: Option<String>,
}

/// Argv mirror of `source extract`'s wire input
/// (`workflow::orchestrate::handlers::ExtractInput`).
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
