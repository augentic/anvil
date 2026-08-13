//! `emery system review` (RFC-104 D10): record architectural
//! authority over one exact wave handoff. Selection is digest-exact
//! against the live definition, never newest-by-time.

use error::Error;
use jiff::Timestamp;
use project::journal::{self, Event, EventKind, JournalRoot};

use crate::coverage::Coverage;
use crate::decision;
use crate::handoff::{self, Projected};
use crate::layout::Layout;
use crate::migration::Migration;
use crate::model::Model;
use crate::scope::Scope;

/// The completed review's accounting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewOutcome {
    /// The reviewed wave's id.
    pub wave: String,
    /// The reviewed handoff's content digest (`sha256:…`).
    pub handoff_digest: String,
    /// False when the same handoff was already reviewed — re-entry is
    /// a read-only no-op that appends nothing.
    pub recorded: bool,
}

/// Select the wave's current handoff: the unique projection whose
/// covered digests all match the live definition files.
///
/// # Errors
///
/// - `system-review-handoff-stale` when no current handoff exists for
///   the wave (re-run `emery system plan`).
/// - `system-review-ambiguous` when more than one matches (fail
///   closed, never resolved by time).
/// - Fail-closed loads of the live definition files.
pub fn current_handoff(layout: &Layout<'_>, wave: &str) -> Result<Projected, Error> {
    let scope = Scope::load(&layout.scope_path())?;
    let coverage = Coverage::load(&layout.coverage_path())?;
    let model = Model::load(&layout.system_path())?;
    let migration = Migration::load(&layout.migration_path())?;
    let decisions = decision::load_all(&layout.decisions_dir())?;
    let scope_digest = scope.digest()?;
    let coverage_digest = coverage.digest()?;
    let model_digest = model.digest()?;
    let migration_digest = migration.digest()?;
    let decisions_digest = handoff::decisions_digest(&decisions)?;
    let mut current: Vec<Projected> = handoff::load_all(layout)?
        .into_iter()
        .filter(|projected| {
            let handoff = &projected.handoff;
            handoff.wave.id == wave
                && handoff.scope_digest == scope_digest
                && handoff.coverage_digest == coverage_digest
                && handoff.system_model_digest == model_digest
                && handoff.migration_plan_digest == migration_digest
                && handoff.decisions_digest == decisions_digest
        })
        .collect();
    match current.len() {
        0 => Err(Error::Diag {
            code: "system-review-handoff-stale",
            detail: format!(
                "no handoff for wave `{wave}` matches the live definition files — run `emery \
                 system plan` to project the current handoff, review it, then re-run the review"
            ),
        }),
        1 => Ok(current.remove(0)),
        _ => Err(Error::Diag {
            code: "system-review-ambiguous",
            detail: format!(
                "more than one handoff for wave `{wave}` matches the live definition files; \
                 review cannot choose between them"
            ),
        }),
    }
}

/// Record architectural authority over one exact handoff.
///
/// Compare-and-set: `supplied` (the bare 64-hex digest or the full
/// `sha256:…` form the operator reviewed) must name the wave's
/// current handoff. Reviewing the same handoff twice is a read-only
/// no-op.
///
/// # Errors
///
/// - Current-handoff selection failures per [`current_handoff`].
/// - `system-review-stale` when `supplied` names a different handoff
///   than the current one — the definition moved under the reviewer.
/// - Journal append failures.
pub fn review(
    layout: &Layout<'_>, wave: &str, supplied: &str, now: Timestamp,
) -> Result<ReviewOutcome, Error> {
    let current = current_handoff(layout, wave)?;
    let digest = current.digest.as_str();
    let reviewed = supplied.strip_prefix("sha256:").unwrap_or(supplied);
    if reviewed != current.digest.digest() {
        return Err(Error::Diag {
            code: "system-review-stale",
            detail: format!(
                "the reviewed handoff `{supplied}` is not wave `{wave}`'s current handoff \
                 `{digest}` — re-read handoffs/{}.yaml and review that projection",
                current.digest.digest()
            ),
        });
    }
    let root = JournalRoot::new(layout.events_dir());
    let already = journal::read_union_at(&root)?.into_iter().any(|event| {
        matches!(
            &event.kind,
            EventKind::SystemWaveReviewed { wave: w, handoff_digest }
                if w == wave && handoff_digest == digest
        )
    });
    if already {
        return Ok(ReviewOutcome {
            wave: wave.to_string(),
            handoff_digest: digest.to_string(),
            recorded: false,
        });
    }
    let event = Event {
        timestamp: now,
        writer: String::new(),
        sequence: 0,
        kind: EventKind::SystemWaveReviewed {
            wave: wave.to_string(),
            handoff_digest: digest.to_string(),
        },
    };
    journal::append_for_at(&root, &journal::writer_id(), std::slice::from_ref(&event))?;
    Ok(ReviewOutcome {
        wave: wave.to_string(),
        handoff_digest: digest.to_string(),
        recorded: true,
    })
}
