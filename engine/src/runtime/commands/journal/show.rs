//! `specify journal show` handler — the read verb over
//! `.specify/journal.jsonl`. Filtering and limit semantics live in
//! [`specify_workflow::journal::show`]; this handler only renders.

use std::io::Write;

use serde::Serialize;
use specify_error::Result;
use specify_workflow::journal::{self, Event};

use crate::runtime::context::Ctx;

/// `specify journal show [--filter <event-id-prefix>] [--limit N]`.
///
/// Read-only projection: emits no journal event and writes nothing.
/// Text mode prints the canonical JSONL lines (one `{ timestamp,
/// event, payload }` object per event — pipeable, replacing ad-hoc
/// `jq` bridges over the file); `--format json` wraps the same events
/// in the standard envelope as `{ count, events }`.
pub fn show(ctx: &Ctx, filter: Option<&str>, limit: Option<usize>) -> Result<()> {
    let events = journal::show(ctx.layout(), filter, limit)?;
    ctx.write(
        &ShowBody {
            count: events.len(),
            events,
        },
        write_show_text,
    )?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct ShowBody {
    count: usize,
    events: Vec<Event>,
}

fn write_show_text(w: &mut dyn Write, body: &ShowBody) -> std::io::Result<()> {
    for event in &body.events {
        let line = serde_json::to_string(event).map_err(std::io::Error::other)?;
        writeln!(w, "{line}")?;
    }
    Ok(())
}
