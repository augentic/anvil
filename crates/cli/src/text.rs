//! Text output
//!
//! The human-readable rendering of each command result. JSON output falls
//! out of the result types' `Serialize` derives; text output needs a hand
//! written shape per result, and those shapes live here.
//!
//! Keeping text rendering apart from the engine's result types lets the
//! terminal presentation follow the Developer Guide's output conventions
//! without those conventions leaking into the engine.

use std::fmt;

use emery_engine::show::ShowBody;
use emery_engine::specify::{Changes, SpecifyBody};

/// Terminal text rendering of one body.
pub trait Text {
    /// Writes the text-mode rendering.
    ///
    /// # Errors
    ///
    /// Propagates the sink's formatting failure.
    fn text(&self, out: &mut dyn fmt::Write) -> fmt::Result;
}

impl Text for SpecifyBody {
    fn text(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        writeln!(out, "committed revision {}", self.revision)?;
        writeln!(out, "  requirements: {}", self.requirements)?;
        writeln!(out, "  sources: {}", self.sources)?;
        for entry in &self.digests {
            writeln!(out, "  digest {}: {}", entry.source, entry.digest)?;
        }
        if let Some(diff) = &self.diff {
            if diff.is_empty() {
                writeln!(out, "  diff vs {}: none (byte-stable)", diff.from)?;
            } else {
                writeln!(out, "  diff vs {}: {}", diff.from, diff.artifacts.join(", "))?;
                changes(out, "spec.md", &diff.spec)?;
                changes(out, "design.md", &diff.design)?;
            }
        }
        Ok(())
    }
}

// One line per changed section, prefixed by its document.
fn changes(out: &mut dyn fmt::Write, document: &str, changes: &Changes) -> fmt::Result {
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

// Text mode is the document alone — a deliberate exception to the
// result-line convention so `emery show spec` pipes cleanly; the
// revision id rides the JSON envelope.
impl Text for ShowBody {
    fn text(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        out.write_str(&self.body)
    }
}
