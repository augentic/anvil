//! Decision Record gates over the
//! slice's `decisions/*.md`: parser-owned per-file findings plus the
//! cross-file slug-collision and supersede-orphan checks.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use specify_diagnostics::{Artifact, Diagnostic, FindingLocation};
use specify_error::{Error, Result};
use specify_model::decision::{DecisionRecord, parse_decision};

use super::path_hint;
use crate::config::Layout;
use crate::decisions::{is_dec_ref, list_md_files, read_baseline};

/// Decision Record gate over
/// `<slice>/decisions/*.md` it raises the per-file findings owned by the
/// `specify-model` parser — `decision-record-schema`,
/// `decision-record-section-missing`, `decision-slug-grammar` (the same
/// parser-drives-findings posture as the `spec.md` provenance parser, so
/// no JSON schema runs here) — plus the two cross-file checks the parser
/// cannot make alone:
///
/// - `decision-slug-collision` — two records in the slice share a `slug`.
/// - `decision-supersede-orphan` — a `supersedes:` target resolves to
///   neither the live baseline catalogue nor a sibling slice record.
///   Re-checked against the live baseline at merge (the baseline may move
///   between refine and merge).
///
/// Absent `decisions/` skips the gate silently — Decision Records are
/// opt-in.
pub(super) fn collect_decision_gates(
    layout: Layout<'_>, slice_dir: &Path,
) -> Result<Vec<Diagnostic>> {
    let decisions_dir = slice_dir.join("decisions");
    if !decisions_dir.is_dir() {
        return Ok(Vec::new());
    }

    let files = list_md_files(&decisions_dir)?;

    let mut findings: Vec<Diagnostic> = Vec::new();
    let mut slug_files: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut records: Vec<(String, DecisionRecord)> = Vec::new();

    for path in &files {
        let text = std::fs::read_to_string(path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.clone(),
            source,
        })?;
        let hint = path_hint(path, slice_dir);
        let parsed = parse_decision(&text);
        for finding in parsed.findings {
            findings.push(finding.into_diagnostic(&hint));
        }
        if let Some(record) = parsed.record {
            slug_files.entry(record.slug.clone()).or_default().push(hint.clone());
            records.push((hint, record));
        }
    }

    for (slug, hints) in &slug_files {
        if hints.len() > 1 {
            findings.push(Diagnostic::violation(
                "decision-slug-collision",
                "Each Decision Record in the slice carries a distinct `slug`",
                format!("slug `{slug}` is shared by {} records: {}", hints.len(), hints.join(", ")),
                Artifact::Decisions,
                None,
            ));
        }
    }

    findings.extend(decision_supersede_orphans(layout, &records, &slug_files)?);
    Ok(findings)
}

/// `decision-supersede-orphan` — every `supersedes:` target must resolve
/// to a baseline `DEC-NNNN` (for a DEC reference) or to a baseline slug
/// or sibling slice record (for a slug reference).
fn decision_supersede_orphans(
    layout: Layout<'_>, records: &[(String, DecisionRecord)],
    slug_files: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<Diagnostic>> {
    let baseline = read_baseline(&layout.decisions_dir())?;
    let baseline_ids: BTreeSet<String> = baseline.iter().map(|b| b.id().to_string()).collect();
    let baseline_slugs: BTreeSet<String> = baseline.iter().map(|b| b.record.slug.clone()).collect();

    let mut findings: Vec<Diagnostic> = Vec::new();
    for (hint, record) in records {
        for target in &record.supersedes {
            let resolved = if is_dec_ref(target) {
                baseline_ids.contains(target)
            } else {
                baseline_slugs.contains(target) || slug_files.contains_key(target)
            };
            if !resolved {
                findings.push(Diagnostic::violation(
                    "decision-supersede-orphan",
                    "every `supersedes:` target resolves to a baseline DEC or a sibling record",
                    format!(
                        "decision `{}` (slug `{}`) supersedes `{target}`, which resolves to \
                         neither the baseline catalogue nor a sibling slice record",
                        record.slug, record.slug
                    ),
                    Artifact::Decisions,
                    Some(FindingLocation {
                        path: hint.clone(),
                        line: None,
                        column: None,
                        end_line: None,
                        end_column: None,
                    }),
                ));
            }
        }
    }
    Ok(findings)
}
