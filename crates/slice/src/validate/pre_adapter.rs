//! Validation gates that run before adapter-specific checks.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use artifacts::discovery::Discovery;
use artifacts::spec::provenance::{self, ParsedSpec, RequirementTag};
use diagnostics::{Artifact, Diagnostic};
use error::Result;
use project::config::Layout;
use project::plan::{Plan, orphan_authority_override};

use super::catalog::catalog_drift;
use super::decisions::decision_gates;
use super::spec_location::file_location;
use super::{collect_spec_files, model_drift, path_hint};
use crate::synthesis::evidence::EvidenceDoc;

struct ScannedSpec {
    path: PathBuf,
    parsed: ParsedSpec,
}

/// `(req-ids, synthesis-tags, provenance-findings)` from [`scan_specs`].
pub(super) type ScanResult = (BTreeSet<String>, Vec<(String, RequirementTag)>, Vec<Diagnostic>);

/// Scan slice specs once for requirement ids, synthesis tags, and
/// provenance diagnostics.
pub(super) fn scan_specs(slice_dir: &Path, source_keys: &BTreeSet<String>) -> Result<ScanResult> {
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
        let text = project::fs::read_text(&path)?;
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

/// Run all pre-adapter gates for one slice.
///
/// File location runs first so structural failures precede derivative
/// drift noise. Independent findings are collected in one pass.
pub(super) fn gates(
    layout: Layout<'_>, slice_dir: &Path, name: &str, evidence_docs: &[EvidenceDoc],
) -> Result<Vec<Diagnostic>> {
    let mut findings: Vec<Diagnostic> = Vec::new();
    findings.extend(file_location(slice_dir));
    findings.extend(override_orphans(layout, name)?);
    findings.extend(catalog_drift(layout, evidence_docs)?);
    findings.extend(model_drift::findings(slice_dir, &layout.plan_path(), name, evidence_docs)?);
    findings.extend(decision_gates(layout, slice_dir)?);
    Ok(findings)
}

/// Report thin discovery synopses when `discovery.md` exists.
///
/// This remains advisory because the heuristic can produce false
/// positives; its purpose is to improve cross-source reconciliation.
pub(super) fn synopsis_thin(layout: Layout<'_>) -> Result<Vec<Diagnostic>> {
    let path = layout.discovery_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let discovery = Discovery::load(&path)?;
    Ok(discovery
        .leads()
        .iter()
        .filter(|lead| is_synopsis_thin(&lead.synopsis))
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

/// Coarse content floor for an advisory finding.
pub(super) fn is_synopsis_thin(synopsis: &str) -> bool {
    let trimmed = synopsis.trim();
    let words = trimmed.split_whitespace().filter(|word| !word.is_empty()).count();
    let chars = trimmed.chars().filter(|character| !character.is_whitespace()).count();
    words < SYNOPSIS_MIN_WORDS || chars < SYNOPSIS_MIN_CHARS
}

const SYNOPSIS_MIN_WORDS: usize = 4;

const SYNOPSIS_MIN_CHARS: usize = 20;

/// Report authority overrides that name sources outside this slice.
fn override_orphans(layout: Layout<'_>, name: &str) -> Result<Vec<Diagnostic>> {
    let plan_path = layout.plan_path();
    if !plan_path.exists() {
        return Ok(Vec::new());
    }
    let plan = Plan::load(&plan_path)?;
    // Validation must not surface findings from other slices.
    let slice_entries: Vec<_> = plan.entries.iter().filter(|e| e.name == name).cloned().collect();
    let findings = orphan_authority_override(&slice_entries);
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

/// Resolve source keys bound to this slice.
pub(super) fn source_keys(layout: Layout<'_>, name: &str) -> Result<BTreeSet<String>> {
    let plan_path = layout.plan_path();
    if !plan_path.exists() {
        return Ok(BTreeSet::new());
    }
    let plan = Plan::load(&plan_path)?;
    let Some(entry) = plan.entries.iter().find(|e| e.name == name) else {
        // Ad-hoc slices can still validate against known plan sources.
        return Ok(plan.sources.keys().cloned().collect());
    };
    Ok(entry.sources.iter().map(|b| b.source().to_string()).collect())
}
