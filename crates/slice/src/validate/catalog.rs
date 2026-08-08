//! Component-catalog drift gate. Cross-references every `component:`
//! directive on the slice's Evidence claims against the project-level
//! component catalog (`.emery/design-system/components.yaml`).

use diagnostics::{Artifact, Diagnostic};
use error::Result;
use project::config::Layout;
use serde_json::Value as JsonValue;

use crate::design_system::{ComponentStatus, ComponentsCatalog};
use crate::synthesis::evidence::EvidenceDoc;

/// Cross-reference Evidence `component:` directives against the
/// project-level component catalog when present.
///
/// A slug absent from the catalog or carrying `status: rejected`
/// yields a `slice-catalog-drift` finding; no catalog means the check
/// returns empty (opt-in). `evidence_docs` is the already-validated
/// typed Evidence set, so `evidence/*.yaml` is not re-read.
pub(super) fn catalog_drift(
    layout: Layout<'_>, evidence_docs: &[EvidenceDoc],
) -> Result<Vec<Diagnostic>> {
    let Some(catalog) = ComponentsCatalog::load(layout.project_dir())? else {
        return Ok(Vec::new());
    };

    let mut findings: Vec<Diagnostic> = Vec::new();

    for doc in evidence_docs {
        let source_key = &doc.source;

        for claim in &doc.document.claims {
            if let Some(slug) = claim.extras.get("component").and_then(JsonValue::as_str) {
                match catalog.status_of(slug) {
                    None => {
                        findings.push(catalog_drift_summary(&format!(
                            "evidence/{source_key}.yaml: claim carries `component: {slug}` \
                             but no entry exists in the component catalog"
                        )));
                    }
                    Some(ComponentStatus::Rejected) => {
                        findings.push(catalog_drift_summary(&format!(
                            "evidence/{source_key}.yaml: claim carries `component: {slug}` \
                             but the catalog entry has `status: rejected`"
                        )));
                    }
                    Some(ComponentStatus::Confirmed) => {}
                }
            }

            // `notes.candidate_component` is informational only — it
            // never triggers `slice-catalog-drift`; only hard
            // `component:` directives above are checked.
        }
    }

    findings.sort_by(|a, b| a.impact.cmp(&b.impact));
    Ok(findings)
}

fn catalog_drift_summary(detail: &str) -> Diagnostic {
    Diagnostic::violation(
        "slice-catalog-drift",
        "Evidence `component:` directives resolve to confirmed catalog entries",
        detail,
        Artifact::Specs,
        None,
    )
}
