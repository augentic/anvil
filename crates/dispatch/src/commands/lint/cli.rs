//! Clap derive surface for `specify lint framework` — the hidden
//! framework CI tool on the native provisioning grammar.
//!
//! The per-subcommand `--format` flag is intentionally distinct from
//! the global `Cli::format` flag: global `--format` toggles JSON vs
//! text for envelope-emitting handlers and the failure path, while
//! `specify lint --output-format` selects the closed diagnostics
//! formatter set (`{ json, pretty, github, compact }`). The handler reads its own
//! per-subcommand flag and ignores the global one for the success
//! body.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use specify_diagnostics::Format as DiagnosticsFormat;

/// Verbs under `specify lint`.
#[derive(Debug, Subcommand)]
pub enum LintAction {
    /// Framework authoring lint over the `augentic/specify` repo.
    ///
    /// Composes the imperative `Check` predicates with the declarative
    /// deterministic-hint interpreter and emits one structured
    /// envelope per run. Defaults `--framework-root` to `.`, hard-codes
    /// the framework scan profile, and always evaluates `CORE-*` rules.
    /// Contributor surface — hidden from operator help.
    Framework(FrameworkArgs),
}

/// Flag surface for `specify lint framework`, with these pinned
/// defaults:
///
/// - `--framework-root` defaults to `.` (the framework repo itself
///   carries the codex tree); also reachable as the legacy
///   `--rules-root` alias.
/// - the scan profile is hard-coded to `framework`; no flag.
/// - `--target` is optional and defaults to the sentinel `none`
///   string (framework scans don't have a single target adapter).
/// - `--include-core` does not exist — `CORE-*` rules are always
///   visible to the framework run.
#[derive(Debug, Args)]
pub struct FrameworkArgs {
    /// Framework repo root used as both rules-root and scan-root.
    /// Defaults to the current directory so a contributor in a
    /// fresh clone can run bare `specify lint framework`.
    #[arg(long, env = "SPECIFY_ROOT", alias = "rules-root", default_value = ".")]
    pub framework_root: PathBuf,

    /// Target-adapter name (kebab, optionally `<name>@v<major>`).
    /// Defaults to the literal `none` because framework scans rarely
    /// scope to one target adapter; when supplied, narrows the
    /// rules applicability filter.
    #[arg(long, default_value = "none")]
    pub target: String,

    /// Source-adapter name; repeatable. Each occurrence contributes
    /// one source overlay to the resolved codex.
    #[arg(long = "source", value_name = "NAME")]
    pub sources: Vec<String>,

    /// Restrict the declarative pass to specific rule ids (debug
    /// surface: `specify lint framework --rule CORE-002`).
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
