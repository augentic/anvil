//! Debt conservation through merge (RFC-86a D5): project the slice's
//! live deferred rows and stamp each staged debt row with one
//! self-describing `Note:` line before the fold.

use std::collections::BTreeMap;
use std::path::Path;

use artifacts::spec::provenance::RequirementStatus;
use artifacts::spec::{REQ_HEADING, REQ_ID_PREFIX};
use error::Error;
use jiff::Timestamp;
use project::config::Layout;
use project::journal::DeferralOrigin;
use project::plan::{Disposition, Plan, collect_events, plan_gaps_body};

use crate::debt::NOTE_PREFIX;

/// The slice's carried debt at merge time.
#[derive(Debug, Clone, Default)]
pub struct SliceDebt {
    /// Originating change (`plan.yaml.name`) stamped into each note.
    pub change: String,
    /// Deferred rows in inventory (declaration) order.
    pub rows: Vec<CarriedDebt>,
}

/// One deferred requirement folding into the baseline as debt.
#[derive(Debug, Clone)]
pub struct CarriedDebt {
    /// Requirement id — final baseline `REQ-NNN` when projected after
    /// identity finalization.
    pub req: String,
    /// Typed gap status (`unknown` | `conflict`).
    pub status: RequirementStatus,
    /// Canonical requirement-body digest — the deferral match key.
    pub requirement_digest: String,
    /// Covering fact's reason (operator or synthesized policy text).
    pub reason: String,
    /// Which surface dispositioned the requirement.
    pub origin: DeferralOrigin,
    /// When the covering fact was appended — the deferral date.
    pub deferred_at: Timestamp,
}

impl CarriedDebt {
    /// The self-describing `Note:` line this row folds into the
    /// baseline (RFC-86a D5).
    #[must_use]
    pub fn note_line(&self, change: &str) -> String {
        let date = self.deferred_at.strftime("%Y-%m-%d");
        format!(
            "{NOTE_PREFIX}origin: {}; change: {change}; date: {date}; reason: {}",
            self.origin, self.reason
        )
    }
}

/// Project the slice's deferred rows from the live model and the
/// deferral fact union — the debt set the merge fold conserves.
///
/// Empty when the project has no `plan.yaml` (standalone merges have
/// no disposition surface) or when nothing on the slice is deferred.
///
/// # Errors
///
/// Propagates plan / journal / model read failures from the gap
/// projection.
pub fn carried(layout: Layout<'_>, slice: &str) -> Result<SliceDebt, Error> {
    let plan_path = layout.plan_path();
    if !plan_path.is_file() {
        return Ok(SliceDebt::default());
    }
    let plan = Plan::load(&plan_path)?;
    let events = collect_events(layout)?;
    let rows = plan_gaps_body(&plan, layout, &events)?
        .rows
        .into_iter()
        .filter(|row| row.slice == slice && row.disposition == Some(Disposition::Deferred))
        .filter_map(|row| {
            // Deferred rows carry a digest and a covering fact by
            // construction — a fact can only match a digest-bearing row.
            let requirement_digest = row.requirement_digest?;
            let deferral = row.deferral?;
            Some(CarriedDebt {
                req: row.req,
                status: row.status,
                requirement_digest,
                reason: deferral.reason,
                origin: deferral.origin,
                deferred_at: deferral.deferred_at,
            })
        })
        .collect();
    Ok(SliceDebt {
        change: plan.name.to_string(),
        rows,
    })
}

/// Append each debt row's note line to its requirement block in the
/// staged `specs/<domain>/spec.md` deltas, keyed by the (finalized)
/// `ID:` line. Idempotent: an already-present identical note line is
/// not appended again, so merge re-entry never duplicates notes.
///
/// # Errors
///
/// Filesystem failures reading or rewriting a staged spec.
pub fn annotate(slice_dir: &Path, debt: &SliceDebt) -> Result<(), Error> {
    if debt.rows.is_empty() {
        return Ok(());
    }
    let notes: BTreeMap<&str, String> =
        debt.rows.iter().map(|row| (row.req.as_str(), row.note_line(&debt.change))).collect();
    let specs = slice_dir.join("specs");
    if !specs.is_dir() {
        return Ok(());
    }
    for entry in project::fs::dir_entries(&specs)? {
        let spec = entry.path().join("spec.md");
        if !spec.is_file() {
            continue;
        }
        let text = project::fs::read_text(&spec)?;
        let annotated = annotate_text(&text, &notes);
        if annotated != text {
            artifacts::atomic::bytes_write(&spec, annotated.as_bytes())?;
        }
    }
    Ok(())
}

/// Insert each note after the last non-blank line of its requirement
/// block. A block runs from its `### Requirement:` heading to the next
/// requirement heading or `## ` section heading.
fn annotate_text(text: &str, notes: &BTreeMap<&str, String>) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut block: Vec<&str> = Vec::new();
    let mut block_id: Option<String> = None;
    let mut in_block = false;

    let flush = |out: &mut Vec<String>, block: &mut Vec<&str>, block_id: &mut Option<String>| {
        if let Some(note) = block_id.take().and_then(|id| notes.get(id.as_str()))
            && !block.iter().any(|line| line.trim() == note)
        {
            let last = block.iter().rposition(|line| !line.trim().is_empty());
            let insert_at = last.map_or(block.len(), |idx| idx + 1);
            out.extend(block[..insert_at].iter().map(ToString::to_string));
            out.push(String::new());
            out.push(note.clone());
            out.extend(block[insert_at..].iter().map(ToString::to_string));
        } else {
            out.extend(block.iter().map(ToString::to_string));
        }
        block.clear();
    };

    for line in text.split('\n') {
        let stripped = line.trim();
        let boundary = stripped.starts_with(REQ_HEADING)
            || (stripped.starts_with("## ") && !stripped.starts_with(REQ_HEADING));
        if boundary && in_block {
            flush(&mut out, &mut block, &mut block_id);
            in_block = false;
        }
        if stripped.starts_with(REQ_HEADING) {
            in_block = true;
        }
        if in_block {
            if block_id.is_none()
                && let Some(rest) = stripped.strip_prefix(REQ_ID_PREFIX)
            {
                block_id = Some(rest.trim().to_string());
            }
            block.push(line);
        } else {
            out.push(line.to_string());
        }
    }
    if in_block {
        flush(&mut out, &mut block, &mut block_id);
    }
    out.join("\n")
}
