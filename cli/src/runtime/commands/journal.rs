//! `specify journal {emit, show}` — the front doors onto the closed
//! workflow §Observability event taxonomy.
//!
//! `emit` is the guarded write: it deserialises `<event-id>` +
//! `--payload` into the closed [`specify_workflow::journal::EventKind`]
//! (the taxonomy *is* the per-kind payload schema — there is no
//! parallel JSON-schema registry), stamps a second-precision UTC
//! timestamp, and appends exactly one well-formed line to
//! `.specify/journal.jsonl`. The emitter mints no event kinds of its
//! own. `show` is the read: a filter/limit projection over the same
//! file that emits nothing.

pub mod cli;
pub mod emit;
pub mod show;
