//! Directive-validation pass.
//!
//! Runs after hint evaluation and before envelope assembly. The pass
//! consumes [`crate::lint::WorkspaceModel::ignore_directives`] and the
//! current scan's finding set, stamps matching findings with
//! [`FindingStatus::Ignored`] (or [`FindingStatus::FalsePositive`]
//! when the directive's rationale carries the `false-positive:`
//! prefix), and mints synthetic `UNI-022` / `UNI-023` findings for
//! malformed or orphan directives.
//!
//! # Graceful degradation
//!
//! When the universal codex tree is absent the match-and-demote logic
//! runs unconditionally;
//! synthetic `UNI-022` / `UNI-023` emission is suppressed when the
//! corresponding rule did not resolve in the current scan. Consumer
//! projects without the shared codex tree see directives applied
//! with `disposition.directive` populated but never trip the
//! synthetic findings.
//!
//! # Determinism
//!
//! The pass sorts directives by `(path, line, rule_id)` before
//! iterating so the stamping order and synthetic ordering are stable
//! irrespective of the indexer's emission order. Findings are
//! mutated in place; the returned synthetics are appended by the
//! runner before envelope assembly.
//!
//! # Status-aware exit decision
//!
//! [`blocking_findings_present`] implements the exit and
//! presentation semantics rule: exit 2 fires only when at least one
//! finding has `severity ∈ {critical, important}` AND `status` is
//! `open` (treating an unset status as `open`). The helper is kept
//! standalone so the lint runner and unit tests share one source of
//! truth for the decision.

use std::collections::HashMap;

use specify_diagnostics::{
    Diagnostic, DiagnosticReport, DirectiveDisposition, DispositionSource, FindingDisposition,
    FindingEvidence, FindingLocation, FindingStatus,
};
use specify_error::{Error, Result};

use crate::lint::IgnoreDirective;
use crate::lint::eval::{SyntheticFinding, make_synthetic_finding};
use crate::rules::ResolvedRule;

/// Rationale prefix that demotes a matched finding to
/// [`FindingStatus::FalsePositive`] instead of
/// [`FindingStatus::Ignored`]. Case-sensitive.
const FALSE_POSITIVE_PREFIX: &str = "false-positive:";

/// Minimum rationale length. Shorter rationales parse cleanly but emit
/// [`UNI_022`].
const MIN_RATIONALE_LEN: usize = 16;

/// `UNI-022` — `ignore-directive-missing-rationale`.
const UNI_022: &str = "UNI-022";

/// `UNI-023` — `ignore-directive-orphan`.
const UNI_023: &str = "UNI-023";

/// Output of [`apply`].
///
/// Carries the synthetic `UNI-022` / `UNI-023` findings the runner
/// appends to the existing finding list plus the updated
/// `FIND-NNNN` counter so monotonic ids survive into any
/// post-directive emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoreOutcome {
    /// Synthetic `UNI-022` / `UNI-023` findings to append to the
    /// envelope. Ordered by `(directive_path, directive_line,
    /// directive_rule_id, UNI-022 before UNI-023)` so envelope output
    /// stays byte-stable irrespective of indexer emission order.
    pub synthetics: Vec<Diagnostic>,
    /// Next monotonic `FIND-NNNN` slot after minting the synthetics.
    pub next_id_counter: u64,
}

