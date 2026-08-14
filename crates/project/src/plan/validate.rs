//! `Plan::validate` and the per-check helpers it composes.
//!
//! Findings accumulate (no check short-circuits another): structural
//! checks first, then consistency checks.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use diagnostics::{
    Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, FindingEvidence, Severity, fingerprint,
};
use error::{Error, Result};
use petgraph::graph::DiGraph;

use super::model::{Divergence, Entry, Plan};

/// Build a plan-domain diagnostic on the neutral currency.
///
/// The stable check `code` becomes the `rule_id`, the offending plan
/// entry (when present) populates `slice`, and the finding is a
/// deterministic `Plan` artifact violation. The fingerprint is
/// recomputed after `slice` is set so dedup identity covers it.
/// `severity` is the neutral severity directly: pass
/// [`Severity::Important`] for a blocking structural error and
/// [`Severity::Suggestion`] for a non-blocking advisory.
#[must_use]
pub fn finding(
    code: &'static str, severity: Severity, message: impl Into<String>, entry: Option<String>,
) -> Diagnostic {
    let message = message.into();
    let mut diagnostic = Diagnostic::finding(
        code.to_string(),
        message.clone(),
        message,
        severity,
        DiagnosticKind::Violation,
        DiagnosticSource::Deterministic,
        Artifact::Plan,
        None,
    );
    diagnostic.slice = entry;
    diagnostic.fingerprint = fingerprint(&diagnostic);
    diagnostic
}

/// As [`finding`], but attaches a structured-evidence payload.
///
/// A health check carries its machine-readable data (the cycle path, the
/// orphan source key, the stale-clone signatures) onto the neutral
/// currency without loss. The fingerprint is recomputed after both
/// `slice` and the structured evidence are set.
#[must_use]
pub fn structured_finding(
    code: &'static str, severity: Severity, message: impl Into<String>, entry: Option<String>,
    summary: impl Into<String>, data: serde_json::Value,
) -> Diagnostic {
    let mut diagnostic = finding(code, severity, message, entry);
    diagnostic.evidence = FindingEvidence::Structured {
        summary: summary.into(),
        data,
        locations: None,
    };
    diagnostic.fingerprint = fingerprint(&diagnostic);
    diagnostic
}

impl Plan {
    /// Run all structural and semantic checks over the plan.
    ///
    /// `slices_dir` (when `Some`) enables the cross-reference checks
    /// against on-disk slice metadata. Findings accumulate — no check
    /// short-circuits another; structural checks run first. Exclusivity
    /// is per-slice claim only (no plan-wide in-progress cap).
    #[must_use]
    pub(crate) fn validate(&self, slices_dir: Option<&Path>) -> Vec<Diagnostic> {
        let mut results = Vec::new();
        results.extend(duplicate_names(&self.entries));
        results.extend(unknown_depends_on(&self.entries));
        results.extend(unknown_targets(self));
        results.extend(unknown_sources(self));
        results.extend(source_shape(self));
        results.extend(duplicate_source_keys(&self.entries));
        results.extend(context_paths(&self.entries));
        results.extend(orphan_authority_override(&self.entries));
        results.extend(divergence_consistency(&self.entries));
        if let Some(dir) = slices_dir.filter(|d| d.is_dir()) {
            results.extend(slices_dir_consistency(self, dir));
        }
        results
    }
}

fn duplicate_names(changes: &[Entry]) -> Vec<Diagnostic> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut out = Vec::new();
    for entry in changes {
        if !seen.insert(entry.name.as_str()) {
            out.push(finding(
                "duplicate-name",
                Severity::Important,
                format!("duplicate plan entry name '{}'", entry.name),
                Some(entry.name.to_string()),
            ));
        }
    }
    out
}

