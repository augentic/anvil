//! Typed gap inventory projection.
//!
//! Pure read of in-scope slice artifacts into `(slice, req, status)` rows
//! for `unknown` / `conflict` / `divergence`; dropped slices are excluded.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use artifacts::spec::provenance::{RequirementStatus, parse_spec_md};
use error::Error;
use serde::{Deserialize, Serialize};

use super::in_scope;
use super::model::{Entry, Plan};
use crate::config::Layout;
use crate::handler::Render;
use crate::slice::SliceMetadata;

/// One open typed-status finding in the gap inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct GapRow {
    /// Plan entry / slice name.
    pub slice: String,
    /// Requirement id (`REQ-NNN`).
    pub req: String,
    /// Typed gap status (`unknown` / `conflict` / `divergence`).
    pub status: RequirementStatus,
    /// Requirement title / summary line.
    pub summary: String,
    /// Contributing `(source, lead)` shared across multiple open
    /// findings, rendered as `source:lead`. Absent when the row's
    /// contributors are not multi-homed in this inventory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_lead: Option<String>,
}

/// Presentation rollup for one multi-homed `(source, lead)` (D19).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SharedLeadRollup {
    /// Plan source key.
    pub source: String,
    /// Discovery lead id.
    pub lead: String,
    /// In-scope slice selectors suggested for a follow-up re-refine
    /// (via `emery plan execute`) after the shared input is fixed.
    pub selectors: Vec<String>,
}

/// Wire body for `emery plan gaps` (and the gaps section of
/// `plan status`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct GapsBody {
    /// Plan name from `plan.yaml.name`.
    pub plan: String,
    /// In-scope open findings, plan order then declaration order.
    pub rows: Vec<GapRow>,
    /// Shared-lead presentation groups (D19). Empty when no lead is
    /// multi-homed across open findings.
    pub rollups: Vec<SharedLeadRollup>,
}

