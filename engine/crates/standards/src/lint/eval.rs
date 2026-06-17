//! Hint interpreter umbrella per the executable hint-kind contract
//! and §"Evaluation algorithm".
//!
//! v1 (Phase 2) ships the four executable hint kinds the contract lists
//! ([`HintKind::PathPattern`], [`HintKind::Schema`], [`HintKind::Regex`],
//! [`HintKind::Tool`]). The framework-convergence
//! family adds [`HintKind::ReferenceResolves`], [`HintKind::Unique`],
//! [`HintKind::SetCoverage`], [`HintKind::Cardinality`], and
//! [`HintKind::ConstantEq`] in the same family. Each rule's
//! hints are partitioned by kind and evaluated in the fixed order
//! `path-pattern → schema → reference-resolves → unique → set-coverage → cardinality → constant-eq → fenced-block → presence → field-grammar → cross-reference → cli-contract → regex → tool`
//! so the cheap filters narrow the candidate file set before the
//! subprocess boundary fires.
//!
//! When a rule carries multiple include `path-pattern` hints they UNION.
//! Hints whose `value` starts with `!` are exclusions applied after the
//! include union. When a rule carries only exclusions,
//! the starting set is every file in the model. When a rule carries zero
//! `path-pattern` hints the
//! candidate set defaults to every [`crate::lint::File`] in
//! [`crate::lint::WorkspaceModel`]; per-kind sub-evaluators apply
//! their own [`crate::lint::FileKind`] filter (e.g. regex skips
//! binaries).
//!
//! Every [`HintKind`] variant has an executable interpreter and the
//! partition is exhaustive over executable arms — no hint kind is
//! reserved.
//!
//! # Evidence cap (the structured evidence union)
//!
//! Every finding minted here passes through
//! [`specify_diagnostics::validate_evidence_size`] before `compute_fingerprint`
//! signs it. Snippet-evidence findings that exceed the 16 `KiB` cap are
//! truncated by halving the snippet value (clamped to a UTF-8 char
//! boundary) and appending a `…[truncated]` marker until the
//! serialised evidence object fits, then re-fingerprinted. Structured
//! evidence too large to inline collapses to
//! `{"truncated": true}`. Findings with [`specify_diagnostics::FindingEvidence::Digest`]
//! evidence above the cap are not produced by v1 evaluators; the
//! truncation loop bails on them rather than synthesising a bogus
//! payload.

pub mod cardinality;
pub mod cli_contract;
pub mod constant_eq;
pub mod cross_reference;
mod error;
pub mod fenced_block;
pub mod field_grammar;
mod finding;
pub mod path_pattern;
pub mod presence;
pub mod reference_resolves;
pub mod regex;
pub mod schema;
pub mod set_coverage;
#[cfg(test)]
pub(crate) mod testkit;
pub mod tool;
pub mod unique;

use std::fmt;
use std::path::{Path, PathBuf};

pub use error::HintError;
pub(crate) use finding::{SyntheticFinding, make_finding, make_synthetic_finding, restamp_finding};
use specify_diagnostics::Diagnostic;
use specify_error::Error as CliError;
pub use tool::{NoopToolRunner, ToolOutput, ToolRunError, ToolRunner};

use crate::lint::WorkspaceModel;
use crate::lint::contract::CliContract;
use crate::lint::diagnostics::map_hint_error;
use crate::rules::{HintKind, LintMode, ResolvedRule, RuleHint};

/// Per-rule output of [`evaluate_rules`].
#[derive(Debug, Clone)]
pub struct HintEvalOutcome {
    /// Findings minted for this rule's executable hints.
    pub findings: Vec<Diagnostic>,
    /// Finding-id counter passed into the next rule's evaluation so
    /// `FIND-NNNN` ids stay monotonic across rules in the same scan.
    pub next_id_counter: u64,
}

/// Borrowed evaluation environment shared by every per-kind arm.
///
/// Carries the indexed model, the project root, the tool runner
/// behind `kind: tool`, and the binary-injected CLI contract behind
/// `kind: cli-contract` (absent when the embedder injects none —
/// `cli-contract` hints then fail as unsupported).
#[derive(Clone, Copy)]
pub struct EvalEnv<'a> {
    /// Indexed workspace facts.
    pub model: &'a WorkspaceModel,
    /// Project root candidate paths are relative to.
    pub project_dir: &'a Path,
    /// Runner backing `kind: tool` hints.
    pub tool_runner: &'a dyn ToolRunner,
    /// Binary-injected CLI contract backing `kind: cli-contract`.
    pub cli_contract: Option<&'a CliContract>,
}

impl fmt::Debug for EvalEnv<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `tool_runner` is a trait object without `Debug`; surface its
        // presence without its contents.
        f.debug_struct("EvalEnv")
            .field("project_dir", &self.project_dir)
            .field("cli_contract", &self.cli_contract.is_some())
            .finish_non_exhaustive()
    }
}

