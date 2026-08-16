//! The slice guest's embedded prompt corpus.
//!
//! The `prompts/` tree is link-checked and embedded at build time into
//! the generated [`DOCS`] table; a dangling relative link fails the build.

use crate::shelf::SERVER as SHELF_SERVER;

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
    "synthesis/boundary.md",
    "synthesis/requirement-block.md",
    "synthesis/authority.md",
    "synthesis/claim-reconciliation.md",
    "synthesis/tags.md",
    "synthesis/decisions.md",
    "synthesis/spec-format.md",
];

/// The measured inline minimum (RFC-96 D9): the sections whose loss
/// the deterministic tail cannot repair cheaply — the proceed /
/// escalation contract (`boundary.md`, tail-gated assessment and
/// escalation checks) and the spec heading grammar (`spec-format.md`,
/// parsed by the provenance validator). Together with `synthesize.md`
/// they are 130 of the corpus's 514 lines (9.1 KB of 35.7 KB
/// assembled); the remaining six playbook sections (~26.6 KB) load
/// from the shelf.
const INLINE_SECTIONS: &[&str] = &["synthesis/boundary.md", "synthesis/spec-format.md"];

/// One-line shelf summaries, in citation order, rendered into the
/// MCP-loading instruction so the agent knows what each document
/// carries before fetching it.
const SHELF_SUMMARIES: &[(&str, &str)] = &[
    (
        "synthesis/substeps.md",
        "the four-artifact substep contract (proposal → specs → design → tasks) and what each substep reads",
    ),
    (
        "synthesis/requirement-block.md",
        "the requirement-block shape and exactly which lines the kernel renders",
    ),
    (
        "synthesis/authority.md",
        "the authority hierarchy, overrides, and what the kernel resolves for you",
    ),
    (
        "synthesis/claim-reconciliation.md",
        "per-kind claim grouping and where each claim kind lands",
    ),
    ("synthesis/tags.md", "the closed review-signal tag set the kernel renders"),
    ("synthesis/decisions.md", "the bar and shape for optional decisions[] entries"),
];

/// Assemble the synthesis system prompt (RFC-96 D9).
///
/// With a granted shelf URL: the authored body, the inline-minimum
/// sections, and an MCP-loading instruction naming every shelf
/// document. Without one: the full playbook inlined as labeled
/// sections, in citation order — offline deployments degrade to the
/// pre-shelf prompt.
#[must_use]
pub fn synthesize_system(shelf: Option<&str>) -> String {
    let mut parts = vec![doc("synthesize.md").to_string()];
    match shelf {
        None => {
            parts.extend(SYNTHESIS_SECTIONS.iter().map(|path| section(path, doc(path))));
        }
        Some(url) => {
            parts.extend(
                SYNTHESIS_SECTIONS
                    .iter()
                    .filter(|path| INLINE_SECTIONS.contains(path))
                    .map(|path| section(path, doc(path))),
            );
            parts.push(shelf_instruction(url));
        }
    }
    parts.join("\n\n---\n\n")
}

fn section(label: &str, body: &str) -> String {
    format!("<!-- reference: {label} -->\n\n{body}")
}

/// The MCP-loading instruction replacing the inlined playbook when the
/// synthesis reference shelf is granted.
fn shelf_instruction(url: &str) -> String {
    use std::fmt::Write as _;

    let mut body = format!(
        "<!-- reference: synthesis shelf -->\n\n# Synthesis playbook references (MCP)\n\n\
         The rest of the synthesis playbook is served by the granted \
         `{SHELF_SERVER}` MCP server at {url}. Before authoring, load each document \
         below with its `read_doc` tool (`{{\"path\": \"<path>\"}}`); `list_docs` \
         lists every available path. These documents are authoritative playbook — \
         do not guess at their contents.\n"
    );
    for (path, summary) in SHELF_SUMMARIES {
        let _ = write!(body, "\n- `{path}` — {summary}");
    }
    body
}

// Keep (private embed-table kernel): the corpus contract is private to
// this crate, so `DOCS` / `SYNTHESIS_SECTIONS` have no public
// projection to test at any CLI or crate boundary.
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

    /// The inline minimum and the shelf summaries partition the
    /// playbook exactly: every section is inline xor shelf-listed.
    #[test]
    fn inline_shelf_partition() {
        for label in SYNTHESIS_SECTIONS {
            let inline = INLINE_SECTIONS.contains(label);
            let shelved = SHELF_SUMMARIES.iter().any(|(path, _)| path == label);
            assert!(inline ^ shelved, "`{label}` must be inline xor shelf-listed");
        }
        assert_eq!(INLINE_SECTIONS.len() + SHELF_SUMMARIES.len(), SYNTHESIS_SECTIONS.len());
    }

    /// Without a shelf grant, the prompt inlines each playbook section
    /// exactly once, in the fixed citation order.
    #[test]
    fn synthesis_assembly_order() {
        let assembled = synthesize_system(None);
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

    /// With a shelf grant, only the inline minimum is embedded; the
    /// remaining playbook is named in the MCP-loading instruction with
    /// the granted URL, and no shelf section body leaks inline.
    #[test]
    fn shelf_assembly() {
        let url = "http://127.0.0.1:9/mcp/engine/synthesis";
        let assembled = synthesize_system(Some(url));
        for label in INLINE_SECTIONS {
            let marker = format!("<!-- reference: {label} -->");
            assert_eq!(assembled.matches(&marker).count(), 1, "{label} stays inline");
        }
        assert!(assembled.contains(url), "the granted URL is named");
        assert!(assembled.contains(SHELF_SERVER), "the server name is named");
        for (path, _) in SHELF_SUMMARIES {
            assert!(assembled.contains(&format!("`{path}`")), "{path} is listed for loading");
            let marker = format!("<!-- reference: {path} -->");
            assert!(!assembled.contains(&marker), "{path} body must not inline");
        }
        assert!(
            assembled.len() < synthesize_system(None).len() / 2,
            "the shelf assembly is materially smaller than the inlined corpus"
        );
    }
}
