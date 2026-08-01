//! The change guest's embedded prompt corpus.
//!
//! Markdown stays the authoring source of truth: everything under this
//! crate's `prompts/` tree is link-checked and embedded at build time
//! into the generated [`DOCS`] table (see the `prose` build crate) —
//! a dangling relative reference fails the build. The corpus is small,
//! so it is pasted into the system prompt rather than shelved behind
//! an MCP route.

/// One embedded prompt document.
#[derive(Debug, Clone, Copy)]
pub struct Doc {
    /// `prompts/`-relative path (e.g. `propose.md`).
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

/// System prompt body for the propose reconciliation leg.
#[must_use]
pub fn propose() -> &'static str {
    doc("propose.md")
}

// Keep (private embed-table kernel): the generated `DOCS` table has no
// public projection — "an embedded prompt assembles nowhere" is
// unobservable at any CLI or crate boundary, so integration cannot own
// this invariant. Link integrity is enforced separately at embed time
// by the `prose` build crate.
#[cfg(test)]
mod tests {
    use super::*;

    /// Every embedded document is consumed by the propose leg. A
    /// prompt file that ships but assembles nowhere is a wiring bug,
    /// not harmless.
    #[test]
    fn every_doc_assembles() {
        for doc in DOCS {
            assert!(doc.path == "propose.md", "embedded prompt `{}` assembles nowhere", doc.path);
        }
    }
}
