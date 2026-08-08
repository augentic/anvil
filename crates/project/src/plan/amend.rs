//! [`Plan::amend`]: in-place edit of an existing entry's non-status fields.

use diagnostics::is_blocking;
use error::Error;

use super::model::{EntryPatch, Plan};
use crate::plan::detect;

impl Plan {
    /// Apply `patch` to the entry named `name`.
    ///
    /// Wholesale-replacement fields replace when `Some`; nullable
    /// fields take the three-way [`Patch`](super::model::Patch)
    /// (`Keep` / `Clear` / `Set`). Progress projects from facts and is
    /// not amendable. After mutation the plan is re-validated; any
    /// `Error`-level finding reverts the single-entry mutation. It is
    /// legal to amend an in-progress entry while a claim fact is live.
    ///
    /// # Errors
    ///
    /// Errors when no entry matches `name` or when post-amend
    /// validation fails.
    pub fn amend(&mut self, name: &str, patch: EntryPatch) -> Result<(), Error> {
        let Some(idx) = self.entries.iter().position(|c| c.name == name) else {
            return Err(self.entry_not_found(name));
        };

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

        let errors: Vec<_> = self.validate(None, None).into_iter().filter(is_blocking).collect();
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