/// Build a `depends_on -> entry` dependency graph for plan entries.
///
/// Every entry becomes a node (in declaration order). For each
/// `entry.depends_on` target that names another entry, an edge runs
/// from the dependency node to `entry`.
#[must_use]
pub fn dependency_graph(entries: &[Entry]) -> DiGraph<&str, ()> {
    let mut graph: DiGraph<&str, ()> = DiGraph::new();
    let mut idx = HashMap::new();
    for entry in entries {
        let node = graph.add_node(entry.name.as_str());
        idx.insert(entry.name.as_str(), node);
    }
    for entry in entries {
        let to = idx[entry.name.as_str()];
        for dep in &entry.depends_on {
            if let Some(&from) = idx.get(dep.as_str()) {
                graph.add_edge(from, to, ());
            }
        }
    }
    graph
}

fn unknown_depends_on(changes: &[Entry]) -> Vec<Diagnostic> {
    let known: HashSet<&str> = changes.iter().map(|c| c.name.as_str()).collect();
    let mut out = Vec::new();
    for entry in changes {
        for target in &entry.depends_on {
            if !known.contains(target.as_str()) {
                out.push(finding(
                    "unknown-depends-on",
                    Severity::Important,
                    format!("depends-on references unknown slice '{target}'"),
                    Some(entry.name.to_string()),
                ));
            }
        }
    }
    out
}

fn unknown_targets(plan: &Plan) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for entry in &plan.entries {
        if !plan.targets.contains_key(&entry.target) {
            out.push(finding(
                "unknown-target",
                Severity::Important,
                format!("slice '{}' binds unknown target '{}'", entry.name, entry.target),
                Some(entry.name.to_string()),
            ));
        }
    }
    out
}

fn source_shape(plan: &Plan) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (key, binding) in &plan.sources {
        if let Err(Error::Diag { code, detail }) = binding.validate(key) {
            out.push(finding(code, Severity::Important, detail, None));
        }
    }
    out
}

fn unknown_sources(plan: &Plan) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for entry in &plan.entries {
        for binding in &entry.sources {
            let key = binding.source();
            if !plan.sources.contains_key(key) {
                out.push(finding(
                    "unknown-source",
                    Severity::Important,
                    format!("sources references unknown source key '{key}'"),
                    Some(entry.name.to_string()),
                ));
            }
        }
    }
    out
}

/// A slice binds at most one lead per source key: Evidence persists to
/// `evidence/<source>.yaml`, so a second lead under the same key would
/// silently overwrite the first at refine time. The propose kernel
/// rejects this shape at projection
/// (`plan-reconcile-slice-source-collision`); this check catches plans
/// reshaped after propose (e.g. via `plan amend`).
fn duplicate_source_keys(changes: &[Entry]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for entry in changes {
        let mut seen: HashSet<&str> = HashSet::new();
        for binding in &entry.sources {
            let key = binding.source();
            if !seen.insert(key) {
                out.push(finding(
                    "duplicate-source-key",
                    Severity::Important,
                    format!("slice '{}' binds source key '{key}' more than once", entry.name),
                    Some(entry.name.to_string()),
                ));
            }
        }
    }
    out
}

/// Post-mutation duplicate-source-key gate.
///
/// Runs `duplicate_source_keys` over `plan` and short-circuits
/// the CLI write with a single `Error::Validation` (exit 2) when any
/// finding fires. The additive `plan amend --add-source` path mutates
/// entry sources after [`Plan::amend`]'s own validate-and-rollback gate
/// has run, so the handler calls this afterwards; the wholesale
/// `--sources` replacement and `plan add` paths are already covered by
/// the validate folded into [`Plan::amend`] / [`Plan::create`].
///
/// # Errors
///
/// Returns `Error::Validation` (`duplicate-source-key`) when at least
/// one slice binds the same source key more than once.
pub fn reject_duplicate_source(plan: &Plan) -> Result<()> {
    let findings: Vec<_> =
        duplicate_source_keys(&plan.entries).into_iter().filter(diagnostics::is_blocking).collect();
    let Some(first) = findings.first() else {
        return Ok(());
    };
    let detail = findings.iter().map(|f| f.impact.clone()).collect::<Vec<_>>().join("; ");
    Err(Error::Validation {
        code: first.rule_id.clone().unwrap_or_default().into(),
        detail,
    })
}

