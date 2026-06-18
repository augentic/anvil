//! Shared JSON envelope shape and text writer for `plan add` /
//! `plan amend`. Both verbs report the resulting [`Entry`] alongside
//! a stable `action` discriminator so skill bodies and tests can
//! branch on which verb produced the body without re-reading
//! `plan.yaml`.

use std::io::Write;

use serde::Serialize;
use specify_workflow::change::Entry;

use super::Ref;

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Action {
    Create,
    Amend,
    Remove,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) struct EntryBody {
    pub plan: Ref,
    pub action: Action,
    pub entry: Entry,
}

pub(super) fn write_entry_text(w: &mut dyn Write, body: &EntryBody) -> std::io::Result<()> {
    let name = &body.entry.name;
    match body.action {
        Action::Create => writeln!(w, "Created plan entry '{name}' with status 'pending'."),
        Action::Amend => writeln!(w, "Amended plan entry '{name}'."),
        Action::Remove => writeln!(w, "Removed plan entry '{name}'."),
    }
}
