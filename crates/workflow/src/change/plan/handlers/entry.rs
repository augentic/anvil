//! Shared JSON envelope shape and text rendering for `plan add` /
//! `plan amend` / `plan remove`. The verbs report the resulting
//! [`Entry`] alongside a stable `action` discriminator so skill bodies
//! and tests can branch on which verb produced the body without
//! re-reading `plan.yaml`.

use std::io::Write;

use serde::Serialize;

use super::Ref;
use crate::change::Entry;
use crate::handler::Render;

/// Which mutating verb produced an [`EntryBody`].
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    /// `plan add`.
    Create,
    /// `plan amend`.
    Amend,
    /// `plan remove`.
    Remove,
}

/// Success envelope shared by the entry-mutating plan verbs.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EntryBody {
    /// The governing plan file.
    pub plan: Ref,
    /// Which verb produced this body.
    pub action: Action,
    /// The resulting entry as persisted.
    pub entry: Entry,
}

impl Render for EntryBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        let name = &self.entry.name;
        match self.action {
            Action::Create => writeln!(w, "Created plan entry '{name}' with status 'pending'."),
            Action::Amend => writeln!(w, "Amended plan entry '{name}'."),
            Action::Remove => writeln!(w, "Removed plan entry '{name}'."),
        }
    }
}
