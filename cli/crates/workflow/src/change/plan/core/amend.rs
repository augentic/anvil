//! [`Plan::amend`]: in-place edit of an existing entry's non-status fields.

use specify_diagnostics::blocking;
use specify_error::Error;

use super::model::{EntryPatch, Plan};
use crate::change::detect;

impl Plan {
    /// Apply `patch` to the entry named `name`. Wholesale-replacement
    /// fields (`depends_on`, `sources`, `context`) replace when `Some`
    /// and leave the corresponding
    /// [`Entry`](super::model::Entry) field unchanged when `None`.
    /// Nullable fields (`project`, `target`, `description`) take a
    /// three-way [`Patch`](super::model::Patch): `Keep` leaves the field
    /// alone, `Clear` sets it to `None`, `Set(v)` replaces it with
    /// `Some(v)`. `status` is intentionally not patchable — see
    /// [`EntryPatch`] and [`Plan::transition`] for the
    /// single-writer-for-status note.
    ///
    /// After mutation, the plan is re-validated. Any `Error`-level
    /// finding reverts the single-entry mutation (we snapshot the
    /// pre-mutation entry at the top of the function and write it
    /// back on failure) and returns an `Error::Diag`.
    ///
    /// `amend` does not consult `Entry::status` — it is legal to
    /// amend the currently-`in-progress` entry's non-status fields,
    /// per the execution spec's §"Phase Boundary → Rule 2".
    ///
    /// # Errors
    ///
    /// Errors when no entry matches `name` or when post-amend
    /// validation fails.
    pub fn amend(&mut self, name: &str, patch: EntryPatch) -> Result<(), Error> {
        let idx = self.entries.iter().position(|c| c.name == name).ok_or_else(|| Error::Diag {
            code: "plan-entry-not-found",
            detail: format!("no slice named '{name}' in plan"),
        })?;

        let snapshot = self.entries[idx].clone();

        {
            let entry = &mut self.entries[idx];
            if let Some(v) = patch.depends_on {
                entry.depends_on = v;
            }
            if let Some(v) = patch.sources {
                entry.sources = v;
            }
            patch.project.apply(&mut entry.project);
            patch.description.apply(&mut entry.description);
            if let Some(v) = patch.context {
                entry.context = v;
            }
            if let Some(d) = patch.divergence {
                entry.divergence = Some(d);
            }
        }

        let errors: Vec<_> = self.validate(None, None).into_iter().filter(blocking).collect();
        let failure_msg = errors
            .first()
            .map(|r| r.impact.clone())
            .or_else(|| detect(&self.entries).into_iter().next().map(|d| d.impact));
        if let Some(msg) = failure_msg {
            self.entries[idx] = snapshot;
            return Err(Error::Diag {
                code: "plan-amend-validation-failed",
                detail: format!("plan validation failed after amend: {msg}"),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
