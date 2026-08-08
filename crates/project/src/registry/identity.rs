//! Deterministic baseline identity projection.
//!
//! Purely structural — never an LLM summary — so the committed
//! `.emery/topology.lock` is verifiable by regenerate-and-compare.

use std::path::Path;

use artifacts::decision::DecisionStatus;
use artifacts::spec::provenance::parse_spec_md;
use error::Error;

use super::topology::{Decision, Surface};
use crate::config::Layout;
use crate::journal::{self, EventKind};

/// Maximum requirement titles projected per domain (`K`). A domain with
/// more emits a `more:` count of the elided tail rather than the
/// titles themselves.
pub const SURFACE_TITLE_CAP: usize = 8;

/// Maximum `slice.archive.created` outcome summaries projected into
/// `recent[]` (`M`). The tail suffices — older merges are already
/// reflected in `surface[]`.
pub const RECENT_TAIL: usize = 10;

/// Maximum accepted Decision Records projected into `decisions[]`
/// (`K`). A catalogue with more emits a `decisions-more` count of the
/// elided remainder.
pub const DECISIONS_CAP: usize = 8;

/// The deterministic identity projection of a project's baseline: the
/// `surface[]` / `recent[]` pair plus the accepted-decision
/// `decisions[]` axis with its overflow count.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Projection {
    /// Owned domains + bounded requirement titles.
    pub surface: Vec<Surface>,
    /// Recent per-merge outcome summaries.
    pub recent: Vec<String>,
    /// Accepted decisions, most-recent `K` in `DEC` ascending order.
    pub decisions: Vec<Decision>,
    /// Count of accepted decisions elided past the cap, if any.
    pub decisions_more: Option<u64>,
}

/// Project `project_dir`'s baseline into the `(surface, recent)` pair.
///
/// `surface` enumerates every `.emery/specs/<domain>/spec.md` sorted
/// by slug; `recent` is the last [`RECENT_TAIL`] `slice.archive.created`
/// outcome summaries in union order. A project with no baseline yields
/// two empty vectors — greenfield reconciliation degrades cleanly.
///
/// # Errors
///
/// Surfaces I/O errors reading the specs tree or a `spec.md`, and any
/// error from reading the journal.
pub fn project_baseline(project_dir: &Path) -> Result<Projection, Error> {
    let surface = project_surface(project_dir)?;
    let recent = project_recent(project_dir)?;
    let (decisions, decisions_more) = project_decisions(project_dir)?;
    Ok(Projection {
        surface,
        recent,
        decisions,
        decisions_more,
    })
}

/// Project `.emery/decisions/` into the bounded `decisions[]` axis.
/// Only `status: accepted` records contribute; superseded and rejected
/// records describe past or
/// not-taken posture and are excluded from *current* identity. The most
/// recent [`DECISIONS_CAP`] (highest `DEC` ids) are kept, then emitted in
/// `DEC` ascending order; the overflow count is returned alongside.
fn project_decisions(project_dir: &Path) -> Result<(Vec<Decision>, Option<u64>), Error> {
    let decisions_dir = Layout::new(project_dir).decisions_dir();
    let baseline = crate::decisions::read_baseline(&decisions_dir)?;
    // `read_baseline` already sorts by `DEC-NNNN` ascending.
    let mut accepted: Vec<Decision> = baseline
        .into_iter()
        .filter(|b| b.record.status == DecisionStatus::Accepted)
        .map(|b| Decision {
            id: b.id().to_string(),
            title: b.title.unwrap_or_default(),
            topics: b.record.topics.clone(),
        })
        .collect();

    let total = accepted.len();
    let more = (total > DECISIONS_CAP).then(|| {
        // Keep the most recent K (highest ids) while preserving the
        // ascending order already in hand: drop the oldest overflow.
        accepted.drain(..total - DECISIONS_CAP);
        u64::try_from(total - DECISIONS_CAP).unwrap_or(u64::MAX)
    });
    Ok((accepted, more))
}

fn project_surface(project_dir: &Path) -> Result<Vec<Surface>, Error> {
    let specs_dir = Layout::new(project_dir).emery_dir().join("specs");
    if !specs_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut domains: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&specs_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let Some(domain) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        // Only project domains that actually carry a baseline spec; an
        // empty domain directory is in-progress noise, not owned surface.
        if !entry.path().join("spec.md").is_file() {
            continue;
        }
        domains.push(domain);
    }
    domains.sort();

    let mut surfaces: Vec<Surface> = Vec::with_capacity(domains.len());
    for domain in domains {
        let text = std::fs::read_to_string(specs_dir.join(&domain).join("spec.md"))?;
        surfaces.push(project_domain(domain, &text));
    }
    Ok(surfaces)
}

/// Project one domain's `spec.md` into its bounded [`Surface`].
fn project_domain(domain: String, spec: &str) -> Surface {
    let parsed = parse_spec_md(spec);
    let mut ordered: Vec<(u64, String)> =
        parsed.requirements.into_iter().map(|req| (requirement_order(&req.id), req.name)).collect();
    // Stable sort by `REQ-NNN` id; requirements without an `ID:` line
    // sort to the tail while keeping document order among themselves.
    ordered.sort_by_key(|(order, _)| *order);
    let mut requirements: Vec<String> = ordered.into_iter().map(|(_, name)| name).collect();

    let total = requirements.len();
    let more = (total > SURFACE_TITLE_CAP).then(|| {
        requirements.truncate(SURFACE_TITLE_CAP);
        u64::try_from(total - SURFACE_TITLE_CAP).unwrap_or(u64::MAX)
    });
    Surface {
        domain,
        requirements,
        more,
    }
}

/// Sort key for a `REQ-NNN` id: the trailing integer, or [`u64::MAX`]
/// when the id is absent or unparseable so unlabelled requirements
/// stable-sort to the tail.
fn requirement_order(id: &str) -> u64 {
    id.rsplit('-').next().and_then(|n| n.parse().ok()).unwrap_or(u64::MAX)
}

fn project_recent(project_dir: &Path) -> Result<Vec<String>, Error> {
    // Tail-read the last `RECENT_TAIL` archive summaries rather than
    // loading every event and discarding all but the tail — cost stays
    // flat as the journal grows.
    journal::read_recent(Layout::new(project_dir), RECENT_TAIL, |event| match event.kind {
        EventKind::SliceArchiveCreated { outcome_summary, .. } => Some(outcome_summary),
        _ => None,
    })
}
