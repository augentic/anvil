//! Stale workspace-clone detection for the `stale-workspace-clone`
//! diagnostic.

use std::path::Path;

use specify_diagnostics::{Diagnostic, Severity};

use super::{CloneSignature, STALE_CLONE, StaleReason};
use crate::change::plan::core::validate::plan_finding_structured;
use crate::registry::workspace::{SlotProblem, SlotProblemReason, slot_problem};
use crate::registry::{Registry, RegistryProject};

/// Stale-slot diagnostics for every project whose materialisation drifted.
///
/// Emits one [`STALE_CLONE`] per registry project whose existing
/// workspace slot would be refused by `workspace sync`. Missing slots
/// are left to `workspace sync`; absent `.specify-sync.yaml` metadata
/// is ignored.
pub(super) fn detect(registry: &Registry, project_dir: &Path) -> Vec<Diagnostic> {
    let mut sorted: Vec<&RegistryProject> = registry.projects.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));

    let mut out = Vec::new();
    for project in sorted {
        if let Some(problem) = slot_problem(project_dir, project) {
            out.push(diag(project, &problem));
        }
    }
    out
}

fn diag(project: &RegistryProject, problem: &SlotProblem) -> Diagnostic {
    let expected = CloneSignature {
        slot_kind: Some(problem.expected_kind.to_string()),
        url: Some(project.url.clone()),
        // The registry `adapter` is only a greenfield seed, so it
        // is not part of the authoritative clone signature.
        adapter: project.adapter.clone(),
        target: problem.expected_target.as_ref().map(|path| path.display().to_string()),
    };
    let observed = CloneSignature {
        slot_kind: problem.observed_kind.map(|kind| kind.to_string()),
        url: problem.observed_url.clone(),
        adapter: None,
        target: problem.observed_target.as_ref().map(|path| path.display().to_string()),
    };
    let reason = if problem.reason == SlotProblemReason::RemoteOriginMismatch {
        StaleReason::SignatureChanged
    } else {
        StaleReason::SlotMismatch
    };
    plan_finding_structured(
        STALE_CLONE,
        Severity::Suggestion,
        format!(
            "workspace slot '{}' is out of sync with `registry.yaml`: {}",
            project.name,
            problem.message()
        ),
        None,
        "stale workspace clone",
        serde_json::json!({
            "project": project.name,
            "reason": reason,
            "expected": expected,
            "observed": observed,
        }),
    )
}