/// Apply the directive-validation pass to `findings`.
///
/// Walks `directives` in `(path, line, rule_id)` order, stamps every
/// finding whose `(path, line, rule_id)` matches with
/// [`FindingStatus::Ignored`] (or [`FindingStatus::FalsePositive`]
/// when the rationale begins with `false-positive:`) plus a
/// populated [`FindingDisposition::directive`], then mints synthetic
/// `UNI-022` / `UNI-023` findings.
///
/// `resolved_rules` carries severity metadata for the synthesised
/// findings; it also gates emission when the universal codex tree is
/// absent — when
/// `UNI-022` / `UNI-023` are absent from the resolved set the
/// matching synthetic is suppressed silently.
///
/// `next_id` seeds the `FIND-NNNN` counter from the runner so
/// synthetic ids stay monotonic with the hint-evaluator output;
/// [`IgnoreOutcome::next_id_counter`] returns the post-mint counter.
pub fn apply(
    findings: &mut [Diagnostic], directives: &[IgnoreDirective], resolved_rules: &[ResolvedRule],
    next_id: u64,
) -> IgnoreOutcome {
    let mut sorted: Vec<&IgnoreDirective> = directives.iter().collect();
    sorted.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });

    let resolved_map: HashMap<&str, &ResolvedRule> =
        resolved_rules.iter().map(|r| (r.rule_id.as_str(), r)).collect();

    let mut matched_flags: Vec<bool> = vec![false; sorted.len()];

    for (idx, directive) in sorted.iter().enumerate() {
        for finding in findings.iter_mut() {
            if !finding_matches_directive(finding, directive) {
                continue;
            }
            matched_flags[idx] = true;
            // First directive to match a finding wins so stamping is
            // deterministic when two directives nominate the same
            // `(path, line, rule_id)`; both still count as matched
            // so orphan emission stays silent for either.
            if finding.disposition.is_some() {
                continue;
            }
            let status = if directive
                .rationale
                .as_deref()
                .is_some_and(|r| r.starts_with(FALSE_POSITIVE_PREFIX))
            {
                FindingStatus::FalsePositive
            } else {
                FindingStatus::Ignored
            };
            finding.status = Some(status);
            finding.disposition = Some(FindingDisposition {
                source: DispositionSource::Directive,
                directive: Some(DirectiveDisposition {
                    path: directive.path.clone(),
                    line: directive.line,
                    rationale: directive.rationale.clone().unwrap_or_default(),
                }),
                since: None,
            });
        }
    }

    let mut next = next_id;
    let mut synthetics: Vec<Diagnostic> = Vec::new();
    for (idx, directive) in sorted.iter().enumerate() {
        let missing_rationale =
            directive.rationale.as_deref().is_none_or(|r| r.len() < MIN_RATIONALE_LEN);
        if missing_rationale && let Some(rule) = resolved_map.get(UNI_022) {
            synthetics.push(mint_synthetic(
                rule,
                directive,
                &mut next,
                "specify-ignore directive missing or short rationale".to_string(),
                format!(
                    "Directive for {rule_id} omits rationale or carries fewer than {MIN_RATIONALE_LEN} characters of rationale.",
                    rule_id = directive.rule_id,
                ),
                "Add a rationale of at least 16 characters explaining why the finding is being tolerated at this location.".to_string(),
            ));
        }

        if !matched_flags[idx]
            && let Some(rule) = resolved_map.get(UNI_023)
        {
            synthetics.push(mint_synthetic(
                rule,
                directive,
                &mut next,
                "specify-ignore directive does not match any finding".to_string(),
                format!(
                    "Directive references {rule_id} but no finding for that rule fires on the target line.",
                    rule_id = directive.rule_id,
                ),
                "Remove the directive, fix the rule id, or restore the underlying finding it was meant to suppress.".to_string(),
            ));
        }
    }

    IgnoreOutcome {
        synthetics,
        next_id_counter: next,
    }
}

/// Status-aware exit predicate.
///
/// Returns `true` when at least one finding has `severity ∈
/// {critical, important}` AND `status` of `open` (an unset `status`
/// is treated as `open`).
#[must_use]
pub fn blocking_findings_present(findings: &[Diagnostic]) -> bool {
    specify_diagnostics::blocking_present(findings)
}

/// Status-aware exit gate.
///
/// Returns `Ok(())` when no open `critical | important` findings
/// remain; otherwise returns [`Error::validation_failed`] with the
/// report summary counts in the detail string.
///
/// # Errors
///
/// Returns [`Error::validation_failed`] with code
/// `review-findings-present` when [`blocking_findings_present`]
/// is true for `report.findings`.
pub fn deny_blocking_findings(report: &DiagnosticReport) -> Result<()> {
    if !blocking_findings_present(&report.findings) {
        return Ok(());
    }
    let detail = format!(
        "critical={} important={} suggestion={} optional={}",
        report.summary.critical,
        report.summary.important,
        report.summary.suggestion,
        report.summary.optional,
    );
    Err(Error::validation_failed(
        "review-findings-present",
        "deterministic review surfaced open critical/important findings",
        detail,
    ))
}

fn finding_matches_directive(finding: &Diagnostic, directive: &IgnoreDirective) -> bool {
    let Some(rule_id) = finding.rule_id.as_deref() else {
        return false;
    };
    if rule_id != directive.rule_id {
        return false;
    }
    let Some(location) = finding.location.as_ref() else {
        return false;
    };
    if location.path != directive.path {
        return false;
    }
    location.line == Some(directive.target_line)
}

fn mint_synthetic(
    rule: &ResolvedRule, directive: &IgnoreDirective, next_id: &mut u64, title: String,
    impact: String, remediation: String,
) -> Diagnostic {
    let id = *next_id;
    *next_id = next_id.saturating_add(1);
    let location = FindingLocation {
        path: directive.path.clone(),
        line: Some(directive.line),
        column: None,
        end_line: None,
        end_column: None,
    };
    let evidence = FindingEvidence::Snippet {
        value: directive.raw.clone(),
    };
    let mut finding = make_synthetic_finding(SyntheticFinding {
        id_num: id,
        rule_id: rule.rule_id.as_str(),
        title,
        severity: rule.severity,
        location: Some(location),
        evidence,
        impact,
        remediation,
        target_adapter: None,
    });
    // Synthetic directive findings default to `open`; the runner's
    // status-aware exit decision treats `None` and `Some(Open)`
    // equivalently but stamping the explicit token keeps wire output
    // unambiguous.
    finding.status = Some(FindingStatus::Open);
    finding
}

#[cfg(test)]
mod tests;
