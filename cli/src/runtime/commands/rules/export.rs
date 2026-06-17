//! `specify rules export` handler — `ResolvedRules` export contract.
//!
//! Read-only. Builds the `ResolveInputs` struct from CLI args,
//! delegates to [`specify_standards::build_resolved_rules`], and
//! streams the resulting envelope to stdout as JSON. v1 supports
//! JSON output only; the global `--format text` default at the
//! `Cli` level surfaces as `Error::Argument` (exit 2) so the
//! JSON-only contract is explicit.
//!
//! Failure modes round-trip through `Exit::from(&Error)` per
//! `docs/standards/handler-shape.md`:
//!
//! | `ResolveError` variant   | `Error` variant         | Exit |
//! |--------------------------|-------------------------|------|
//! | `RulesRootRequired`      | `Validation`            | 2    |
//! | `DuplicateRuleId`        | `Validation`            | 2    |
//! | `Parse`                  | `Validation`            | 2    |
//! | `Filesystem`             | `Filesystem { op }`     | 1    |

use specify_error::{Error, Result};
use specify_standards::{ResolveInputs, build_resolved_rules, map_resolve_error};

use crate::runtime::cli::Format;
use crate::runtime::commands::rules::cli::ExportArgs;
use crate::runtime::output;

/// Run the export against the parsed clap argument group.
pub fn run(format: Format, args: &ExportArgs) -> Result<()> {
    require_json(format)?;

    let inputs = ResolveInputs {
        project_dir: &args.project_dir,
        rules_root: args.rules_root.as_deref(),
        target_adapter: &args.target,
        source_adapters: &args.sources,
        artifact_paths: &args.artifacts,
        languages: &args.languages,
        include_deprecated: args.include_deprecated,
        include_unmatched: args.include_unmatched,
        include_core: args.include_core,
    };

    let resolved = build_resolved_rules(&inputs).map_err(map_resolve_error)?;

    output::emit(&mut std::io::stdout().lock(), Format::Json, &resolved, |_, _| Ok(()))?;
    Ok(())
}

/// Reject `--format text` with an argument-shape error (exit 2).
fn require_json(format: Format) -> Result<()> {
    match format {
        Format::Json => Ok(()),
        Format::Text => Err(Error::Argument {
            flag: "--format",
            detail: "specify rules export currently emits JSON only; rerun with --format json"
                .to_string(),
        }),
    }
}