/// Evaluate a single rule's hints against the workspace model, threaded
/// with a caller-owned [`schema::SchemaCache`] so a `kind: schema`
/// validator (and its resolved project path) is built once per lint run
/// rather than once per rule. [`evaluate_rules`] owns the run-scoped
/// cache.
///
/// Hints are partitioned by kind and run in the order
/// `path-pattern → schema → reference-resolves → unique → set-coverage → cardinality → constant-eq → fenced-block → presence → field-grammar → cross-reference → cli-contract → regex → tool`
/// per §"Evaluation algorithm".
/// `path-pattern` hits build the candidate file set the later kinds
/// consume.
///
/// `start_id_counter` seeds the `FIND-NNNN` id sequence; the caller
/// threads [`HintEvalOutcome::next_id_counter`] into the next call so
/// ids stay monotonic across rules.
fn evaluate_with_cache(
    rule: &ResolvedRule, hints: &[RuleHint], env: EvalEnv<'_>, start_id_counter: u64,
    schema_cache: &mut schema::SchemaCache,
) -> Result<HintEvalOutcome, HintError> {
    let mut findings: Vec<Diagnostic> = Vec::new();
    let mut next_id = start_id_counter;
    let model = env.model;
    let project_dir = env.project_dir;

    let partition = PartitionedHints::from_hints(hints);
    let candidates = build_candidate_set(rule, &partition.path_pattern, model)?;

    for hint in partition.schema {
        let mut new = schema::evaluate(
            rule,
            hint,
            &candidates,
            project_dir,
            model,
            &mut next_id,
            schema_cache,
        )?;
        findings.append(&mut new);
    }
    for hint in partition.reference_resolves {
        let mut new = reference_resolves::evaluate(rule, hint, &candidates, model, &mut next_id)?;
        findings.append(&mut new);
    }
    for hint in partition.unique {
        let mut new = unique::evaluate(rule, hint, &candidates, model, &mut next_id)?;
        findings.append(&mut new);
    }
    for hint in partition.set_coverage {
        let mut new = set_coverage::evaluate(rule, hint, &candidates, model, &mut next_id)?;
        findings.append(&mut new);
    }
    for hint in partition.cardinality {
        let mut new = cardinality::evaluate(rule, hint, &candidates, model, &mut next_id)?;
        findings.append(&mut new);
    }
    for hint in partition.constant_eq {
        let mut new = constant_eq::evaluate(rule, hint, &candidates, model, &mut next_id)?;
        findings.append(&mut new);
    }
    for hint in partition.fenced_block {
        let mut new = fenced_block::evaluate(rule, hint, &candidates, model, &mut next_id)?;
        findings.append(&mut new);
    }
    for hint in partition.presence {
        let mut new = presence::evaluate(rule, hint, &candidates, model, &mut next_id)?;
        findings.append(&mut new);
    }
    for hint in partition.field_grammar {
        let mut new = field_grammar::evaluate(rule, hint, &candidates, model, &mut next_id)?;
        findings.append(&mut new);
    }
    for hint in partition.cross_reference {
        let mut new = cross_reference::evaluate(rule, hint, &candidates, model, &mut next_id)?;
        findings.append(&mut new);
    }
    for hint in partition.cli_contract {
        let mut new = cli_contract::evaluate(
            rule,
            hint,
            &candidates,
            project_dir,
            model,
            env.cli_contract,
            &mut next_id,
        )?;
        findings.append(&mut new);
    }
    for hint in partition.regex {
        let mut new = regex::evaluate(rule, hint, &candidates, project_dir, model, &mut next_id)?;
        findings.append(&mut new);
    }
    for hint in partition.tool {
        let mut new =
            tool::evaluate(rule, hint, &candidates, project_dir, env.tool_runner, &mut next_id)?;
        findings.append(&mut new);
    }

    Ok(HintEvalOutcome {
        findings,
        next_id_counter: next_id,
    })
}

/// A rule's hints bucketed by [`HintKind`], in the fixed evaluation
/// order. Built once per rule by [`PartitionedHints::from_hints`] so the
/// evaluation driver stays a flat sequence of per-kind loops.
#[derive(Default)]
struct PartitionedHints<'a> {
    path_pattern: Vec<&'a RuleHint>,
    schema: Vec<&'a RuleHint>,
    reference_resolves: Vec<&'a RuleHint>,
    unique: Vec<&'a RuleHint>,
    set_coverage: Vec<&'a RuleHint>,
    cardinality: Vec<&'a RuleHint>,
    constant_eq: Vec<&'a RuleHint>,
    fenced_block: Vec<&'a RuleHint>,
    presence: Vec<&'a RuleHint>,
    field_grammar: Vec<&'a RuleHint>,
    cross_reference: Vec<&'a RuleHint>,
    cli_contract: Vec<&'a RuleHint>,
    regex: Vec<&'a RuleHint>,
    tool: Vec<&'a RuleHint>,
}

