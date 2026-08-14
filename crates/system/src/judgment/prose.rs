//! The system guest's embedded prompt corpus.
//!
//! The `prompts/` tree is link-checked and embedded at build time into
//! the generated [`DOCS`] table; a dangling relative link fails the build.

/// One embedded prompt document.
#[derive(Debug, Clone, Copy)]
pub struct Doc {
    /// `prompts/`-relative path (e.g. `correlate.md`).
    pub path: &'static str,
    /// The document body, verbatim.
    pub body: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/prose_docs.rs"));

/// The embedded body at `prompts/<path>`.
///
/// # Panics
///
/// Panics when no document carries `path` — the corpus is embedded at
/// build time, so a miss is a programmer error, not a runtime state.
#[must_use]
pub fn doc(path: &str) -> &'static str {
    DOCS.iter()
        .find(|doc| doc.path == path)
        .unwrap_or_else(|| panic!("no embedded prompt at prompts/{path}"))
        .body
}

/// The correlation system prompt.
#[must_use]
pub fn correlate_system() -> &'static str {
    doc("correlate.md")
}

/// The initial-plan proposal system prompt.
#[must_use]
pub fn propose_system() -> &'static str {
    doc("propose.md")
}
