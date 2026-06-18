//! Orphan source detection for the `orphan-source` diagnostic.

use std::collections::HashSet;

use specify_diagnostics::{Diagnostic, Severity};

use super::ORPHAN_SOURCE;
use crate::change::plan::core::Plan;
use crate::change::plan::core::validate::plan_finding_structured;

/// Top-level `sources:` keys declared but not referenced by any entry.
///
/// The inverse of validate's `unknown-source`, which catches *entry
/// references* to undeclared keys; this catches *declarations* with no
/// references. The orphaned key is carried on the diagnostic's
/// structured evidence under `key`.
pub(super) fn detect(plan: &Plan) -> Vec<Diagnostic> {
    let mut referenced: HashSet<&str> = HashSet::new();
    for entry in &plan.entries {
        for binding in &entry.sources {
            referenced.insert(binding.source());
        }
    }
    let mut orphans: Vec<&str> = plan
        .sources
        .keys()
        .filter(|k| !referenced.contains(k.as_str()))
        .map(String::as_str)
        .collect();
    orphans.sort_unstable();
    orphans
        .into_iter()
        .map(|key| {
            plan_finding_structured(
                ORPHAN_SOURCE,
                Severity::Suggestion,
                format!(
                    "source key '{key}' is declared in the plan-level `sources:` map but no entry references it; either reference it from an entry's `sources:` list or remove the declaration"
                ),
                None,
                "orphan source key",
                serde_json::json!({ "key": key }),
            )
        })
        .collect()
}