impl<'a> PartitionedHints<'a> {
    fn from_hints(hints: &'a [RuleHint]) -> Self {
        let mut partition = Self::default();
        for hint in hints {
            match hint.kind {
                HintKind::PathPattern => partition.path_pattern.push(hint),
                HintKind::Schema => partition.schema.push(hint),
                HintKind::ReferenceResolves => partition.reference_resolves.push(hint),
                HintKind::Unique => partition.unique.push(hint),
                HintKind::SetCoverage => partition.set_coverage.push(hint),
                HintKind::Cardinality => partition.cardinality.push(hint),
                HintKind::ConstantEq => partition.constant_eq.push(hint),
                HintKind::FencedBlock => partition.fenced_block.push(hint),
                HintKind::Presence => partition.presence.push(hint),
                HintKind::FieldGrammar => partition.field_grammar.push(hint),
                HintKind::CrossReference => partition.cross_reference.push(hint),
                HintKind::CliContract => partition.cli_contract.push(hint),
                HintKind::Regex => partition.regex.push(hint),
                HintKind::Tool => partition.tool.push(hint),
            }
        }
        partition
    }
}

/// Fold the per-rule `evaluate_with_cache` kernel over every rule,
/// accumulating findings and the `FIND-NNNN` id counter.
///
/// Shared by both lint surfaces (`specify lint project` and `specify lint framework`)
/// so the per-rule gating stays identical: rules in `lint-mode:
/// model-assisted` and rules with no (or empty) `rule_hints`
/// are skipped; `start_id` is threaded forward so ids stay monotonic.
///
/// `rule_filter` is the operator's allow-list: EMPTY means no filtering
/// (runtime), non-empty keeps only rules whose `rule_id` matches
/// verbatim (the `specify lint framework --rules` surface — exact, case-sensitive).
///
/// Per-rule [`HintError`]s are mapped through [`map_hint_error`] here so
/// both call sites collapse to one fallible call.
///
/// # Errors
///
/// The [`map_hint_error`] mapping of the first rule whose
/// `evaluate_with_cache` call fails.
pub fn evaluate_rules(
    rules: &[ResolvedRule], env: EvalEnv<'_>, start_id: u64, rule_filter: &[&str],
) -> Result<(Vec<Diagnostic>, u64), CliError> {
    let mut findings: Vec<Diagnostic> = Vec::new();
    let mut next_id = start_id;
    // One cache per run: a schema referenced by many rules compiles once.
    let mut schema_cache = schema::SchemaCache::default();

    for rule in rules {
        if !rule_filter.is_empty() && !rule_filter.contains(&rule.rule_id.as_str()) {
            continue;
        }
        // `lint-mode: model-assisted` rules carry no executable hints
        // the deterministic engine can score. Rather than silently
        // dropping them, surface each as a non-blocking `kind: review`
        // diagnostic so the "needs judgment" signal stays first-class
        // on the wire (the model scorer / human reviewer picks it up).
        if matches!(rule.lint_mode, Some(LintMode::ModelAssisted)) {
            findings.push(finding::make_review_finding(rule, next_id));
            next_id += 1;
            continue;
        }
        let Some(hints) = rule.rule_hints.as_deref() else {
            continue;
        };
        if hints.is_empty() {
            continue;
        }
        let outcome = evaluate_with_cache(rule, hints, env, next_id, &mut schema_cache)
            .map_err(|err| map_hint_error(rule, err))?;
        findings.extend(outcome.findings);
        next_id = outcome.next_id_counter;
    }

    Ok((findings, next_id))
}

fn build_candidate_set(
    rule: &ResolvedRule, path_pattern_hints: &[&RuleHint], model: &WorkspaceModel,
) -> Result<Vec<PathBuf>, HintError> {
    if path_pattern_hints.is_empty() {
        let mut paths: Vec<PathBuf> = model.files.iter().map(|f| PathBuf::from(&f.path)).collect();
        paths.sort();
        return Ok(paths);
    }

    let (excludes, includes): (Vec<&RuleHint>, Vec<&RuleHint>) =
        path_pattern_hints.iter().copied().partition(|hint| path_pattern::is_exclusion(hint));

    let mut set: std::collections::BTreeSet<PathBuf> = std::collections::BTreeSet::new();
    if includes.is_empty() {
        for file in &model.files {
            set.insert(PathBuf::from(&file.path));
        }
    } else {
        for hint in &includes {
            for path in path_pattern::evaluate(rule, hint, model)? {
                set.insert(path);
            }
        }
    }
    for hint in &excludes {
        for path in path_pattern::evaluate(rule, hint, model)? {
            set.remove(&path);
        }
    }
    Ok(set.into_iter().collect())
}

/// Render the candidate `PathBuf` slice into the `/`-relative string
/// set the fact-iterating sub-evaluators test membership against.
///
/// Every fact-iterating kind (`set-coverage`, `constant-eq`,
/// `reference-resolves`, `unique`, `cardinality`)
/// narrows its facts to the `path-pattern` candidate set by string
/// path. Sharing the conversion keeps that lookup identical across
/// kinds.
pub(crate) fn candidate_set(candidates: &[PathBuf]) -> std::collections::BTreeSet<String> {
    candidates.iter().map(|p| p.to_string_lossy().into_owned()).collect()
}
