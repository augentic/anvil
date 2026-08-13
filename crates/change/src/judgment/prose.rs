//! The change guest's embedded prompt corpus: the `prompts/` tree is
//! link-checked and embedded at build time into the generated [`DOCS`]
//! table; a dangling relative reference fails the build.

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

/// System prompt body for the partition judgment.
#[must_use]
pub fn partition() -> &'static str {
    doc("partition.md")
}

/// System prompt body for the boundary-review judgment.
#[must_use]
pub fn review() -> &'static str {
    doc("review.md")
}

// Keep (private embed-table kernel): "an embedded prompt assembles
// nowhere" is unobservable at any CLI or crate boundary, so
// integration cannot own this invariant.
#[cfg(test)]
mod tests {
    use super::*;

    /// Every embedded document is consumed by a judgment leg. A
    /// prompt file that ships but assembles nowhere is a wiring bug,
    /// not harmless.
    #[test]
    fn every_doc_assembles() {
        for doc in DOCS {
            assert!(
                matches!(doc.path, "propose.md" | "partition.md" | "review.md"),
                "embedded prompt `{}` assembles nowhere",
                doc.path
            );
        }
    }
}
