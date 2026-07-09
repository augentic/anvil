//! Clap derive surface for `specify source *`. The umbrella `cli.rs`
//! re-exports `SourceAction`.

use std::path::PathBuf;

use clap::Subcommand;

/// Verbs under `specify source`.
#[derive(Debug, Subcommand)]
pub enum SourceAction {
    /// Resolve a source adapter by kebab name.
    ///
    /// Resolves the single `.wasm` component: the global
    /// store entry for a pinned identity, else the project component
    /// cache / development release build for a bare name. Emits the
    /// resolved component path plus the axis's closed operation set.
    Resolve {
        /// Kebab-case source-adapter name (e.g. `intent`,
        /// `documentation`, `typescript`, `screenshots`).
        name: String,
        /// Project directory containing `.specify/` (defaults to the
        /// current directory).
        #[arg(long, default_value = ".")]
        project_dir: PathBuf,
    },

    /// Run a source adapter's `survey` against a plan-bound source and
    /// merge the resulting lead set into `discovery.md`.
    ///
    /// Resolves `<source>` against `plan.yaml.sources.<key>` (not
    /// the adapter name) and drives the bound source adapter's
    /// collapsed survey orchestration in the workflow guest — one call
    /// covering the source dispatch, `leads.md` validation, and the
    /// `discovery.md` merge.
    Survey {
        /// Source key from `plan.yaml.sources.<key>`.
        source: String,
        /// Plan name guard. When set, must match `plan.yaml.name`.
        #[arg(long)]
        plan: Option<String>,
    },

    /// Run a source adapter's `extract` for one `(source, lead)`
    /// pair and persist the resulting Evidence to
    /// `.specify/slices/<slice>/evidence/<source>.yaml`.
    ///
    /// Resolves `<source>` against `plan.yaml.sources.<key>` (not
    /// the adapter name) and drives the bound source adapter's
    /// collapsed extract orchestration in the workflow guest — one
    /// call covering the source dispatch, the Evidence schema gate
    /// (`schemas/evidence.schema.json`), and the persist.
    Extract {
        /// Source key from `plan.yaml.sources.<key>`.
        source: String,
        /// Lead id (from `discovery.md`) the Evidence is bound to.
        lead: String,
        /// Slice the Evidence is extracted into; keys the
        /// `.specify/slices/<slice>/evidence/` target.
        #[arg(long)]
        slice: String,
    },
}
