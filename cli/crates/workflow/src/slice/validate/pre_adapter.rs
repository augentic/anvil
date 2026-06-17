//! Pre-adapter gate machinery: the slice-spec provenance scan, the
//! gate-collection bundle the orchestrator sequences, the per-slice
//! authority-override orphan gate, the source-key resolution helper,
//! and the non-blocking synopsis content-floor advisory.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use specify_diagnostics::{Artifact, Diagnostic};
use specify_error::{Error, Result};
use specify_model::discovery::Discovery;
use specify_model::spec::provenance::{self, ParsedSpec, RequirementTag};

use super::catalog::collect_catalog_drift_findings;
use super::decisions::collect_decision_gates;
use super::model_drift::model_drift_findings;
use super::spec_location::collect_spec_file_location_findings;
use super::{collect_spec_files, path_hint};
use crate::change::{Plan, orphan_authority_override_keys};
use crate::config::Layout;
use crate::schema::EvidenceDoc;

/// One parsed `spec.md` from the slice specs walk.
struct ScannedSpec {
    path: PathBuf,
    parsed: ParsedSpec,
}

/// `(req-ids, synthesis-tags, provenance-findings)` from [`scan_slice_specs`].
pub(super) type ScanSliceSpecsResult =
    (BTreeSet<String>, Vec<(String, RequirementTag)>, Vec<Diagnostic>);

/// Walk `<slice>/specs/**/*.md` once, parse each file, and fan out
/// REQ ids (all files), synthesis tags (annotated files only), and
/// provenance diagnostics (annotated files only).
pub(super) fn scan_slice_specs(
    slice_dir: &Path, source_keys: &BTreeSet<String>,
) -> Result<ScanSliceSpecsResult> {
    let specs_dir = slice_dir.join("specs");
    if !specs_dir.is_dir() {
        return Ok((BTreeSet::new(), Vec::new(), Vec::new()));
    }
    let spec_files = collect_spec_files(&specs_dir)?;
    if spec_files.is_empty() {
        return Ok((BTreeSet::new(), Vec::new(), Vec::new()));
    }

    let mut req_ids = BTreeSet::new();
    let mut synthesis_tags = Vec::new();
    let mut provenance_findings = Vec::new();

    for path in spec_files {
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.clone(),
            source,
        })?;
        let scanned = ScannedSpec {
            path,
            parsed: provenance::parse_spec_md(&text),
        };

        for req in &scanned.parsed.requirements {
            if !req.id.is_empty() {
                req_ids.insert(req.id.clone());
            }
        }
        if scanned.parsed.is_unannotated() {
            continue;
        }
        for (id, tag) in scanned.parsed.synthesis_tags() {
            synthesis_tags.push((id.to_string(), tag));
        }
        let path_hint = path_hint(&scanned.path, slice_dir);
        let validation_findings = provenance::validate(&scanned.parsed, source_keys);
        for f in scanned.parsed.findings.into_iter().chain(validation_findings) {
            provenance_findings.push(f.into_diagnostic(&path_hint));
        }
    }

    Ok((req_ids, synthesis_tags, provenance_findings))
}

/// Bundle the pre-adapter gates that fire on a single slice:
///
/// 1. Spec file-location check — root `spec.md` exists
///    but no canonical `specs/<domain>/spec.md` files found. Fires
///    first so the operator sees the structural cause before
///    downstream drift noise.
/// 2. per-slice authority override — orphan source keys on the slice's
///    `plan.yaml.slices[].authority-override` map.
/// 3. component catalog contract — catalog drift between Evidence `component:`
///    directives and `.specify/design-system/components.yaml`.
/// 4. typed-model drift — the seven drift-validation findings
///    over `<slice>/model.yaml` (skipped when absent).
///
/// Provenance has no file-drift gate: it is carried inline in
/// `model.yaml` and projected on demand (`specify slice provenance`),
/// so there is no second representation to drift against. Spec-level
/// `Sources:` / `Status:` coherence still runs in [`scan_slice_specs`].
///
/// All checks can fail independently; we collect every finding
/// into one [`Diagnostic`] vector so the caller can render the full
/// surface in one pass instead of one error per re-run.
pub(super) fn collect_pre_adapter_gates(
    layout: Layout<'_>, slice_dir: &Path, name: &str, evidence_docs: &[EvidenceDoc],
) -> Result<Vec<Diagnostic>> {
    let mut findings: Vec<Diagnostic> = Vec::new();
    findings.extend(collect_spec_file_location_findings(slice_dir));
    findings.extend(override_orphans(layout, name)?);
    findings.extend(collect_catalog_drift_findings(layout, evidence_docs)?);
    findings.extend(model_drift_findings(slice_dir, &layout.plan_path(), name, evidence_docs)?);
    findings.extend(collect_decision_gates(layout, slice_dir)?);
    Ok(findings)
}

