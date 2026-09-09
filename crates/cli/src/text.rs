//! Text output
//!
//! The human-readable rendering of each command result. JSON output falls
//! out of the result types' `Serialize` derives; text output needs a hand
//! written shape per result, and those shapes live here as the render fns
//! the command projector encodes `--format text` through.
//!
//! Keeping text rendering apart from the engine's result types lets the
//! terminal presentation follow the Developer Guide's output conventions
//! without those conventions leaking into the engine.

use std::fmt;

use emery_engine::show::{Document, ShowBody};
use emery_engine::specify::{Changes, SpecifyBody};

/// The `specify` result line and its indented detail.
pub fn specify(body: &SpecifyBody, out: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(out, "committed revision {}", body.revision)?;
    writeln!(out, "  requirements: {}", body.requirements)?;
    writeln!(out, "  sources: {}", body.sources)?;
    for entry in &body.digests {
        writeln!(out, "  digest {}: {}", entry.source, entry.digest)?;
    }
    if let Some(diff) = &body.diff {
        if diff.is_empty() {
            writeln!(out, "  diff vs {}: none (byte-stable)", diff.from)?;
        } else {
            writeln!(out, "  diff vs {}: {}", diff.from, diff.artifacts.join(", "))?;
            changes(out, Document::Spec, &diff.spec)?;
            changes(out, Document::Design, &diff.design)?;
        }
    }
    Ok(())
}

// One line per changed section, prefixed by its document.
fn changes(out: &mut dyn fmt::Write, document: Document, changes: &Changes) -> fmt::Result {
    let document = document.file();
    for heading in &changes.added {
        writeln!(out, "    {document} + {heading}")?;
    }
    for heading in &changes.removed {
        writeln!(out, "    {document} - {heading}")?;
    }
    for heading in &changes.changed {
        writeln!(out, "    {document} ~ {heading}")?;
    }
    Ok(())
}

/// The document alone — a deliberate exception to the result-line
/// convention so `emery show spec` pipes cleanly; the revision id rides
/// the JSON envelope.
pub fn show(body: &ShowBody, out: &mut dyn fmt::Write) -> fmt::Result {
    out.write_str(&body.body)
}
