//! Shared JSON envelope and text rendering for `plan add` / `plan
//! amend` / `plan remove`: the resulting [`Entry`] plus a stable
//! `action` discriminator naming which verb produced the body.

use std::io::Write;

use project::handler::Render;
use project::plan::Entry;
use serde::Serialize;

use super::Ref;

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
            Action::Create => writeln!(w, "created plan entry `{name}`")?,
            Action::Amend => writeln!(w, "amended plan entry `{name}`")?,
            Action::Remove => writeln!(w, "removed plan entry `{name}`")?,
        }
        writeln!(w, "  plan: {}", self.plan.path.display())
    }
}
