//! Typed gap inventory projection.
//!
//! In-scope `unknown`/`conflict`/`divergence` rows with `open | deferred`
//! dispositions joined from the deferral fact union (RFC-86a D2).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use artifacts::spec::provenance::{RequirementStatus, parse_spec_md};
use error::Error;
use serde::{Deserialize, Serialize};

use super::in_scope;
use super::model::{Entry, Plan};
use crate::config::Layout;
use crate::handler::Render;
use crate::journal::{DeferralOrigin, Event, EventKind};
use crate::slice::{RequirementBody, SliceMetadata};

/// Computed gap disposition (RFC-86a D2): joined from the deferral
/// fact union against the live model at projection time, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Disposition {
    /// No live deferral covers the requirement.
    Open,
    /// A live deferral fact covers the requirement's `(slice, digest)`.
    Deferred,
}

/// Covering deferral detail on a deferred row (RFC-86a D7): the
/// reason, origin, and timestamp of the latest live `gap.deferred`
/// fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Deferral {
    /// Operator reason, or the synthesized policy reason.
    pub reason: String,
    /// Which surface dispositioned the requirement.
    pub origin: DeferralOrigin,
    /// When the covering fact was appended — the deferral date the
    /// merge fold stamps into the baseline debt note (RFC-86a D5).
    #[serde(with = "crate::serde_time::rfc3339")]
    pub deferred_at: jiff::Timestamp,
}

/// Deferred-gap debt counts with conflicts broken out (RFC-86a
/// D6/D7): a shipped-around contradiction is always louder news than
/// a shipped-around absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DebtCounts {
    /// Deferred `[unknown]` rows.
    pub unknown: usize,
    /// Deferred `[conflict]` rows.
    pub conflict: usize,
}

impl DebtCounts {
    /// Total deferred rows.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.unknown + self.conflict
    }

    /// Whether the plan carries no deferred debt.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

