//! [`Plan::validate`] and the per-check helpers it composes. Findings
//! accumulate (no check short-circuits another); order is structural
//! checks first, then consistency checks against the registry.
//!
//! Every check emits a neutral [`specify_diagnostics::Diagnostic`] via
//! [`plan_finding`]: the stable check code becomes the `rule_id`, the
//! offending plan entry (when present) populates `slice`, an `error`
//! maps to a blocking `important` violation, and a `warning` maps to a
//! non-blocking `suggestion`.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use petgraph::graph::DiGraph;
use specify_diagnostics::{
    Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, FindingEvidence, Severity, fingerprint,
};
use specify_error::{Error, Result};

use super::model::{Divergence, Entry, Plan, Status};
use crate::registry::Registry;

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
pub fn plan_finding(
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

/// As [`plan_finding`], but attaches a structured-evidence payload.
///
/// A health check carries its machine-readable data (the cycle path, the
/// orphan source key, the stale-clone signatures) onto the neutral
/// currency without loss. The fingerprint is recomputed after both
/// `slice` and the structured evidence are set.
#[must_use]
pub fn plan_finding_structured(
    code: &'static str, severity: Severity, message: impl Into<String>, entry: Option<String>,
    summary: impl Into<String>, data: serde_json::Value,
) -> Diagnostic {
    let mut diagnostic = plan_finding(code, severity, message, entry);
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
    /// `slices_dir` (when `Some`) points at `.specify/slices/` and
    /// enables the cross-reference checks against on-disk slice
    /// metadata. `registry` (when `Some`) enables the cross-registry
    /// checks (`project-not-in-registry`).
    ///
    /// Findings are accumulated — no check short-circuits another. Order
    /// is structural checks first (duplicate names, unknown
    /// depends-on / sources, duplicate source keys, multiple
    /// in-progress) followed by consistency checks against `slices_dir`
    /// when provided.
    ///
    /// Note on "well-formed status values": `Status` is an enum, so
    /// every in-memory instance is well-formed by construction. serde
    /// rejects invalid statuses at parse time, which is not reachable
    /// in-process — so nothing is emitted for it.
    #[must_use]
    pub fn validate(
        &self, slices_dir: Option<&Path>, registry: Option<&Registry>,
    ) -> Vec<Diagnostic> {
        let mut results = Vec::new();
        results.extend(duplicate_names(&self.entries));
        results.extend(check_unknown_depends_on(&self.entries));
        results.extend(check_unknown_sources(self));
        results.extend(check_duplicate_source_keys(&self.entries));
        results.extend(check_single_in_progress(&self.entries));
        results.extend(check_context_paths(&self.entries));
        results.extend(orphan_authority_override_keys(&self.entries));
        results.extend(check_divergence_consistency(&self.entries));
        if let Some(reg) = registry {
            results.extend(check_project_in_registry(&self.entries, reg));
            results.extend(check_project_binding_required(&self.entries, reg));
        }
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
            out.push(plan_finding(
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
pub fn entry_dependency_graph(entries: &[Entry]) -> DiGraph<&str, ()> {
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

fn check_unknown_depends_on(changes: &[Entry]) -> Vec<Diagnostic> {
    let known: HashSet<&str> = changes.iter().map(|c| c.name.as_str()).collect();
    let mut out = Vec::new();
    for entry in changes {
        for target in &entry.depends_on {
            if !known.contains(target.as_str()) {
                out.push(plan_finding(
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

fn check_unknown_sources(plan: &Plan) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for entry in &plan.entries {
        for binding in &entry.sources {
            let key = binding.source();
            if !plan.sources.contains_key(key) {
                out.push(plan_finding(
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
fn check_duplicate_source_keys(changes: &[Entry]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for entry in changes {
        let mut seen: HashSet<&str> = HashSet::new();
        for binding in &entry.sources {
            let key = binding.source();
            if !seen.insert(key) {
                out.push(plan_finding(
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
/// Runs `check_duplicate_source_keys` over `plan` and short-circuits
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
pub fn reject_duplicate_source_keys(plan: &Plan) -> Result<()> {
    let findings: Vec<_> = check_duplicate_source_keys(&plan.entries)
        .into_iter()
        .filter(specify_diagnostics::blocking)
        .collect();
    let Some(first) = findings.first() else {
        return Ok(());
    };
    let detail = findings.iter().map(|f| f.impact.clone()).collect::<Vec<_>>().join("; ");
    Err(Error::Validation {
        code: first.rule_id.clone().unwrap_or_default().into(),
        detail,
    })
}

fn check_single_in_progress(changes: &[Entry]) -> Vec<Diagnostic> {
    let offenders: Vec<&Entry> =
        changes.iter().filter(|c| c.status == Status::InProgress).collect();
    if offenders.len() <= 1 {
        return Vec::new();
    }
    offenders
        .into_iter()
        .map(|c| {
            plan_finding(
                "multiple-in-progress",
                Severity::Important,
                "multiple in-progress entries: at most one allowed per plan",
                Some(c.name.to_string()),
            )
        })
        .collect()
}

fn check_project_in_registry(changes: &[Entry], registry: &Registry) -> Vec<Diagnostic> {
    let project_names: HashSet<&str> = registry.projects.iter().map(|p| p.name.as_str()).collect();
    let mut out = Vec::new();
    for entry in changes {
        if let Some(project) = &entry.project
            && !project_names.contains(project.as_str())
        {
            out.push(plan_finding(
                "project-not-in-registry",
                Severity::Important,
                format!(
                    "project '{}' on slice '{}' does not match any project in registry.yaml",
                    project, entry.name
                ),
                Some(entry.name.to_string()),
            ));
        }
    }
    out
}

/// A slice may omit `project` only when the topology offers exactly one
/// project (the kernel and [`super::resolve_target`] auto-bind it). When
/// the registry declares more than one project an omitted `project` is
/// ambiguous, so flag it early rather than waiting for `plan next` to
/// fail with `plan-reconcile-project-binding-required`.
///
/// The single-regular-project case (no registry) is not reached here —
/// an omitted `project` there always resolves to the sole synthesised
/// project.
fn check_project_binding_required(changes: &[Entry], registry: &Registry) -> Vec<Diagnostic> {
    if registry.projects.len() <= 1 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for entry in changes {
        if entry.project.is_none() {
            out.push(plan_finding(
                "plan-reconcile-project-binding-required",
                Severity::Important,
                format!(
                    "entry '{}' omits 'project' but the registry declares {} projects; \
                     bind one explicitly",
                    entry.name,
                    registry.projects.len()
                ),
                Some(entry.name.to_string()),
            ));
        }
    }
    out
}

/// per-slice authority override — refuse orphan per-slice `authority-override` values.
///
/// For every slice's override map, every value MUST appear in that
/// slice's `sources[].source` list; otherwise the operator has named a
/// source key that does not exist on the slice, and synthesis would
/// silently fall through to the default authority. Findings sort
/// deterministically by slice name (declaration order) then by
/// claim kind (the `BTreeMap` iteration order on
/// [`super::model::SliceAuthorityOverride::by_kind`]).
///
/// Public for the per-slice helper at `specify slice validate` to
/// surface only the findings relevant to one slice.
#[must_use]
pub fn orphan_authority_override_keys(changes: &[Entry]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for entry in changes {
        if entry.authority_override.by_kind.is_empty() {
            continue;
        }
        let known: BTreeSet<&str> =
            entry.sources.iter().map(super::model::SliceSourceBinding::source).collect();
        for (kind, key) in &entry.authority_override.by_kind {
            if !known.contains(key.as_str()) {
                out.push(plan_finding(
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

/// Structural divergence consistency (RFC-46 D4). The agent owns the
/// *materiality* judgment; the CLI only checks that the flag and the
/// recorded values agree:
///
/// - a slice that flags a live divergence (`likely` / `accepted`) but
///   records no adequate `disagreements[]` (non-empty, ≥2 distinct
///   source values per field) is incomplete
///   (`slice-divergence-unrecorded`);
/// - a slice that records `disagreements[]` without a divergence flag is
///   an orphan record (`slice-divergence-orphan-values`).
///
/// Both are advisory (`Suggestion`): `divergence` is operator-settable
/// standalone via `specify plan amend --divergence` (a contract-locked,
/// advisory-metadata-in-v1 surface), so neither finding may block that
/// write. They surface the inconsistency at `plan validate` / Gate 1.
/// `rejected` carries no obligation (the plan is to be re-proposed), so
/// it triggers neither check.
fn check_divergence_consistency(changes: &[Entry]) -> Vec<Diagnostic> {
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
                out.push(plan_finding(
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
            out.push(plan_finding(
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

fn check_context_paths(changes: &[Entry]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for entry in changes {
        for path in &entry.context {
            if path.starts_with('/') || path.contains("..") {
                out.push(plan_finding(
                    "plan.context-path-invalid",
                    Severity::Important,
                    format!(
                        "entry '{}': context path '{}' must be relative to .specify/ (no '..' or absolute paths)",
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
            out.push(plan_finding(
                "orphan-slice-dir",
                Severity::Suggestion,
                format!("slice directory '{name}' has no plan entry"),
                Some(name.clone()),
            ));
        }
    }

    for entry in &plan.entries {
        if entry.status == Status::InProgress {
            let candidate = slices_dir.join(entry.name.as_str());
            if !candidate.is_dir() {
                out.push(plan_finding(
                    "missing-slice-dir-for-in-progress",
                    Severity::Suggestion,
                    format!(
                        "in-progress entry '{}' has no slice directory (may briefly be absent during phase start-up)",
                        entry.name
                    ),
                    Some(entry.name.to_string()),
                ));
            }
        }
    }

    out
}

#[cfg(test)]
mod tests;
