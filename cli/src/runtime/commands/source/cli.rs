//! Clap derive surface for `specify source *`. The umbrella `cli.rs`
//! re-exports `SourceAction`.

use std::path::PathBuf;

use clap::{Subcommand, ValueEnum};

/// Which phase of a two-phase `specify source` operation
/// (`survey` / `extract`) to run.
///
/// Source operations are agent-driven and two-phase: `prepare` builds
/// the sandbox and prints the handoff envelope, then the agent runs
/// the brief and calls back with `finalize`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum Phase {
    /// Build the sandbox + scratch + output target, emit
    /// `source.execution.agent`, and print the handoff envelope. The
    /// default.
    #[default]
    Prepare,
    /// Validate the agent-produced output and merge it into
    /// `discovery.md` (`survey`) / persist the Evidence (`extract`).
    Finalize,
}

#[derive(Subcommand)]
pub enum SourceAction {
    /// Resolve a source-adapter manifest by kebab name.
    ///
    /// Probe order: `<project-cache>/manifests/sources/<name>/adapter.yaml`
    /// (agent-populated out-of-tree manifest cache), then
    /// `<project-dir>/adapters/sources/<name>/adapter.yaml`
    /// (in-repo). Emits the resolved directory path plus the
    /// manifest's declared operations.
    Resolve {
        /// Kebab-case source-adapter name (e.g. `intent`,
        /// `documentation`, `typescript`, `screenshots`).
        name: String,
        /// Project directory containing `.specify/` (defaults to the
        /// current directory).
        #[arg(long, default_value = ".")]
        project_dir: PathBuf,
    },

    /// Run a source adapter's survey + extract in isolation
    /// (`specify source preview` contract).
    ///
    /// Resolves the adapter manifest, validates the `--source` path,
    /// scaffolds the output directory with an `evidence/` subtree, and
    /// emits a summary of adapter info and brief paths. The agent then
    /// executes the briefs against the prepared environment.
    ///
    /// Workflow-free: nothing is written into `.specify/`, no lifecycle
    /// moves, and no journal events fire. Output lives entirely under
    /// `--out`.
    Preview {
        /// Kebab-case source-adapter name (e.g. `screenshots`,
        /// `typescript`, `documentation`).
        adapter: String,
        /// Bound source path (`$SOURCE_DIR` for the adapter's briefs).
        #[arg(long)]
        source: PathBuf,
        /// Restrict extraction to specific lead IDs; defaults to
        /// all leads discovered by `survey`.
        #[arg(long)]
        lead: Vec<String>,
        /// Output directory for Evidence files (default:
        /// `.specify-preview/`). Created if absent.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Project directory used for adapter resolution (defaults to
        /// the current directory). Does not require an initialised
        /// `.specify/` directory.
        #[arg(long, default_value = ".")]
        project_dir: PathBuf,
    },

    /// Run a source adapter's `survey` against a plan-bound source and
    /// merge the resulting lead set into `discovery.md`.
    ///
    /// Resolves `<source>` against `plan.yaml.sources.<key>` (not
    /// the adapter name), resolves the bound source adapter, and builds
    /// the four-root sandbox under
    /// `.specify/scratch/<adapter>/survey/`.
    ///
    /// The operation is two-phase: `--phase prepare` (the default)
    /// prints the handoff envelope and returns control to the agent;
    /// `--phase finalize` validates the agent-produced `leads.md` and
    /// merges it.
    Survey {
        /// Source key from `plan.yaml.sources.<key>`.
        source: String,
        /// Plan name guard. When set, must match `plan.yaml.name`.
        #[arg(long)]
        plan: Option<String>,
        /// Phase to run (`prepare` | `finalize`).
        #[arg(long, value_enum, default_value_t = Phase::Prepare)]
        phase: Phase,
    },

    /// Run a source adapter's `extract` for one `(source, lead)`
    /// pair and persist the resulting Evidence to
    /// `.specify/slices/<slice>/evidence/<source>.yaml`.
    ///
    /// Resolves `<source>` against `plan.yaml.sources.<key>` (not
    /// the adapter name), resolves the bound source adapter, and builds
    /// the four-root sandbox with scratch under
    /// `.specify/scratch/<adapter>/<slice>/`.
    ///
    /// The operation is two-phase: `--phase prepare` (the default)
    /// prints the handoff envelope and returns control to the agent;
    /// `--phase finalize` validates the agent-produced Evidence
    /// against `schemas/evidence.schema.json` before it is persisted.
    Extract {
        /// Source key from `plan.yaml.sources.<key>`.
        source: String,
        /// Lead id (from `discovery.md`) the Evidence is bound to.
        lead: String,
        /// Slice the Evidence is extracted into; keys the scratch
        /// directory and the `.specify/slices/<slice>/evidence/` target.
        #[arg(long)]
        slice: String,
        /// Phase to run (`prepare` | `finalize`).
        #[arg(long, value_enum, default_value_t = Phase::Prepare)]
        phase: Phase,
    },
}
