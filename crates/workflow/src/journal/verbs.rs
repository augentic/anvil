//! `specify journal {emit, show}` — the front doors onto the closed
//! workflow §Observability event taxonomy.
//!
//! `emit` is the guarded write: it deserialises `<event-id>` +
//! `--payload` into the closed [`EventKind`]
//! (the taxonomy *is* the per-kind payload schema — there is no
//! parallel JSON-schema registry), stamps a second-precision UTC
//! timestamp, and appends exactly one well-formed line to
//! `.specify/journal.jsonl`. The emitter mints no event kinds of its
//! own. `show` is the read: a filter/limit projection over the same
//! file that emits nothing.

use std::io::Write;

use error::Error;
use omnia_guest::api::{Context, Handler, Reply};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{Event, EventKind};
use crate::verb::{Anchor, Ctx, Out, Render};

// ---------------------------------------------------------------------------
// journal emit
// ---------------------------------------------------------------------------

/// Wire input for `journal emit`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct EmitInput {
    /// Dotted-kebab event id (e.g. `slice.build.started`).
    pub event: String,
    /// JSON object carrying the event's payload fields (as a string);
    /// omit for events with no payload fields.
    #[serde(default)]
    pub payload: Option<String>,
}

/// `specify journal emit <event-id> [--payload <json>]`.
///
/// Reassembles the adjacently-tagged `{ event, payload }` wire shape
/// and runs a single serde round-trip into [`EventKind`]; the closed
/// taxonomy is the per-kind payload schema, so that one deserialise
/// validates both the id and the payload fields. The verb then stamps
/// a second-precision UTC timestamp (the [`Event`] serde format
/// truncates `Timestamp::now()` to seconds) and appends exactly one
/// line to `.specify/journal.jsonl` via [`journal::append_batch`].
///
/// Failures: `journal-emit-unknown-event` (exit 2) when `event` is not
/// a variant in the closed taxonomy; `journal-emit-payload-schema`
/// (exit 2) when `payload` is not valid JSON or does not satisfy the
/// named variant's field schema.
#[derive(Debug)]
pub struct Emit {
    input: EmitInput,
}

impl<P: Anchor> Handler<P> for Emit {
    type Error = crate::verb::Error;
    type Input = EmitInput;
    type Output = Out<EmitBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let EmitInput { event, payload } = self.input;
        // The `--payload` body defaults to an empty object so the single
        // round-trip below surfaces a `journal-emit-payload-schema`
        // missing-field failure for variants that require fields.
        let payload_value: Value = match payload.as_deref() {
            Some(raw) => serde_json::from_str(raw).map_err(|err| {
                payload_schema_error(format!("--payload is not valid JSON: {err}"))
            })?,
            None => Value::Object(Map::new()),
        };

        let mut tagged = Map::new();
        tagged.insert("event".to_string(), Value::String(event.clone()));
        tagged.insert("payload".to_string(), payload_value);

        let kind: EventKind =
            serde_json::from_value(Value::Object(tagged)).map_err(|err| classify(&event, &err))?;

        let journal_event = Event::new(cx.now(), kind);
        super::append_batch(cx.layout(), std::slice::from_ref(&journal_event))?;

        Ok(Reply::ok(Out(EmitBody { event })))
    }
}

/// Split a failed [`EventKind`] deserialise into the two operator-
/// facing buckets. An unknown adjacently-tagged variant surfaces as
/// serde's `unknown variant` error; everything else (missing/invalid
/// payload field) is a payload-schema failure.
fn classify(event: &str, err: &serde_json::Error) -> Error {
    let message = err.to_string();
    if message.contains("unknown variant") {
        Error::validation_failed(
            "journal-emit-unknown-event",
            "<event-id> must name a variant in the closed journal taxonomy",
            format!("unknown journal event id `{event}`: {message}"),
        )
    } else {
        payload_schema_error(format!(
            "payload does not satisfy the `{event}` event schema: {message}"
        ))
    }
}

fn payload_schema_error(detail: String) -> Error {
    Error::validation_failed(
        "journal-emit-payload-schema",
        "--payload must satisfy the named event's field schema",
        detail,
    )
}

/// Success envelope for `journal emit`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EmitBody {
    /// The appended event id.
    pub event: String,
}

impl Render for EmitBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "Appended journal event '{}'.", self.event)
    }
}

// ---------------------------------------------------------------------------
// journal show
// ---------------------------------------------------------------------------

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

/// `specify journal show [--filter <event-id-prefix>] [--limit N]`.
///
/// Read-only projection: emits no journal event and writes nothing.
/// Text mode prints the canonical JSONL lines (one `{ timestamp,
/// event, payload }` object per event — pipeable, replacing ad-hoc
/// `jq` bridges over the file); JSON wraps the same events in the
/// standard envelope as `{ count, events }`.
#[derive(Debug)]
pub struct Show {
    input: ShowInput,
}

impl<P: Anchor> Handler<P> for Show {
    type Error = crate::verb::Error;
    type Input = ShowInput;
    type Output = Out<ShowBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let events = super::show(cx.layout(), self.input.filter.as_deref(), self.input.limit)?;
        Ok(Reply::ok(Out(ShowBody {
            count: events.len(),
            events,
        })))
    }
}

/// Success envelope for `journal show`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowBody {
    /// Matching event count.
    pub count: usize,
    /// The matching events in append order.
    pub events: Vec<Event>,
}

impl Render for ShowBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        for event in &self.events {
            let line = serde_json::to_string(event).map_err(std::io::Error::other)?;
            writeln!(w, "{line}")?;
        }
        Ok(())
    }
}
