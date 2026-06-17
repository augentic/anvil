//! `specify journal emit` handler. A guarded front door onto the
//! closed [`specify_workflow::journal::EventKind`] taxonomy: the
//! handler mints no event kinds of its own, it only deserialises the
//! operator-supplied id + payload into an existing variant, stamps the
//! timestamp, and appends one line.

use std::io::Write;

use serde::Serialize;
use serde_json::{Map, Value};
use specify_error::{Error, Result};
use specify_workflow::journal::{self, Event, EventKind};

use crate::runtime::context::Ctx;

/// `specify journal emit <event-id> [--payload <json>]`.
///
/// Reassembles the adjacently-tagged `{ event, payload }` wire shape
/// and runs a single serde round-trip into [`EventKind`]; the closed
/// taxonomy is the per-kind payload schema, so that one deserialise
/// validates both the id and the payload fields. The CLI then stamps a
/// second-precision UTC timestamp (the [`Event`] serde format truncates
/// `Timestamp::now()` to seconds) and appends exactly one line to
/// `.specify/journal.jsonl` via [`journal::append_batch`].
///
/// # Errors
///
/// - `journal-emit-unknown-event` (exit 2) — `event` is not a variant
///   in the closed taxonomy.
/// - `journal-emit-payload-schema` (exit 2) — `payload` is not valid
///   JSON or does not satisfy the named variant's field schema.
pub fn emit(ctx: &Ctx, event: &str, payload: Option<&str>) -> Result<()> {
    // The `--payload` body defaults to an empty object so the single
    // round-trip below surfaces a `journal-emit-payload-schema`
    // missing-field failure for variants that require fields.
    let payload_value: Value = match payload {
        Some(raw) => serde_json::from_str(raw)
            .map_err(|err| payload_schema_error(format!("--payload is not valid JSON: {err}")))?,
        None => Value::Object(Map::new()),
    };

    let mut tagged = Map::new();
    tagged.insert("event".to_string(), Value::String(event.to_string()));
    tagged.insert("payload".to_string(), payload_value);

    let kind: EventKind =
        serde_json::from_value(Value::Object(tagged)).map_err(|err| classify(event, &err))?;

    let journal_event = Event::new(ctx.now(), kind);
    journal::append_batch(ctx.layout(), std::slice::from_ref(&journal_event))?;

    ctx.write(
        &EmitBody {
            event: event.to_string(),
        },
        write_emit_text,
    )?;
    Ok(())
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

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct EmitBody {
    event: String,
}

fn write_emit_text(w: &mut dyn Write, body: &EmitBody) -> std::io::Result<()> {
    writeln!(w, "Appended journal event '{}'.", body.event)
}