impl GapsBody {
    /// Whether the inventory carries any open findings.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

impl Render for GapsBody {
    fn render(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        if self.rows.is_empty() {
            writeln!(w, "gaps: none")?;
            return Ok(());
        }
        writeln!(
            w,
            "{:<14} {:<10} {:<12} {:<32} shared-lead",
            "slice", "req", "status", "summary"
        )?;
        for row in &self.rows {
            let shared = row.shared_lead.as_deref().unwrap_or("—");
            writeln!(
                w,
                "{:<14} {:<10} {:<12} {:<32} {shared}",
                row.slice,
                row.req,
                row.status,
                truncate(&row.summary, 32)
            )?;
        }
        for rollup in &self.rollups {
            writeln!(
                w,
                "# shared lead {}:{} → re-refine selectors: {}",
                rollup.source,
                rollup.lead,
                rollup.selectors.join(" ")
            )?;
        }
        Ok(())
    }
}

/// Project the typed gap inventory for `plan` under `layout`.
///
/// # Errors
///
/// Propagates I/O and YAML failures when reading slice metadata or
/// `model.yaml`. Missing slice trees and absent model/spec files are
/// not errors — they contribute no rows.
pub fn plan_gaps_body(plan: &Plan, layout: Layout<'_>) -> Result<GapsBody, Error> {
    let mut raw: Vec<RawFinding> = Vec::new();
    for entry in &plan.entries {
        let slice_dir = layout.slice_dir(entry.name.as_str());
        let meta = SliceMetadata::load_optional(&slice_dir)?;
        if !in_scope(plan, entry, meta.as_ref()) {
            continue;
        }
        for finding in slice_findings(entry, &slice_dir)? {
            raw.push(finding);
        }
    }

    let lead_hits = count_leads(&raw);
    let rollups = build_rollups(&raw, &lead_hits);
    let rows = raw
        .into_iter()
        .map(|finding| {
            let shared_lead = finding
                .leads
                .iter()
                .find(|key| lead_hits.get(*key).is_some_and(|n| *n > 1))
                .map(|(source, lead)| format!("{source}:{lead}"));
            GapRow {
                slice: finding.slice,
                req: finding.req,
                status: finding.status,
                summary: finding.summary,
                shared_lead,
            }
        })
        .collect();

    Ok(GapsBody {
        plan: plan.name.to_string(),
        rows,
        rollups,
    })
}

/// One finding before shared-lead annotation.
struct RawFinding {
    slice: String,
    req: String,
    status: RequirementStatus,
    summary: String,
    /// Contributing `(source, lead)` pairs from plan bindings ∩
    /// requirement sources.
    leads: BTreeSet<(String, String)>,
}

/// Minimal `model.yaml` view — only the fields the inventory needs.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ModelView {
    #[serde(default)]
    requirements: Vec<ModelReq>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ModelReq {
    #[serde(default)]
    id: Option<String>,
    title: String,
    #[serde(default)]
    status: Option<RequirementStatus>,
    #[serde(default)]
    sources: Vec<String>,
}

fn slice_findings(entry: &Entry, slice_dir: &Path) -> Result<Vec<RawFinding>, Error> {
    let model_path = slice_dir.join("model.yaml");
    if model_path.is_file() {
        let text = std::fs::read_to_string(&model_path)?;
        let model: ModelView = serde_saphyr::from_str(&text)?;
        if !model.requirements.is_empty() {
            return Ok(model
                .requirements
                .into_iter()
                .filter_map(|req| gap_from_model(entry, req))
                .collect());
        }
    }
    Ok(findings_from_specs(entry, &slice_dir.join("specs")))
}

fn gap_from_model(entry: &Entry, req: ModelReq) -> Option<RawFinding> {
    let status = req.status.filter(|&s| is_gap(s))?;
    let id = req.id.filter(|s| !s.is_empty())?;
    Some(RawFinding {
        slice: entry.name.to_string(),
        req: id,
        status,
        summary: req.title,
        leads: contributing_leads(entry, &req.sources),
    })
}

fn findings_from_specs(entry: &Entry, specs_dir: &Path) -> Vec<RawFinding> {
    let Ok(entries) = std::fs::read_dir(specs_dir) else {
        return Vec::new();
    };
    let mut domains: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().join("spec.md").is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    domains.sort();
    let mut out = Vec::new();
    for domain in domains {
        let path = specs_dir.join(&domain).join("spec.md");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for req in parse_spec_md(&text).requirements {
            let Some(status) = req.status.filter(|&s| is_gap(s)) else {
                continue;
            };
            if req.id.is_empty() {
                continue;
            }
            out.push(RawFinding {
                slice: entry.name.to_string(),
                req: req.id,
                status,
                summary: req.name,
                leads: contributing_leads(entry, &req.sources),
            });
        }
    }
    out
}

const fn is_gap(status: RequirementStatus) -> bool {
    matches!(
        status,
        RequirementStatus::Unknown | RequirementStatus::Conflict | RequirementStatus::Divergence
    )
}

fn contributing_leads(entry: &Entry, sources: &[String]) -> BTreeSet<(String, String)> {
    let mut leads = BTreeSet::new();
    for source in sources {
        let lead = entry
            .sources
            .iter()
            .find(|b| b.source == *source)
            .map_or_else(|| entry.name.to_string(), |b| b.lead(entry.name.as_str()).to_string());
        leads.insert((source.clone(), lead));
    }
    leads
}

fn count_leads(findings: &[RawFinding]) -> BTreeMap<(String, String), usize> {
    let mut hits = BTreeMap::new();
    for finding in findings {
        for key in &finding.leads {
            *hits.entry(key.clone()).or_insert(0) += 1;
        }
    }
    hits
}

fn build_rollups(
    findings: &[RawFinding], hits: &BTreeMap<(String, String), usize>,
) -> Vec<SharedLeadRollup> {
    let mut by_lead: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for finding in findings {
        for key in &finding.leads {
            if hits.get(key).is_some_and(|n| *n > 1) {
                by_lead.entry(key.clone()).or_default().insert(finding.slice.clone());
            }
        }
    }
    by_lead
        .into_iter()
        .map(|((source, lead), selectors)| SharedLeadRollup {
            source,
            lead,
            selectors: selectors.into_iter().collect(),
        })
        .collect()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
