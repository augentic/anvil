//! Clap derive surface for `specify source *`. The umbrella `cli.rs`
//! re-exports `SourceAction`.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde::Serialize;

/// Verbs under `specify source`.
#[derive(Debug, Subcommand)]
pub enum SourceAction {
    /// Resolve a source adapter by kebab name.
    ///
    /// Resolves the single `.wasm` component: the global
    /// store entry for a pinned identity, else the project component
    /// cache / development release build for a bare name. Emits the
    /// resolved component path plus the axis's closed operation set.
    Resolve(ResolveArgs),

    /// Run a source adapter's `survey` against a plan-bound source and
    /// merge the resulting lead set into `discovery.md`.
    ///
    /// Resolves `<source>` against `plan.yaml.sources.<key>` (not
    /// the adapter name) and drives the bound source adapter's
    /// collapsed survey orchestration in the workflow guest — one call
    /// covering the source dispatch, `leads.md` validation, and the
    /// `discovery.md` merge.
    Survey(SurveyArgs),

    /// Run a source adapter's `extract` for one `(source, lead)`
    /// pair and persist the resulting Evidence to
    /// `.specify/slices/<slice>/evidence/<source>.yaml`.
    ///
    /// Resolves `<source>` against `plan.yaml.sources.<key>` (not
    /// the adapter name) and drives the bound source adapter's
    /// collapsed extract orchestration in the workflow guest — one
    /// call covering the source dispatch, the Evidence schema gate
    /// (`schemas/evidence.schema.json`), and the persist.
    Extract(ExtractArgs),
}

/// Argv mirror of `source resolve`'s wire input
/// (`workflow::adapter::handlers::ResolveInput`).
#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "kebab-case")]
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
#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SurveyArgs {
    /// Source key from `plan.yaml.sources.<key>`.
    pub source: String,
    /// Plan name guard. When set, must match `plan.yaml.name`.
    #[arg(long)]
    pub plan: Option<String>,
}

/// Argv mirror of `source extract`'s wire input
/// (`workflow::orchestrate::handlers::ExtractInput`).
#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "kebab-case")]
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
