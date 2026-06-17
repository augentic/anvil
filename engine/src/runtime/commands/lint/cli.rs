//! Clap derive surface for `specify lint *`. Mirrors the
//! `RulesAction` shape in `src/runtime/commands/rules/cli.rs`.
//!
//! The per-subcommand `--format` flag is intentionally distinct from
//! the global `Cli::format` flag: global `--format` toggles JSON vs
//! text for envelope-emitting handlers and the failure path, while
//! `specify lint --format` selects the closed the diagnostics formatter set set
//! (`{ json, pretty, github, compact }`). The handler reads its own
//! per-subcommand flag and ignores the global one for the success
//! body.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use specify_diagnostics::Format as DiagnosticsFormat;

#[derive(Subcommand)]
pub enum LintAction {
    /// Downstream consumer-project scan: resolve applicable codex
    /// rules, build a `WorkspaceModel`, evaluate deterministic hints,
    /// and emit the structured review envelope.
    Project(ProjectArgs),
    /// Framework authoring lint over the `augentic/specify` repo.
    ///
    /// Composes the imperative `Check` predicates with the declarative
    /// deterministic-hint interpreter and emits one structured
    /// envelope per run. Defaults `--framework-root` to `.`, hard-codes
    /// the framework scan profile, and always evaluates `CORE-*` rules.
    /// Contributor surface — operators reach for `lint framework`.
    Framework(FrameworkArgs),
}

/// Flag surface for `specify lint framework`. Mirrors [`ProjectArgs`]
/// modulo these pinned defaults:
///
/// - `--framework-root` defaults to `.` (the framework repo itself
///   carries the codex tree); also reachable as the legacy
///   `--rules-root` alias.
/// - the scan profile is hard-coded to `framework`; no flag.
/// - `--target` is optional and defaults to the sentinel `none`
///   string (framework scans don't have a single target adapter).
/// - `--include-core` does not exist — `CORE-*` rules are always
///   visible to the framework run.
#[derive(Args)]
pub struct FrameworkArgs {
    /// Framework repo root used as both rules-root and scan-root.
    /// Defaults to the current directory so a contributor in a
    /// fresh clone can run bare `specify lint framework`.
    #[arg(long, env = "SPECIFY_ROOT", alias = "rules-root", default_value = ".")]
    pub framework_root: PathBuf,

    /// Target-adapter name (kebab, optionally `<name>@v<major>`).
    /// Defaults to the literal `none` because framework scans rarely
    /// scope to one target adapter; when supplied, narrows the
    /// applicability filter the same way `lint project --target` does.
    #[arg(long, default_value = "none")]
    pub target: String,

    /// Source-adapter name; repeatable. Each occurrence contributes
    /// one source overlay to the resolved codex.
    #[arg(long = "source", value_name = "NAME")]
    pub sources: Vec<String>,

    /// Restrict the declarative pass to specific rule ids (debug
    /// surface: `specify lint framework --rule CORE-001`).
    /// Repeatable; empty means "evaluate every applicable rule".
    /// Does not filter the imperative pass — authoring rule ids
    /// (`rules.schema-violation`, `skill.unknown-tool`, …) do not
    /// match the closed codex `rule-id` regex.
    #[arg(long = "rule", value_name = "RULE_ID")]
    pub rules: Vec<String>,

    /// Restrict the scan to specific artifact paths (lint scope
    /// resolution). Repeatable. Project-relative to `framework-root`.
    #[arg(long = "artifact", value_name = "PATH")]
    pub artifacts: Vec<PathBuf>,

    /// Lowercase language token; repeatable. Passed to both
    /// `build_resolved_rules` and the framework indexer.
    #[arg(long = "language", value_name = "TOKEN")]
    pub languages: Vec<String>,

    /// Emit the `WorkspaceModel` only (debug). Validates the model
    /// against `WORKSPACE_MODEL_JSON_SCHEMA` before stdout emit;
    /// skips hint evaluation entirely.
    #[arg(long)]
    pub dump_model: bool,

    /// Output format. Closed set per the diagnostics
    /// formatter set: `{ json, pretty, github, compact }`. When
    /// unset, derived from the global `--format` flag: `json` →
    /// `Json`, `text` → `Pretty`.
    ///
    /// Spelled `--output-format` rather than `--format` to avoid a
    /// clap conflict with the global `--format` flag on `Cli`
    /// (text vs JSON for the failure envelope).
    #[arg(long, value_enum)]
    pub output_format: Option<LintFormat>,
}

