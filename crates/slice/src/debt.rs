//! The baseline debt projection (RFC-86a D9).
//!
//! Reads the baseline specs alone — never archived fact logs — listing
//! every carried `unknown` / `conflict` row with its D5 note parsed.

use std::path::Path;

use artifacts::spec::provenance::{Requirement, RequirementStatus, parse_spec_md};
use error::Error;
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use project::journal::DeferralOrigin;
use serde::Serialize;

/// Line prefix of the self-describing baseline debt note. The tail is
/// `origin: <o>; change: <c>; date: <YYYY-MM-DD>; reason: <free text>`
/// — reason last, so free text never confuses the fixed-key parse.
pub const NOTE_PREFIX: &str = "Note: deferred — ";

/// One carried gap-status requirement in the baseline backlog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DebtRow {
    /// Domain directory under `.emery/specs/` that carries the row.
    pub domain: String,
    /// Baseline requirement id (`REQ-NNN`).
    pub req: String,
    /// Typed gap status (`unknown` | `conflict`).
    pub status: RequirementStatus,
    /// Requirement heading name.
    pub summary: String,
    /// Parsed deferral note. `None` when the row carries no
    /// well-formed note (a gap merged outside the deferral surface,
    /// or a hand-mangled note) — the row is still debt, just without
    /// its provenance detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deferral: Option<DebtNote>,
}

/// The self-describing deferral note fields (RFC-86a D5), plus the
/// age computed against the projection clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DebtNote {
    /// Covering deferral's reason (operator or synthesized policy text).
    pub reason: String,
    /// Which surface dispositioned the requirement.
    pub origin: DeferralOrigin,
    /// Originating change (`plan.yaml.name` at merge time).
    pub change: String,
    /// Deferral date as stamped (`YYYY-MM-DD`).
    pub deferred_on: String,
    /// Whole days between the deferral date and the projection clock.
    pub age_days: u64,
}

/// Project the baseline debt inventory under `specs_dir`, in domain
/// order then document order. A missing specs tree is an empty
/// backlog, not an error.
///
/// # Errors
///
/// Propagates filesystem failures reading the specs tree.
pub fn baseline(specs_dir: &Path, now: Timestamp) -> Result<Vec<DebtRow>, Error> {
    let mut rows = Vec::new();
    if !specs_dir.is_dir() {
        return Ok(rows);
    }
    let today = now.to_zoned(TimeZone::UTC).date();
    let mut domains: Vec<String> = project::fs::dir_entries(specs_dir)?
        .iter()
        .filter(|entry| entry.path().join("spec.md").is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    domains.sort();
    for domain in domains {
        let text = project::fs::read_text(&specs_dir.join(&domain).join("spec.md"))?;
        for req in parse_spec_md(&text).requirements {
            let carried = matches!(
                req.status,
                Some(RequirementStatus::Unknown | RequirementStatus::Conflict)
            );
            if !carried || req.id.is_empty() {
                continue;
            }
            rows.push(DebtRow {
                domain: domain.clone(),
                req: req.id.clone(),
                status: req.status.unwrap_or(RequirementStatus::Unknown),
                summary: req.name.clone(),
                deferral: parse_note(&req, today),
            });
        }
    }
    Ok(rows)
}

/// Parse the newest well-formed deferral note in the requirement's
/// body. Lenient: a mangled note degrades to `None` — the projection
/// never fails on operator-edited prose.
fn parse_note(req: &Requirement, today: Date) -> Option<DebtNote> {
    req.body.lines().rev().find_map(|line| {
        let rest = line.trim().strip_prefix(NOTE_PREFIX)?.strip_prefix("origin: ")?;
        let (origin, rest) = rest.split_once("; change: ")?;
        let (change, rest) = rest.split_once("; date: ")?;
        let (date, reason) = rest.split_once("; reason: ")?;
        let origin: DeferralOrigin = origin.parse().ok()?;
        let deferred: Date = date.parse().ok()?;
        // Clamp future-dated notes (clock skew) to zero age rather
        // than dropping the provenance detail.
        let age_days = deferred
            .until(today)
            .ok()
            .and_then(|span| u64::try_from(span.get_days().max(0)).ok())?;
        Some(DebtNote {
            reason: reason.to_string(),
            origin,
            change: change.to_string(),
            deferred_on: date.to_string(),
            age_days,
        })
    })
}

/// The `## Carried debt` review-prose section (RFC-86a D9).
///
/// `plan author` renders it into `change.md` — the same inventory
/// `emery debt` projects, so a corrective change is scoped with the
/// backlog in view. `None` when the baseline carries no debt.
#[must_use]
pub fn markdown(rows: &[DebtRow]) -> Option<String> {
    use std::fmt::Write as _;

    if rows.is_empty() {
        return None;
    }
    let mut out = String::from(
        "## Carried debt\n\n\
         Baseline requirements deferred by earlier changes (see `emery debt`). \
         New evidence in this change's sources resolves a carried row at refine; \
         an untouched row carries forward.\n",
    );
    for (status, heading) in
        [(RequirementStatus::Unknown, "Unknowns"), (RequirementStatus::Conflict, "Conflicts")]
    {
        let mut headed = false;
        for row in rows.iter().filter(|row| row.status == status) {
            if !headed {
                let _ = write!(out, "\n{heading}:\n\n");
                headed = true;
            }
            let _ = writeln!(out, "- {}", row.render_line());
        }
    }
    Some(out)
}

impl DebtRow {
    /// One-line rendering shared by the CLI text projection and the
    /// review-prose section: `<domain>/<req> <summary> — <note detail>`.
    #[must_use]
    pub fn render_line(&self) -> String {
        let head = format!("{}/{} {}", self.domain, self.req, self.summary);
        match &self.deferral {
            Some(note) => {
                let noun = if note.age_days == 1 { "day" } else { "days" };
                format!(
                    "{head} — {} ({}, change {}, {} {noun})",
                    note.reason, note.origin, note.change, note.age_days
                )
            }
            None => head,
        }
    }
}
