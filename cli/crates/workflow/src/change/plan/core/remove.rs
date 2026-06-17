//! [`Plan::remove`]: drop one pending plan entry while the plan is still
//! replaceable (Gate 1 curation).

use specify_error::Error;

use super::model::{Lifecycle, Plan, Status};

impl Plan {
    /// Whether the plan accepts wholesale slice replacement (`propose
    /// --from`) or per-entry removal (`plan remove`).
    #[must_use]
    pub fn is_replaceable(&self) -> bool {
        self.lifecycle == Lifecycle::Pending
            && self.entries.iter().all(|e| e.status == Status::Pending)
    }

    /// Remove the entry named `name`. Allowed only while
    /// [`Plan::is_replaceable`] holds.
    ///
    /// # Errors
    ///
    /// Errors when the plan is not replaceable, the entry is missing,
    /// or another entry lists `name` in `depends-on`.
    pub fn remove(&mut self, name: &str) -> Result<(), Error> {
        if !self.is_replaceable() {
            return Err(Error::validation_failed(
                "plan-remove-plan-not-replaceable",
                "plan remove requires a replaceable plan",
                "lifecycle is approved or any entry is in-progress or done",
            ));
        }

        if !self.entries.iter().any(|e| e.name == name) {
            return Err(Error::Diag {
                code: "plan-entry-not-found",
                detail: format!("no slice named '{name}' in plan"),
            });
        }

        let referencers: Vec<&str> = self
            .entries
            .iter()
            .filter(|e| e.name != name && e.depends_on.iter().any(|d| d == name))
            .map(|e| e.name.as_str())
            .collect();
        if !referencers.is_empty() {
            return Err(Error::validation_failed(
                "plan-remove-entry-referenced",
                "plan remove refuses when another entry depends on the target",
                format!("slice '{name}' is listed in depends-on by: {}", referencers.join(", ")),
            ));
        }

        self.entries.retain(|e| e.name != name);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
