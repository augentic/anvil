//! The slice guest's embedded prompt corpus.
//!
//! Markdown stays the authoring source of truth: everything under this
//! crate's `prompts/` tree is link-checked and embedded at build time
//! into the generated [`DOCS`] table (see the `prose` build crate) —
//! a dangling relative reference fails the build. The corpus is small
//! (about 50 kilobytes), so it is pasted into the system prompt rather
//! than shelved behind an MCP route.

/// One embedded prompt document.
#[derive(Debug, Clone, Copy)]
pub struct Doc {
    /// `prompts/`-relative path (e.g. `synthesis/substeps.md`).
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

/// The synthesis playbook references appended to the authored
/// `synthesize.md` body, in citation order.
const SYNTHESIS_SECTIONS: &[&str] = &[
    "synthesis/substeps.md",
    "synthesis/requirement-block.md",
    "synthesis/authority.md",
    "synthesis/claim-reconciliation.md",
    "synthesis/tags.md",
    "synthesis/decisions.md",
    "synthesis/spec-format.md",
];

/// Assemble the synthesis system prompt: the authored prompt body plus
/// the playbook references as labeled sections, in citation order.
#[must_use]
pub fn synthesize_system() -> String {
    std::iter::once(doc("synthesize.md").to_string())
        .chain(SYNTHESIS_SECTIONS.iter().map(|path| section(path, doc(path))))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

fn section(label: &str, body: &str) -> String {
    format!("<!-- reference: {label} -->\n\n{body}")
}

// The corpus contract is private to this crate (the module is
// `pub(crate)`), so it is checked here against the private kernel
// rather than widened for an integration suite. Link resolution is
// enforced twice elsewhere: at embed time by the `prose` build crate
// and by the framework link gate over `crates/slice/prompts/`.
#[cfg(test)]
mod tests {
    use super::*;

    /// Every embedded document is consumed by the synthesize leg — the
    /// authored body or a synthesis playbook section. A prompt file
    /// that ships but assembles nowhere is a wiring bug, not harmless.
    #[test]
    fn every_doc_assembles() {
        for doc in DOCS {
            let consumed = doc.path == "synthesize.md" || SYNTHESIS_SECTIONS.contains(&doc.path);
            assert!(consumed, "embedded prompt `{}` assembles nowhere", doc.path);
        }
    }

    /// The synthesis system prompt assembles each playbook section
    /// exactly once, in the fixed citation order.
    #[test]
    fn synthesis_assembly_order() {
        let assembled = synthesize_system();
        let mut cursor = 0;
        for label in SYNTHESIS_SECTIONS {
            let marker = format!("<!-- reference: {label} -->");
            assert_eq!(assembled.matches(&marker).count(), 1, "{label} assembles exactly once");
            let position = assembled.find(&marker).expect("marker present");
            assert!(position > cursor, "{label} out of citation order");
            cursor = position;
        }
        assert!(
            assembled.starts_with(doc("synthesize.md")),
            "the authored prompt body leads the assembly"
        );
    }
}
