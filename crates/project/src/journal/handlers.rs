//! `emery journal show` — the read-only projection over the closed
//! workflow §Observability event taxonomy.
//!
//! Writes route through the internal appenders only: CLI verbs append
//! their own events as a side effect of the operation (there is no
//! operator-facing emit verb). `show` is a filter/limit projection
//! over the per-writer union under `.emery/events/` that emits nothing.

use std::io::Write;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use super::Event;
use crate::handler::{Anchor, Ctx, Render};

/// Wire input for `journal show`.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowInput {
    /// Keep only events whose dotted-kebab id starts with this prefix.
    #[serde(default)]
    pub filter: Option<String>,
    /// Keep only the most recent N matching events.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// `emery journal show [--filter <event-id-prefix>] [--limit N]`.
///
/// Read-only projection: emits no journal event and writes nothing.
/// Merges every `.emery/events/<writer>.jsonl` file in
/// `(timestamp, writer, sequence)` order. Text mode prints the
/// canonical JSONL lines (one `{ timestamp, writer, sequence, event,
/// payload }` object per event — pipeable); JSON wraps the same
/// events in the standard envelope as `{ count, events }`.
#[derive(Clone, Copy, Debug)]
pub struct Show;

impl<P: Anchor> Operation<P> for Show {
    type Error = crate::handler::Error;
    type Input = ShowInput;
    type Output = ShowBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let events = super::show(cx.layout(), input.filter.as_deref(), input.limit)?;
        Ok(ShowBody {
            count: events.len(),
            events,
        })
    }
}

/// Success envelope for `journal show`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowBody {
    /// Number of matching events.
    pub count: usize,
    /// Matches in union order (`timestamp`, `writer`, `sequence`).
    pub events: Vec<Event>,
}

impl Render for ShowBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.events.is_empty() {
            // Empty states always say so; there is no JSONL to pipe.
            return writeln!(w, "no events");
        }
        for event in &self.events {
            let line = serde_json::to_string(event).map_err(std::io::Error::other)?;
            writeln!(w, "{line}")?;
        }
        Ok(())
    }
}
