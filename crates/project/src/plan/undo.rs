//! Fact-based `plan undo` kernel (RFC-86 D2 / D7).
//!
//! Walks one projected ladder rung backwards by appending
//! `fact.retracted` (and a projection-label `plan.transition.undone`)
//! — it does **not** rewrite stored `Entry.status`.

use error::Error;
use jiff::Timestamp;

use super::execution::{collect_events, project_ladders};
use super::model::{Plan, Status};
use crate::config::Layout;
use crate::journal::{self, Event, EventKind, claim};
use crate::name::SliceName;

/// One `(from, to)` rung an undo walk visited (projection labels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndoStep {
    /// Ladder label before the reverse step.
    pub from: Status,
    /// Ladder label after the reverse step.
    pub to: Status,
}

/// Walk `name` one rung (or until `to`) backwards on the projected
/// ladder. Appends retract facts; leaves `plan.yaml` untouched.
///
/// # Errors
///
/// - [`Error::ArtifactNotFound`] when `plan.yaml` is absent.
/// - `plan-entry-not-found` when `name` is not on the plan.
/// - `plan-transition-undo` when already at the target, or at
///   `pending` with nothing to undo.
/// - journal I/O failures.
pub fn undo_entry(
    layout: Layout<'_>, now: Timestamp, name: &str, to: Option<Status>,
) -> Result<Vec<UndoStep>, Error> {
    let plan = Plan::load(&layout.plan_path())?;
    let slice: SliceName = name.into();
    if !plan.entries.iter().any(|entry| entry.name == slice) {
        return Err(plan.entry_not_found(name));
    }

    let events = collect_events(&plan, layout)?;
    let ladders = project_ladders(&plan, &events);
    let current = ladders.get(&slice).copied().unwrap_or(Status::Pending);
    if let Some(target) = to
        && current == target
    {
        return Err(Error::Diag {
            code: "plan-transition-undo",
            detail: format!("entry `{name}` is already `{target}`; nothing to undo"),
        });
    }

    let mut steps = Vec::new();
    loop {
        let events = collect_events(&plan, layout)?;
        let ladders = project_ladders(&plan, &events);
        let from = ladders.get(&slice).copied().unwrap_or(Status::Pending);
        let to_step = match from {
            Status::Done => Status::InProgress,
            Status::InProgress => Status::Pending,
            Status::Pending => {
                return Err(Error::Diag {
                    code: "plan-transition-undo",
                    detail: format!(
                        "cannot undo from `pending` on slice `{name}`; `pending` is the entry \
                         point and has no prior status to reinstate"
                    ),
                });
            }
        };
        retract_rung(layout, now, &plan, &slice, from, &events)?;
        journal::append_one(
            layout,
            &Event::new(
                now,
                EventKind::PlanTransitionUndone {
                    plan_name: plan.name.clone(),
                    slice_name: slice.clone(),
                    from,
                    to: to_step,
                },
            ),
        )?;
        steps.push(UndoStep { from, to: to_step });
        match to {
            None => break,
            Some(target) if to_step == target => break,
            Some(_) => {}
        }
    }
    Ok(steps)
}

/// Retract the facts that project `from` for this slice.
fn retract_rung(
    layout: Layout<'_>, now: Timestamp, plan: &Plan, slice: &SliceName, from: Status,
    events: &[Event],
) -> Result<(), Error> {
    let retracted = claim::retracted_targets(events);
    let targets: Vec<(String, u64)> = match from {
        Status::Done => events
            .iter()
            .rev()
            .filter(|event| !retracted.contains(&(event.actor.as_str(), event.sequence)))
            .filter_map(|event| match &event.kind {
                EventKind::SliceArchiveCreated { slice_name, .. }
                | EventKind::SliceMergePostflightFailed { slice_name, .. }
                    if slice_name == slice =>
                {
                    Some((event.actor.clone(), event.sequence))
                }
                _ => None,
            })
            .take(1)
            .collect(),
        Status::InProgress => {
            let mut targets = Vec::new();
            for event in events.iter().rev() {
                if retracted.contains(&(event.actor.as_str(), event.sequence)) {
                    continue;
                }
                match &event.kind {
                    EventKind::SliceClaimed { slice_name } if slice_name == slice => {
                        targets.push((event.actor.clone(), event.sequence));
                    }
                    EventKind::PlanEntryAdvanced {
                        plan_name,
                        slice_name,
                    } if plan_name == &plan.name && slice_name == slice => {
                        targets.push((event.actor.clone(), event.sequence));
                    }
                    _ => {}
                }
            }
            // One retract per distinct line; newest-first already.
            let mut seen = std::collections::BTreeSet::new();
            targets
                .into_iter()
                .filter(|(actor, sequence)| seen.insert((actor.clone(), *sequence)))
                .collect()
        }
        Status::Pending => Vec::new(),
    };
    for (actor, sequence) in targets {
        journal::append_one(
            layout,
            &Event::new(now, EventKind::FactRetracted { actor, sequence }),
        )?;
    }
    Ok(())
}
