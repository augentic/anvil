//! Text-mode rendering of the bodies the command surface emits; the
//! JSON path is `Serialize`. Style follows the Developer Guide's CLI
//! output shapes.

use std::fmt;

use emery_engine::show::ShowBody;
use emery_engine::specify::SpecifyBody;

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
                for subject in &diff.added {
                    writeln!(out, "    + {subject}")?;
                }
                for subject in &diff.removed {
                    writeln!(out, "    - {subject}")?;
                }
                for subject in &diff.changed {
                    writeln!(out, "    ~ {subject}")?;
                }
            }
        }
        Ok(())
    }
}

// Text mode is the document alone — a deliberate exception to the
// result-line convention so `emery show spec` pipes cleanly; the
// revision id rides the JSON envelope.
impl Text for ShowBody {
    fn text(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        out.write_str(&self.body)
    }
}