/// `3 deferred gaps (2 unknown, 1 conflict)` — the debt line beside
/// the `plan status` milestones (RFC-86a D7).
impl std::fmt::Display for DebtCounts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let noun = if self.total() == 1 { "gap" } else { "gaps" };
        write!(
            f,
            "{} deferred {noun} ({} unknown, {} conflict)",
            self.total(),
            self.unknown,
            self.conflict
        )
    }
}

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
    /// Canonical requirement-body digest (`sha256:<hex>`) — the
    /// deferral match key (RFC-86a D2). Present on `model.yaml`-backed
    /// rows; the legacy `spec.md` fallback carries no body fields to
    /// digest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement_digest: Option<String>,
    /// Computed disposition. Present on `unknown` / `conflict` rows;
    /// `[divergence]` rows take no disposition (RFC-86a D2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disposition: Option<Disposition>,
    /// Covering deferral's reason and origin — present exactly when
    /// [`Self::disposition`] is deferred (RFC-86a D7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deferral: Option<Deferral>,
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
    /// Whether the inventory carries any findings at all.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Whether any row's computed disposition is `open` — the set
    /// next-actions and the strict gate compute over (RFC-86a D7).
    #[must_use]
    pub fn has_open(&self) -> bool {
        self.rows.iter().any(|row| row.disposition == Some(Disposition::Open))
    }

    /// Deferred-debt counts with conflicts broken out (RFC-86a D6/D7).
    #[must_use]
    pub fn debt(&self) -> DebtCounts {
        let mut counts = DebtCounts {
            unknown: 0,
            conflict: 0,
        };
        for row in &self.rows {
            if row.disposition != Some(Disposition::Deferred) {
                continue;
            }
            match row.status {
                RequirementStatus::Unknown => counts.unknown += 1,
                RequirementStatus::Conflict => counts.conflict += 1,
                RequirementStatus::Divergence | RequirementStatus::Agreed => {}
            }
        }
        counts
    }

    /// Render the deferred rows of one gap kind under `heading`, each
    /// with the covering fact's reason and origin. Deferred conflicts
    /// and deferred unknowns get separate blocks (RFC-86a D6/D7).
    fn render_deferred(
        &self, w: &mut dyn std::io::Write, status: RequirementStatus, heading: &str,
    ) -> std::io::Result<()> {
        let rows = self
            .rows
            .iter()
            .filter(|row| row.disposition == Some(Disposition::Deferred) && row.status == status);
        let mut headed = false;
        for row in rows {
            if !headed {
                writeln!(w, "{heading}")?;
                headed = true;
            }
            // Deferred rows carry the covering fact by construction.
            let detail = row
                .deferral
                .as_ref()
                .map_or_else(String::new, |d| format!(" — {} ({})", d.reason, d.origin));
            writeln!(w, "  {}/{} {}{detail}", row.slice, row.req, truncate(&row.summary, 48))?;
        }
        Ok(())
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
            "{:<14} {:<10} {:<12} {:<12} {:<32} shared-lead",
            "slice", "req", "status", "disposition", "summary"
        )?;
        for row in &self.rows {
            let shared = row.shared_lead.as_deref().unwrap_or("—");
            let disposition = row.disposition.map_or_else(|| "—".to_string(), |d| d.to_string());
            writeln!(
                w,
                "{:<14} {:<10} {:<12} {:<12} {:<32} {shared}",
                row.slice,
                row.req,
                row.status,
                disposition,
                truncate(&row.summary, 32)
            )?;
        }
        self.render_deferred(w, RequirementStatus::Unknown, "deferred unknowns:")?;
        self.render_deferred(w, RequirementStatus::Conflict, "deferred conflicts:")?;
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
/// Each row's `open | deferred` disposition joins the deferral facts
/// in `events` (the journal union). Liveness is recomputed, never
/// stored: the latest defer/retract fact per `(slice, digest)` wins by
/// `(timestamp, writer, sequence)`; duplicates are idempotent; a
/// digest absent from the live model is simply not live (lapse), and
/// its reappearance revives it (RFC-86a D2).
///
/// # Errors
///
/// Propagates I/O and YAML failures when reading slice metadata or
/// `model.yaml`. Missing slice trees and absent model/spec files are
/// not errors — they contribute no rows.
pub fn plan_gaps_body(
    plan: &Plan, layout: Layout<'_>, events: &[Event],
) -> Result<GapsBody, Error> {
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

    let deferred = live_deferrals(events);
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
            let (disposition, deferral) = disposition(&finding, &deferred);
            GapRow {
                slice: finding.slice,
                req: finding.req,
                status: finding.status,
                summary: finding.summary,
                requirement_digest: finding.digest,
                disposition,
                deferral,
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

/// Disposition and covering deferral for one finding: `[divergence]`
/// takes none; a live deferral on the row's `(slice, digest)` defers
/// it with the fact's reason and origin; everything else is open
/// (including digest-less `spec.md`-fallback rows, which no fact can
/// match).
fn disposition(
    finding: &RawFinding, deferred: &BTreeMap<(String, String), Deferral>,
) -> (Option<Disposition>, Option<Deferral>) {
    if finding.status == RequirementStatus::Divergence {
        return (None, None);
    }
    finding
        .digest
        .as_ref()
        .and_then(|digest| deferred.get(&(finding.slice.clone(), digest.clone())))
        .map_or((Some(Disposition::Open), None), |deferral| {
            (Some(Disposition::Deferred), Some(deferral.clone()))
        })
}

/// Envelope ordering key of one fact: `(timestamp, writer, sequence)`.
type FactOrder = (jiff::Timestamp, String, u64);

/// Live deferral detail per `(slice, digest)`: present when the
/// latest defer/retract fact is a deferral. Latest wins by
/// `(timestamp, writer, sequence)` regardless of the slice of `events`
/// being pre-sorted; duplicate facts fold idempotently.
fn live_deferrals(events: &[Event]) -> BTreeMap<(String, String), Deferral> {
    let mut latest: BTreeMap<(String, String), (FactOrder, Option<Deferral>)> = BTreeMap::new();
    for event in events {
        let (slice, digest, deferral) = match &event.kind {
            EventKind::GapDeferred {
                slice,
                requirement_digest,
                reason,
                origin,
                ..
            } => (
                slice,
                requirement_digest,
                Some(Deferral {
                    reason: reason.clone(),
                    origin: *origin,
                    deferred_at: event.timestamp,
                }),
            ),
            EventKind::GapDeferralRetracted {
                slice,
                requirement_digest,
                ..
            } => (slice, requirement_digest, None),
            _ => continue,
        };
        let order = (event.timestamp, event.writer.clone(), event.sequence);
        let key = (slice.as_str().to_string(), digest.clone());
        match latest.get(&key) {
            Some((existing, _)) if *existing > order => {}
            _ => {
                latest.insert(key, (order, deferral));
            }
        }
    }
    latest.into_iter().filter_map(|(key, (_, deferral))| deferral.map(|live| (key, live))).collect()
}

/// One finding before shared-lead annotation.
struct RawFinding {
    slice: String,
    req: String,
    status: RequirementStatus,
    summary: String,
    /// Canonical body digest — `None` on `spec.md`-fallback rows,
    /// which carry no body fields to digest.
    digest: Option<String>,
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
    /// Required, matching the strict typed model in `crates/slice` — a
    /// malformed `model.yaml` must not mint a deferral match key over
    /// an empty statement (RFC-86a D2).
    statement: String,
    #[serde(default)]
    scenarios: Vec<String>,
    #[serde(default)]
    notes: Option<String>,
}

impl ModelReq {
    /// Canonical body digest — the same [`RequirementBody`] encoding
    /// the typed `slice` model view computes, so both layers mint one
    /// deferral match key (RFC-86a D2).
    fn body_digest(&self) -> String {
        RequirementBody {
            title: &self.title,
            statement: &self.statement,
            scenarios: &self.scenarios,
            notes: self.notes.as_deref(),
        }
        .digest()
    }
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
    let digest = req.body_digest();
    let id = req.id.filter(|s| !s.is_empty())?;
    Some(RawFinding {
        slice: entry.name.to_string(),
        req: id,
        status,
        summary: req.title,
        digest: Some(digest),
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
                digest: None,
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
