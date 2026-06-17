//! `Plan::transition` and `Plan::transition_lifecycle` — the only
//! writers of `Entry::status` for `done` and `Plan::lifecycle` for
//! `approved`. The legal edges are `Pending → InProgress` (written by
//! `plan next`, never here) and `InProgress → Done` per entry, plus
//! `Pending → Approved` plan-level (operator stamp at Gate 1, workflow
//! §The Plan / §Writer ownership; `/spec:plan` MUST NOT call it).

use specify_error::Error;

use super::model::{Entry, Lifecycle, Plan, Status};

impl Plan {
    /// Transition the named entry to `target` (in practice always
    /// [`Status::Done`]; `Pending → InProgress` is reserved for
    /// `Plan::next`).
    ///
    /// # Errors
    /// `plan-entry-not-found` / `plan-transition` — see module docs.
    pub fn transition(&mut self, name: &str, target: Status) -> Result<(), Error> {
        let entry: &mut Entry =
            self.entries.iter_mut().find(|c| c.name == name).ok_or_else(|| Error::Diag {
                code: "plan-entry-not-found",
                detail: format!("no slice named '{name}' in plan"),
            })?;
        let current = entry.status;
        if matches!(
            (current, target),
            (Status::Pending, Status::InProgress) | (Status::InProgress, Status::Done)
        ) {
            entry.status = target;
            Ok(())
        } else {
            Err(Error::Diag {
                code: "plan-transition",
                detail: format!("cannot transition from {current:?} to {target:?}"),
            })
        }
    }

    /// Transition [`Plan::lifecycle`] to `target` — see module docs.
    ///
    /// # Errors
    /// `plan-lifecycle-transition` when the edge is not legal.
    pub fn transition_lifecycle(&mut self, target: Lifecycle) -> Result<(), Error> {
        let current = self.lifecycle;
        if matches!((current, target), (Lifecycle::Pending, Lifecycle::Approved)) {
            self.lifecycle = target;
            Ok(())
        } else {
            Err(Error::Diag {
                code: "plan-lifecycle-transition",
                detail: format!("cannot transition plan lifecycle from {current:?} to {target:?}"),
            })
        }
    }

    /// Walk a single entry one step backwards along the legal v1
    /// lifecycle (`Done → InProgress`, `InProgress → Pending`) and
    /// return `(from, to)` so the caller can emit the matching
    /// `plan.transition.undone` journal event.
    ///
    /// The undo verb refuses to skip rungs — `Done → Pending` MUST
    /// run twice so the journal records each rung independently and
    /// the operator never lands in a state the forward path cannot
    /// reach. `Pending` has no predecessor (no prior status to
    /// reinstate); `plan add` / `plan amend` are the only writers
    /// of `Pending`.
    ///
    /// # Errors
    ///
    /// - `plan-entry-not-found` when no entry on `self` matches
    ///   `name`.
    /// - `plan-transition-undo` when the entry is already at
    ///   `Pending` (nothing to undo).
    pub fn transition_undo(&mut self, name: &str) -> Result<(Status, Status), Error> {
        let entry: &mut Entry =
            self.entries.iter_mut().find(|c| c.name == name).ok_or_else(|| Error::Diag {
                code: "plan-entry-not-found",
                detail: format!("no slice named '{name}' in plan"),
            })?;
        let from = entry.status;
        let to = match from {
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
        entry.status = to;
        Ok((from, to))
    }
}

#[cfg(test)]
mod tests;