/// Refuse orphan per-slice `authority-override` values.
///
/// Every override value MUST appear in that slice's `sources[].source`
/// list; otherwise synthesis would silently fall through to the
/// default authority. Findings sort by slice declaration order, then
/// claim kind. Public so `emery slice validate` can surface one
/// slice's findings.
#[must_use]
pub fn orphan_authority_override(changes: &[Entry]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for entry in changes {
        if entry.authority_override.by_kind.is_empty() {
            continue;
        }
        let known: BTreeSet<&str> =
            entry.sources.iter().map(super::model::SliceSourceBinding::source).collect();
        for (kind, key) in &entry.authority_override.by_kind {
            if !known.contains(key.as_str()) {
                out.push(finding(
                    "slice-authority-override-orphan-source",
                    Severity::Important,
                    format!(
                        "slice '{}' override for kind '{kind}' references source key '{key}', \
                         not present in slice sources",
                        entry.name
                    ),
                    Some(entry.name.to_string()),
                ));
            }
        }
    }
    out
}

/// Structural divergence consistency: the flag and the recorded values
/// must agree (the agent owns the *materiality* judgment).
///
/// A live divergence without adequate `disagreements[]` is
/// `slice-divergence-unrecorded`; values without a flag are
/// `slice-divergence-orphan-values`. Both stay advisory (`Suggestion`)
/// because `--divergence` is operator-settable standalone and neither
/// finding may block that write; `rejected` triggers neither check.
fn divergence_consistency(changes: &[Entry]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for entry in changes {
        let requires_values = entry.divergence.is_some_and(Divergence::requires_values);
        let recorded = !entry.disagreements.is_empty();
        let unflagged = matches!(entry.divergence, None | Some(Divergence::None));
        if requires_values {
            let adequate = recorded
                && entry.disagreements.iter().all(|d| {
                    d.values.iter().map(|v| v.source.as_str()).collect::<HashSet<_>>().len() >= 2
                });
            if !adequate {
                out.push(finding(
                    "slice-divergence-unrecorded",
                    Severity::Suggestion,
                    format!(
                        "slice '{}' flags divergence but records no disagreeing values \
                         (each field needs ≥2 distinct source values)",
                        entry.name
                    ),
                    Some(entry.name.to_string()),
                ));
            }
        } else if unflagged && recorded {
            out.push(finding(
                "slice-divergence-orphan-values",
                Severity::Suggestion,
                format!(
                    "slice '{}' records disagreements but is not flagged 'divergence'",
                    entry.name
                ),
                Some(entry.name.to_string()),
            ));
        }
    }
    out
}

fn context_paths(changes: &[Entry]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for entry in changes {
        for path in &entry.context {
            if path.starts_with('/') || path.contains("..") {
                out.push(finding(
                    "plan.context-path-invalid",
                    Severity::Important,
                    format!(
                        "entry '{}': context path '{}' must be relative to .emery/ (no '..' or absolute paths)",
                        entry.name, path
                    ),
                    Some(entry.name.to_string()),
                ));
            }
        }
    }
    out
}

fn slices_dir_consistency(plan: &Plan, slices_dir: &Path) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let declared: HashSet<&str> = plan.entries.iter().map(|c| c.name.as_str()).collect();

    let Ok(read_dir) = std::fs::read_dir(slices_dir) else {
        return out;
    };
    let mut dir_names: Vec<String> = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        dir_names.push(name.to_string());
    }
    dir_names.sort();

    for name in &dir_names {
        if !declared.contains(name.as_str()) {
            out.push(finding(
                "orphan-slice-dir",
                Severity::Suggestion,
                format!("slice directory '{name}' has no plan entry"),
                Some(name.clone()),
            ));
        }
    }

    out
}
