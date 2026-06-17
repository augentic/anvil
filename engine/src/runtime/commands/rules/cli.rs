//! Clap derive surface for `specify rules *`. The umbrella `cli.rs`
//! re-exports `RulesAction`.

use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum RulesAction {
    /// Resolve and export the codex envelope as JSON (rules
    /// §"Resolved rules export").
    ///
    /// Read-only: no `.specify/` writes, no lifecycle transitions,
    /// no journal events. The handler probes for shared `UNI-*`
    /// rules (via `--rules-root`, a project-local monorepo
    /// `adapters/shared/rules/universal/` tree, or the distributed
    /// out-of-tree codex cache `<project-cache>/codex/`), discovers the
    /// `--target` and `--source` overlays per the rules contract §"Resolution
    /// roots", and streams the sorted `ResolvedRules` envelope to
    /// stdout.
    ///
    /// Only `--format json` is supported in v1; text output is
    /// deferred to a follow-up. The global `--format text` default
    /// at the `Cli` level surfaces as an explicit argument error so
    /// the closed JSON-only contract stays visible.
    Export(ExportArgs),

    /// Distribute (or refresh) the shared codex into the out-of-tree project
    /// codex cache `<project-cache>/codex/`, pinned to the project's adapter
    /// source/ref (codex root resolution, RM-07).
    ///
    /// Mirrors `adapters/shared/rules/universal/` (and, with
    /// `--include-framework`, `core/`) from the adapter source so the
    /// resolver's rules-root probe finds shared `UNI-*` rules without
    /// `--rules-root`. Requires an initialised `.specify/`; writes only
    /// under the out-of-tree `<project-cache>/codex/`.
    Sync(SyncArgs),
}

/// Flag surface for `specify rules sync`.
#[derive(Args)]
pub struct SyncArgs {
    /// Also distribute the framework `core/` pack
    /// (`adapters/shared/rules/core/`) alongside the always-distributed
    /// `universal/` pack. Default off — consumer projects carry only
    /// `UNI-*` rules.
    #[arg(long)]
    pub include_framework: bool,

    /// Adapter source to pull the codex from (bare name or URL).
    /// Defaults to the project's recorded `adapter:` value; required
    /// for workspace projects, which declare no adapter.
    #[arg(long)]
    pub source: Option<String>,
}

/// Flag surface for `specify rules export`. Grouped into one struct so
/// the handler threads a single reference instead of a positional
/// argument list.
#[derive(Args)]
pub struct ExportArgs {
    /// Codex root supplying shared `UNI-*` rules and rules-root
    /// fallback overlays (codex root resolution (v1)). When omitted the
    /// resolver probes the project-local monorepo
    /// `adapters/shared/rules/universal/` tree, then the distributed
    /// out-of-tree codex cache `<project-cache>/codex/` (populated by `specify init`
    /// / `specify rules sync`); failing both, exits with
    /// `rules-root-required`.
    #[arg(long)]
    pub rules_root: Option<PathBuf>,

    /// Target-adapter name (kebab, optionally `<name>@v<major>`).
    /// Required.
    #[arg(long)]
    pub target: String,

    /// Source-adapter name bound to the export context;
    /// repeatable. Each occurrence contributes one
    /// `Origin::Source` overlay.
    #[arg(long = "source", value_name = "NAME")]
    pub sources: Vec<String>,

    /// Project-relative artifact path passed to CH-13's
    /// `applicability.paths` glob check; repeatable.
    #[arg(long = "artifact", value_name = "PATH")]
    pub artifacts: Vec<PathBuf>,

    /// Lowercase language token passed to CH-13's
    /// `applicability.languages` match; repeatable.
    #[arg(long = "language", value_name = "TOKEN")]
    pub languages: Vec<String>,

    /// Include rules marked `deprecated:` in the export.
    #[arg(long)]
    pub include_deprecated: bool,

    /// Include rules whose applicability dimensions the caller
    /// did not satisfy.
    #[arg(long)]
    pub include_unmatched: bool,

    /// Include `CORE-*` rules resolved from
    /// `adapters/shared/rules/core/`.
    /// Default off — consumer-project exports never carry
    /// framework-only `CORE-*` rules unless the caller opts in.
    #[arg(long)]
    pub include_core: bool,

    /// Project directory used as the resolver's `project_dir`
    /// (defaults to the current directory). Does not require
    /// an initialised `.specify/`.
    #[arg(long, default_value = ".")]
    pub project_dir: PathBuf,
}