/// Flag surface for `specify lint project`. Grouped into one struct so the
/// handler threads a single reference instead of a positional argument
/// list.
#[derive(Args)]
pub struct ProjectArgs {
    /// Codex root supplying shared `UNI-*` rules. Resolution
    /// order (rules-root resolution): this flag / `$RULES_ROOT` env →
    /// monorepo `adapters/shared/rules/universal/` under the project →
    /// distributed out-of-tree codex cache `<project-cache>/codex/` (populated by
    /// `specify init` / `specify rules sync`). Missing every rung exits
    /// 2 with `rules-root-required`.
    #[arg(long, env = "RULES_ROOT")]
    pub rules_root: Option<PathBuf>,

    /// Target-adapter name (kebab, optionally `<name>@v<major>`).
    #[arg(long)]
    pub target: String,

    /// Source-adapter name; repeatable. Each occurrence
    /// contributes one source overlay to the resolved codex.
    #[arg(long = "source", value_name = "NAME")]
    pub sources: Vec<String>,

    /// Restrict the scan to one slice's tasks (lint scope resolution).
    /// Reads the slice's `tasks.md` for `Touches:` / `Produces:`
    /// paths plus `.specify/slices/<name>/**`.
    #[arg(long)]
    pub slice: Option<String>,

    /// Restrict the scan to specific artifact paths
    /// (lint scope resolution). Repeatable; composes with `--slice` (union).
    #[arg(long = "artifact", value_name = "PATH")]
    pub artifacts: Vec<PathBuf>,

    /// Lowercase language token; repeatable. Passed to both
    /// `specify rules export` and the consumer indexer.
    #[arg(long = "language", value_name = "TOKEN")]
    pub languages: Vec<String>,

    /// Emit the `WorkspaceModel` only (debug). Validates the
    /// model against `WORKSPACE_MODEL_JSON_SCHEMA` before
    /// stdout emit; skips hint evaluation entirely.
    #[arg(long)]
    pub dump_model: bool,

    /// Include `CORE-*` rules resolved from
    /// `adapters/shared/rules/core/`.
    /// Default off — consumer-project review runs never evaluate
    /// `CORE-*` hints unless the caller opts in.
    #[arg(long)]
    pub include_core: bool,

    /// Output format. Closed set per the diagnostics formatter set:
    /// `{ json, pretty, github, compact }`; default `pretty`.
    ///
    /// Spelled `--output-format` rather than `--format` to
    /// avoid a clap conflict with the global `--format` flag
    /// on the `Cli` (text vs JSON for the failure envelope).
    /// clap's `debug_asserts` refuses two flags whose long
    /// name OR derived id agree, so the per-subcommand axis
    /// gets its own field name (`output_format`) AND long
    /// name (`--output-format`).
    #[arg(long, default_value = "pretty")]
    pub output_format: LintFormat,

    /// Project directory used as the scan root (defaults to the
    /// current directory). The handler resolves the nearest
    /// ancestor that contains `.specify/project.yaml`.
    #[arg(long, default_value = ".")]
    pub project_dir: PathBuf,
}

/// Clap-derivable mirror of [`DiagnosticsFormat`] per the diagnostics formatter set.
///
/// Kept distinct from the `specify-standards` enum so the standards crate
/// stays runtime-agnostic; the [`From`] impl below is the single
/// adapter. The wire spelling matches the closed diagnostics formatter set
/// (`compact`, `github`, `json`, `pretty`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum LintFormat {
    /// Tab-separated one-line-per-finding shape.
    Compact,
    /// GitHub Actions workflow-annotation lines.
    Github,
    /// `DiagnosticReport` wire envelope; schema-validated before emit.
    Json,
    /// Terminal output with severity colour and source location.
    Pretty,
}

impl From<LintFormat> for DiagnosticsFormat {
    fn from(value: LintFormat) -> Self {
        match value {
            LintFormat::Compact => Self::Compact,
            LintFormat::Github => Self::Github,
            LintFormat::Json => Self::Json,
            LintFormat::Pretty => Self::Pretty,
        }
    }
}