/// Synopsis content-floor advisory (DECISIONS §Lead reconciliation). Loads
/// `<project_dir>/discovery.md` when present and emits one
/// non-blocking `discovery-lead-synopsis-thin` finding per lead whose
/// `synopsis` falls below a contentfulness heuristic. Absent
/// `discovery.md` skips the check silently.
///
/// **Non-blocking by design** — surfaced at `suggestion` severity
/// (`Diagnostic::review`), it never parks planning or transitions a
/// plan. A thin synopsis is a nudge to improve the source adapter's
/// `survey` brief output, not a gate: cross-source reconciliation is
/// only ever as good as the discriminating power of each lead's
/// `synopsis`, so the floor catches synopses the agent cannot match
/// or split on at `propose` time.
pub(super) fn synopsis_thin(layout: Layout<'_>) -> Result<Vec<Diagnostic>> {
    let path = layout.discovery_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let discovery = Discovery::load(&path)?;
    Ok(discovery
        .leads()
        .iter()
        .filter(|lead| synopsis_is_thin(&lead.synopsis))
        .map(|lead| {
            Diagnostic::review(
                "discovery-lead-synopsis-thin",
                "lead synopses should name behaviour distinctly enough to match or split on \
                 content, not just the slug",
                format!(
                    "lead `{}:{}` has a thin synopsis (`{}`); name the operation/surface and its \
                     salient constraint so a same-slug lead from another source can be \
                     reconciled on content",
                    lead.source,
                    lead.lead,
                    lead.synopsis.trim()
                ),
                Artifact::Plan,
                None,
            )
        })
        .collect())
}

/// Contentfulness heuristic for a lead `synopsis` (DECISIONS §Lead reconciliation).
/// A synopsis is "thin" when it carries fewer than
/// [`SYNOPSIS_MIN_WORDS`] whitespace-delimited words OR fewer than
/// [`SYNOPSIS_MIN_CHARS`] non-whitespace characters once trimmed — too
/// little for the agent to distinguish it from a same-slug lead in
/// another source. Deliberately coarse: the finding is advisory, so a
/// false positive costs the operator nothing.
pub(super) fn synopsis_is_thin(synopsis: &str) -> bool {
    let trimmed = synopsis.trim();
    let words = trimmed.split_whitespace().filter(|word| !word.is_empty()).count();
    let chars = trimmed.chars().filter(|character| !character.is_whitespace()).count();
    words < SYNOPSIS_MIN_WORDS || chars < SYNOPSIS_MIN_CHARS
}

/// Minimum whitespace-delimited word count below which a `synopsis` is
/// flagged as thin.
const SYNOPSIS_MIN_WORDS: usize = 4;

/// Minimum non-whitespace character count below which a `synopsis` is
/// flagged as thin.
const SYNOPSIS_MIN_CHARS: usize = 20;

/// per-slice authority override orphan-source gate. Loads `plan.yaml` (when
/// present) and reports one finding per `(slice, kind)` pair
/// whose source value is not in the slice's own `sources[]`
/// list. Absent `plan.yaml` (e.g. ad-hoc slice without a plan)
/// skips the check silently; the structural issue would already
/// have surfaced earlier in workflow.
fn override_orphans(layout: Layout<'_>, name: &str) -> Result<Vec<Diagnostic>> {
    let plan_path = layout.plan_path();
    if !plan_path.exists() {
        return Ok(Vec::new());
    }
    let plan = Plan::load(&plan_path)?;
    // Filter to the named slice only — `specify slice validate` is
    // per-slice by definition, and surfacing findings from other
    // slices would confuse the operator.
    let slice_entries: Vec<_> = plan.entries.iter().filter(|e| e.name == name).cloned().collect();
    let findings = orphan_authority_override_keys(&slice_entries);
    Ok(findings
        .into_iter()
        .map(|f| {
            Diagnostic::violation(
                f.rule_id.clone().unwrap_or_default(),
                "Per-slice `authority-override` source key must appear in the slice's \
                 `sources[]` list",
                f.impact,
                Artifact::Plan,
                None,
            )
        })
        .collect())
}

/// Resolve the plan-level source keys declared on the slice's plan
/// entry (`Plan.sources` is the universe of keys; the slice's
/// `sources[]` is the subset the slice actually binds). When no
/// `plan.yaml` exists, returns an empty set so the cross-validation
/// rule no-ops — structural rules still run.
pub(super) fn resolve_slice_source_keys(
    layout: Layout<'_>, name: &str,
) -> Result<BTreeSet<String>> {
    let plan_path = layout.plan_path();
    if !plan_path.exists() {
        return Ok(BTreeSet::new());
    }
    let plan = Plan::load(&plan_path)?;
    let Some(entry) = plan.entries.iter().find(|e| e.name == name) else {
        // Plan exists but the slice is not in it (e.g. operator
        // hand-created the slice). Fall back to the universe of plan
        // sources so the operator still gets useful validation
        // against any key spelt correctly.
        return Ok(plan.sources.keys().cloned().collect());
    };
    Ok(entry.sources.iter().map(|b| b.source().to_string()).collect())
}
