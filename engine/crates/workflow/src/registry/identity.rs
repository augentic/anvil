//! Deterministic baseline identity projection.
//!
//! Projects a materialised project/slot directory into the
//! `surface[]` / `recent[]` pair carried by `.specify/topology.lock`
//! and the reconciliation envelope. The projection is purely
//! structural — domain slugs, requirement-block headings, and the
//! journal's `slice.archive.created` outcome summaries — never an LLM
//! summary, so the committed lock is verifiable by
//! regenerate-and-compare.

use std::path::Path;

use specify_error::Error;
use specify_model::decision::DecisionStatus;
use specify_model::spec::provenance::parse_spec_md;

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
pub struct IdentityProjection {
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
/// `surface` enumerates every `.specify/specs/<domain>/spec.md`, sorted
/// by slug, each carrying up to [`SURFACE_TITLE_CAP`] requirement
/// titles in `REQ-NNN` id order plus a `more` count when capped.
/// `recent` is the last [`RECENT_TAIL`] `slice.archive.created`
/// outcome summaries from `.specify/journal.jsonl`, in append order.
/// A project with no baseline yields two empty vectors — greenfield
/// reconciliation degrades cleanly to `description` only.
///
/// # Errors
///
/// Surfaces I/O errors reading the specs tree or a `spec.md`, and any
/// error from reading the journal.
pub fn project_baseline(project_dir: &Path) -> Result<IdentityProjection, Error> {
    let surface = project_surface(project_dir)?;
    let recent = project_recent(project_dir)?;
    let (decisions, decisions_more) = project_decisions(project_dir)?;
    Ok(IdentityProjection {
        surface,
        recent,
        decisions,
        decisions_more,
    })
}

/// Project `.specify/decisions/` into the bounded `decisions[]` axis.
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
    let specs_dir = Layout::new(project_dir).specify_dir().join("specs");
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn write_spec(project_dir: &Path, domain: &str, body: &str) {
        let dir = project_dir.join(".specify").join("specs").join(domain);
        fs::create_dir_all(&dir).expect("mkdir domain");
        fs::write(dir.join("spec.md"), body).expect("write spec.md");
    }

    fn requirement(id: &str, name: &str) -> String {
        format!("### Requirement: {name}\n\nID: {id}\n\nSome body.\n\n")
    }

    #[test]
    fn empty_baseline_projects_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let projection = project_baseline(dir.path()).expect("project");
        assert!(projection.surface.is_empty());
        assert!(projection.recent.is_empty());
        assert!(projection.decisions.is_empty());
        assert!(projection.decisions_more.is_none());
    }

    #[test]
    fn domains_by_slug_reqs_by_req_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_spec(
            dir.path(),
            "session",
            &format!(
                "{}{}",
                requirement("REQ-002", "Revoke session"),
                requirement("REQ-001", "Issue token")
            ),
        );
        write_spec(dir.path(), "password-reset", &requirement("REQ-001", "Request reset"));

        let surface = project_baseline(dir.path()).expect("project").surface;
        assert_eq!(surface.len(), 2);
        assert_eq!(surface[0].domain, "password-reset");
        assert_eq!(surface[1].domain, "session");
        // `session` requirements are emitted in REQ id order, not the
        // (reversed) document order they were authored in.
        assert_eq!(surface[1].requirements, vec!["Issue token", "Revoke session"]);
        assert!(surface[1].more.is_none());
    }

    #[test]
    fn requirements_capped_with_more_count() {
        let dir = tempfile::tempdir().expect("tempdir");
        let body: String =
            (1..=12).map(|n| requirement(&format!("REQ-{n:03}"), &format!("Req {n}"))).collect();
        write_spec(dir.path(), "billing", &body);

        let surface = project_baseline(dir.path()).expect("project").surface;
        assert_eq!(surface[0].requirements.len(), SURFACE_TITLE_CAP);
        assert_eq!(surface[0].more, Some(4));
        assert_eq!(surface[0].requirements[0], "Req 1");
    }

    #[test]
    fn recent_keeps_last_m_archive_summaries() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = Layout::new(dir.path());
        let ts = journal::test_timestamp("2026-01-01T00:00:00Z");
        let mut events: Vec<journal::Event> = (1..=13)
            .map(|n| {
                journal::Event::new(
                    ts,
                    EventKind::SliceArchiveCreated {
                        slice_name: format!("s{n}").into(),
                        touched_specs: vec![format!("u{n}")],
                        outcome_summary: format!("u{n}: 1 modified"),
                        merge_sha: None,
                        decisions: Vec::new(),
                    },
                )
            })
            .collect();
        // An unrelated event kind must be ignored by the projection.
        events.push(journal::Event::new(
            ts,
            EventKind::PlanTransitionApproved {
                plan_name: "p".into(),
                actor: journal::Actor::Operator,
            },
        ));
        journal::append_batch(layout, &events).expect("append journal");

        let recent = project_baseline(dir.path()).expect("project").recent;
        assert_eq!(recent.len(), RECENT_TAIL);
        assert_eq!(recent.first().map(String::as_str), Some("u4: 1 modified"));
        assert_eq!(recent.last().map(String::as_str), Some("u13: 1 modified"));
    }

    /// Write a promoted baseline Decision Record at
    /// `.specify/decisions/DEC-NNNN-<slug>.md`.
    fn write_decision(project_dir: &Path, id: &str, slug: &str, status: &str, title: &str) {
        let dir = project_dir.join(".specify").join("decisions");
        fs::create_dir_all(&dir).expect("mkdir decisions");
        let body = format!(
            "---\nid: {id}\nslug: {slug}\nstatus: {status}\nslice: s\ndate: 2026-06-02\n---\n\
             # {title}\n\n## Context\nc\n\n## Decision\nd\n\n## Consequences\ne\n"
        );
        fs::write(dir.join(format!("{id}-{slug}.md")), body).expect("write decision");
    }

    #[test]
    fn decisions_accepted_only_ascending() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_decision(dir.path(), "DEC-0001", "use-postgres", "accepted", "Use PostgreSQL");
        write_decision(dir.path(), "DEC-0002", "drop-redis", "rejected", "Drop Redis");
        write_decision(dir.path(), "DEC-0003", "event-sourcing", "superseded", "Event sourcing");
        write_decision(dir.path(), "DEC-0004", "use-grpc", "accepted", "Use gRPC");

        let projection = project_baseline(dir.path()).expect("project");
        // Only accepted records, in DEC ascending order, title only.
        let ids: Vec<&str> = projection.decisions.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, vec!["DEC-0001", "DEC-0004"]);
        assert_eq!(projection.decisions[0].title, "Use PostgreSQL");
        assert_eq!(projection.decisions[1].title, "Use gRPC");
        assert!(projection.decisions_more.is_none());
    }

    #[test]
    fn decisions_capped_keeps_most_recent() {
        let dir = tempfile::tempdir().expect("tempdir");
        for n in 1..=11 {
            write_decision(
                dir.path(),
                &format!("DEC-{n:04}"),
                &format!("slug-{n}"),
                "accepted",
                &format!("Decision {n}"),
            );
        }

        let projection = project_baseline(dir.path()).expect("project");
        assert_eq!(projection.decisions.len(), DECISIONS_CAP);
        // The most recent K (highest ids) survive, in ascending order.
        assert_eq!(projection.decisions.first().map(|d| d.id.as_str()), Some("DEC-0004"));
        assert_eq!(projection.decisions.last().map(|d| d.id.as_str()), Some("DEC-0011"));
        assert_eq!(projection.decisions_more, Some(3));
    }
}
