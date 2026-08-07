//! [`Plan::remove`]: drop one pending plan entry while the plan is still
//! replaceable (plan-review curation).

use std::collections::HashMap;
use std::hash::BuildHasher;

use error::Error;

use super::model::{Plan, Status};
use crate::name::SliceName;

impl Plan {
    /// Whether the plan accepts wholesale slice replacement (the
    /// reconciliation kernel) or per-entry removal (`plan remove`) —
    /// true while every projected ladder label is still `pending`.
    #[must_use]
    pub fn is_replaceable<S: BuildHasher>(ladders: &HashMap<SliceName, Status, S>) -> bool {
        ladders.values().all(|status| *status == Status::Pending)
    }

    /// Remove the entry named `name`. Allowed only while every
    /// projected ladder is still `pending`.
    ///
    /// # Errors
    ///
    /// Errors when the plan is not replaceable, the entry is missing,
    /// or another entry lists `name` in `depends-on`.
    pub fn remove<S: BuildHasher>(
        &mut self, name: &str, ladders: &HashMap<SliceName, Status, S>,
    ) -> Result<(), Error> {
        if !Self::is_replaceable(ladders) {
            return Err(Error::validation_failed(
                "plan-remove-plan-not-replaceable",
                "plan remove requires a replaceable plan",
                "an entry is in-progress or done",
            ));
        }

        if !self.entries.iter().any(|e| e.name == name) {
            return Err(self.entry_not_found(name));
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
