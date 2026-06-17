//! Plan scaffolding: [`Plan::init`] for an empty plan and
//! [`Plan::create`] for a single-entry append.

use std::collections::BTreeMap;

use specify_diagnostics::blocking;
use specify_error::Error;

use super::model::{Entry, Lifecycle, Plan, SourceBinding, Status};
use crate::change::detect;
use crate::slice::actions::validate_name;

impl Plan {
    /// Create an empty plan with the given name and optional named sources.
    ///
    /// Every entry starts with `status: pending`; this just initialises the
    /// top-level struct. The name is validated with
    /// [`crate::slice::actions::validate_name`] so it obeys the same kebab-case
    /// rules as change names.
    ///
    /// Does NOT write anything to disk. Call [`Plan::save`] afterwards.
    ///
    /// # Errors
    ///
    /// Errors when `name` is not kebab-case.
    pub fn init(name: &str, sources: BTreeMap<String, SourceBinding>) -> Result<Self, Error> {
        validate_name(name)?;
        Ok(Self {
            name: name.into(),
            lifecycle: Lifecycle::Pending,
            sources,
            entries: vec![],
        })
    }

    /// Append a new entry to the plan, rejecting duplicate names and
    /// invalid kebab-case names. The incoming `status` is forced to
    /// [`Status::Pending`] (and `status_reason` cleared) so that
    /// creation cannot introduce a pre-occupied lifecycle state — the
    /// single-writer-for-status invariant documented in
    /// [`Plan::transition`].
    ///
    /// After mutation, the plan is re-validated. Any `Error`-level
    /// finding (unknown `depends_on`/`sources`, cycle introduced by the
    /// new entry, etc.) rolls back the append and returns an
    /// `Error::Diag`. Warnings are tolerated — they're a CLI concern,
    /// not a library-level hard stop.
    ///
    /// # Errors
    ///
    /// Errors when the name is invalid, when an entry with the same
    /// name already exists, or when post-append validation fails.
    pub fn create(&mut self, change: Entry) -> Result<(), Error> {
        validate_name(&change.name)?;

        if self.entries.iter().any(|c| c.name == change.name) {
            return Err(Error::Diag {
                code: "plan-entry-duplicate-name",
                detail: format!("plan already contains a change named '{}'", change.name),
            });
        }

        let mut change = change;
        change.status = Status::Pending;

        self.entries.push(change);
        let errors: Vec<_> = self.validate(None, None).into_iter().filter(blocking).collect();
        let failure_msg = errors
            .first()
            .map(|r| r.impact.clone())
            .or_else(|| detect(&self.entries).into_iter().next().map(|d| d.impact));
        if let Some(msg) = failure_msg {
            self.entries.pop();
            return Err(Error::Diag {
                code: "plan-create-validation-failed",
                detail: format!("plan validation failed after create: {msg}"),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
