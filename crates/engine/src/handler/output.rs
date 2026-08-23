//! Transport-neutral output rendering.

use std::io::Write;

use serde::Serialize;

/// Human-readable rendering for a serializable command body.
pub trait Render: Serialize {
    /// Writes `self` to `w`.
    ///
    /// # Errors
    ///
    /// Propagates I/O errors.
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()>;
}
