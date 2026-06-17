//! Component-catalog drift gate. Cross-references every `component:`
//! directive on the slice's Evidence claims against the project-level
//! component catalog (`.specify/design-system/components.yaml`).

use serde_json::Value as JsonValue;
use specify_diagnostics::{Artifact, Diagnostic};
use specify_error::Result;

use crate::config::Layout;
use crate::design_system::{ComponentStatus, ComponentsCatalog};
use crate::schema::EvidenceDoc;

/// component catalog contract catalog-drift gate. Loads the project-level component
/// catalog (`.specify/design-system/components.yaml`) when present
/// and cross-references every `component: <slug>` directive on
/// Evidence claims in `<slice>/evidence/*.yaml` against it:
///
/// - Slug absent from catalog → `slice-catalog-drift` finding.
/// - Slug has `status: rejected` → `slice-catalog-drift` finding.
///
/// Claims carrying a slug in `notes.candidate_component` are exempt
/// when the catalog entry is `rejected` — the operator has
/// intentionally declined the promotion, and the note is purely
/// informational.
///
/// When no catalog exists the check returns empty — the catalog is
/// opt-in.
///
/// `evidence_docs` is the parsed Evidence set the pre-adapter sweep
/// already read and schema-validated, so the `component:` directives
/// are inspected without re-reading `evidence/*.yaml`.
pub(super) fn collect_catalog_drift_findings(
    layout: Layout<'_>, evidence_docs: &[EvidenceDoc],
) -> Result<Vec<Diagnostic>> {
    let Some(catalog) = ComponentsCatalog::load(layout.project_dir())? else {
        return Ok(Vec::new());
    };

    let mut findings: Vec<Diagnostic> = Vec::new();

    for doc in evidence_docs {
        let source_key =
            doc.path.file_stem().and_then(|s| s.to_str()).unwrap_or("<unknown>").to_string();

        let Some(claims) = doc.value.get("claims").and_then(JsonValue::as_array) else {
            continue;
        };

        for claim in claims {
            if let Some(slug) = claim.get("component").and_then(JsonValue::as_str) {
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

            // `notes.candidate_component` is purely informational
            // (a hint from the adapter's stage-6 detection). It never
            // triggers `slice-catalog-drift` regardless of whether
            // the slug is in the catalog, absent, or rejected. Only
            // hard `component:` directives above are checked.
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
