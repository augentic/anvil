//! Clap derive surface for `specify journal *`. The umbrella `cli.rs`
//! re-exports `JournalAction`.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum JournalAction {
    /// Append one event to `.specify/journal.jsonl`.
    ///
    /// `<event-id>` names a variant in the closed workflow
    /// §Observability event taxonomy (e.g. `source.execution.agent`);
    /// `--payload` carries that variant's fields as a JSON object. The
    /// taxonomy *is* the payload schema — a single serde round-trip
    /// validates both the id and the fields. An unknown id exits `2`
    /// with `journal-emit-unknown-event`; a payload that fails the
    /// variant's field schema exits `2` with
    /// `journal-emit-payload-schema`. On success the CLI stamps a
    /// second-precision UTC timestamp and appends exactly one line.
    Emit {
        /// Dotted-kebab event id (e.g. `slice.build.started`).
        event: String,

        /// JSON object carrying the event's payload fields (e.g.
        /// `{"source":"runtime","adapter":"captures",...}`).
        /// Omit for events with no payload fields.
        #[arg(long)]
        payload: Option<String>,
    },

    /// Read events from `.specify/journal.jsonl` in append order.
    ///
    /// Read-only: emits no journal event and writes nothing. Text mode
    /// prints the canonical JSONL lines — one `{ timestamp, event,
    /// payload }` object per event, pipeable — while `--format json`
    /// wraps the same events in the standard envelope. Blank and
    /// unparseable lines are skipped, matching every other journal
    /// reader; a missing journal yields no events.
    Show {
        /// Keep only events whose dotted-kebab id starts with this
        /// prefix (e.g. `slice.build` or `plan.entry.advanced`).
        #[arg(long)]
        filter: Option<String>,

        /// Keep only the most recent N matching events.
        #[arg(long)]
        limit: Option<usize>,
    },
}
