//! Shared lint pipeline runner.
//!
//! Both lint surfaces (`specify lint project` and `specify lint framework`) compose the
//! identical sequence: resolve the codex, index the workspace, evaluate
//! the declarative deterministic hints, dedupe by fingerprint, apply the
//! ignore-directive pass, and assemble the [`DiagnosticReport`] envelope.
//! This module owns that sequence so the two handlers stay thin and
//! cannot drift.
//!
//! The surfaces differ only in configuration — scan profile and tool
//! runner — which [`PipelineConfig`] captures. Handler-specific concerns
//! (artifact scope composition, fallback-envelope emission,
//! `lint-completed` journalling, exit-code mapping) stay in the handlers.
//!
//! Codex resolution is always fatal: a resolver failure (missing rules
//! root, duplicate rule id, parse error) aborts the run and surfaces the
//! error, on both surfaces. There is no longer a degraded "skip the
//! declarative pass" mode — every check resolves through declarative
//! hints + referenced tools, so skipping the pass would silently pass a
//! broken codex.

use std::collections::HashSet;
use std::fmt;

use specify_diagnostics::{
    Diagnostic, DiagnosticReport, DiagnosticReportVersion, DiagnosticSummary,
};
use specify_error::Result;

use crate::lint::ScanProfile;
use crate::lint::contract::CliContract;
use crate::lint::diagnostics::{emit_dump_model, map_index_error};
use crate::lint::eval::tool::ToolRunner;
use crate::lint::eval::{EvalEnv, evaluate_rules};
use crate::lint::ignore::apply as apply_directives;
use crate::lint::index::build as build_model;
use crate::rules::{ResolveInputs, build_resolved_rules, map_resolve_error};

/// Configuration for one [`run`] of the shared lint pipeline.
pub struct PipelineConfig<'a> {
    /// Indexer profile (`Project` for `specify lint project`, `Framework` for
    /// `specify lint framework`).
    pub profile: ScanProfile,
    /// When set, emit the indexed `WorkspaceModel` and stop before the
    /// evaluator pass.
    pub dump_model: bool,
    /// Apply the ignore-directive demotion pass. Always `true` for the
    /// lint surfaces; lifecycle gates set this `false` so validation
    /// stays non-silenceable.
    pub apply_ignore_directives: bool,
    /// Operator allow-list of `rule_id`s (empty means no filtering).
    pub rule_filter: &'a [&'a str],
    /// Extension runner backing `kind: tool` hints.
    pub tool_runner: &'a dyn ToolRunner,
    /// Binary-injected CLI contract backing `kind: cli-contract`
    /// hints. The root binary builds it (clap introspection + const
    /// tables); embedders without a contract pass `None` and any
    /// `cli-contract` hint fails as unsupported.
    pub cli_contract: Option<&'a CliContract>,
}

impl fmt::Debug for PipelineConfig<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The trait-object field (`tool_runner`) is not `Debug`; surface
        // its presence without its contents.
        f.debug_struct("PipelineConfig")
            .field("profile", &self.profile)
            .field("dump_model", &self.dump_model)
            .field("apply_ignore_directives", &self.apply_ignore_directives)
            .field("rule_filter", &self.rule_filter)
            .field("cli_contract", &self.cli_contract.is_some())
            .finish_non_exhaustive()
    }
}

/// Outcome of [`run`].
#[derive(Debug)]
pub enum RunOutcome {
    /// A fully composed envelope ready to render.
    Report(DiagnosticReport),
    /// `--dump-model` short-circuit; the model has already been
    /// emitted to stdout.
    DumpedModel,
}

/// Resolve, index, evaluate, and assemble one lint envelope.
///
/// # Errors
///
/// - [`map_resolve_error`] when codex resolution fails.
/// - [`map_index_error`] when the indexer walk fails.
/// - The hint-evaluator mapping when a deterministic hint fails.
/// - A render/serialise error when `--dump-model` emit fails.
pub fn run(inputs: &ResolveInputs<'_>, config: &PipelineConfig<'_>) -> Result<RunOutcome> {
    let resolved = build_resolved_rules(inputs).map_err(map_resolve_error)?;

    let model =
        build_model(inputs.project_dir, config.profile, inputs.artifact_paths, inputs.languages)
            .map_err(map_index_error)?;

    if config.dump_model {
        emit_dump_model(&model)?;
        return Ok(RunOutcome::DumpedModel);
    }

    let env = EvalEnv {
        model: &model,
        project_dir: inputs.project_dir,
        tool_runner: config.tool_runner,
        cli_contract: config.cli_contract,
    };
    let (mut combined, mut next_id) = evaluate_rules(&resolved.rules, env, 1, config.rule_filter)?;

    deduplicate_by_fingerprint(&mut combined);

    if config.apply_ignore_directives {
        let outcome =
            apply_directives(&mut combined, &model.ignore_directives, &resolved.rules, next_id);
        combined.extend(outcome.synthetics);
        next_id = outcome.next_id_counter;
    }

    // `next_id` is intentionally not consumed further in v1; future
    // post-passes (baseline matching, telemetry ids) continue to
    // thread it.
    let _ = next_id;

    let result = DiagnosticReport {
        version: DiagnosticReportVersion,
        summary: DiagnosticSummary::from_diagnostics(&combined),
        findings: combined,
    };
    Ok(RunOutcome::Report(result))
}

/// Deduplicate `findings` by canonical fingerprint, preserving
/// first-occurrence order. When two rules emit a byte-identical
/// fingerprint for the same `(rule-id, location)` pair this collapse
/// keeps a single envelope row; distinct findings never share a
/// fingerprint.
fn deduplicate_by_fingerprint(findings: &mut Vec<Diagnostic>) {
    let mut seen: HashSet<String> = HashSet::with_capacity(findings.len());
    findings.retain(|f| seen.insert(f.fingerprint.clone()));
}
