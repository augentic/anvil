//! The workflow guest's embedded prompt corpus.
//!
//! Markdown stays the authoring source of truth: the prompt bodies and
//! the synthesis playbook references are inlined and link-checked by
//! this crate's `build.rs` (a dangling relative reference fails the
//! build) and embedded from
//! `OUT_DIR/prose/`. The corpus is small (about 50 kilobytes), so it is
//! pasted into the system prompt rather than shelved behind an MCP
//! route.

/// System prompt body for the propose reconciliation leg.
pub const PROPOSE: &str = include_str!(concat!(env!("OUT_DIR"), "/prose/propose.md"));

/// System prompt body for the slice synthesis leg. The playbook
/// references below are appended as labeled sections.
pub const SYNTHESIZE: &str = include_str!(concat!(env!("OUT_DIR"), "/prose/synthesize.md"));

/// `prompts/synthesis/substeps.md` — the synthesis playbook's step
/// ordering.
pub const SYNTHESIS_SUBSTEPS: &str = include_str!(concat!(env!("OUT_DIR"), "/prose/substeps.md"));

/// `prompts/synthesis/requirement-block.md` — the requirement-block
/// authoring contract.
pub const SYNTHESIS_REQUIREMENT_BLOCK: &str =
    include_str!(concat!(env!("OUT_DIR"), "/prose/requirement-block.md"));

/// `prompts/synthesis/authority.md` — authority resolution order and
/// override surface.
pub const SYNTHESIS_AUTHORITY: &str = include_str!(concat!(env!("OUT_DIR"), "/prose/authority.md"));

/// `prompts/synthesis/claim-reconciliation.md` — claim-level agreement
/// and reconciliation guidance.
pub const SYNTHESIS_CLAIM_RECONCILIATION: &str =
    include_str!(concat!(env!("OUT_DIR"), "/prose/claim-reconciliation.md"));

/// `prompts/synthesis/tags.md` — the `[unknown]` / `[conflict]` /
/// `[divergence]` tag vocabulary.
pub const SYNTHESIS_TAGS: &str = include_str!(concat!(env!("OUT_DIR"), "/prose/tags.md"));

/// `prompts/synthesis/decisions.md` — the optional Decision Record
/// authoring contract (durable bar, supersession, traceability).
pub const SYNTHESIS_DECISIONS: &str = include_str!(concat!(env!("OUT_DIR"), "/prose/decisions.md"));

/// `prompts/synthesis/spec-format.md` — canonical heading conventions
/// for requirement blocks and scenario headings.
pub const SPEC_FORMAT: &str = include_str!(concat!(env!("OUT_DIR"), "/prose/spec-format.md"));

/// Assemble the synthesis system prompt: the authored prompt body plus
/// the playbook references as labeled sections, in citation order.
#[must_use]
pub fn synthesize_system() -> String {
    [
        SYNTHESIZE.to_string(),
        section("synthesis/substeps.md", SYNTHESIS_SUBSTEPS),
        section("synthesis/requirement-block.md", SYNTHESIS_REQUIREMENT_BLOCK),
        section("synthesis/authority.md", SYNTHESIS_AUTHORITY),
        section("synthesis/claim-reconciliation.md", SYNTHESIS_CLAIM_RECONCILIATION),
        section("synthesis/tags.md", SYNTHESIS_TAGS),
        section("synthesis/decisions.md", SYNTHESIS_DECISIONS),
        section("synthesis/spec-format.md", SPEC_FORMAT),
    ]
    .join("\n\n---\n\n")
}

fn section(label: &str, body: &str) -> String {
    format!("<!-- reference: {label} -->\n\n{body}")
}

// The corpus contract is private to this crate (the module is
// `pub(crate)`), so it is checked here against the private kernel
// rather than widened for an integration suite. Link resolution is
// enforced twice elsewhere: at embed time by `build.rs` and by the
// framework link gate over `crates/workflow/prompts/`.
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;

    /// Every runtime constant, in declaration order.
    const CONSTANTS: &[&str] = &[
        PROPOSE,
        SYNTHESIZE,
        SYNTHESIS_SUBSTEPS,
        SYNTHESIS_REQUIREMENT_BLOCK,
        SYNTHESIS_AUTHORITY,
        SYNTHESIS_CLAIM_RECONCILIATION,
        SYNTHESIS_TAGS,
        SYNTHESIS_DECISIONS,
        SPEC_FORMAT,
    ];

    fn prompt_files() -> Vec<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("prompts");
        let mut files = Vec::new();
        collect(&root, &mut files);
        files.sort();
        files
    }

    fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("read prompts dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                collect(&path, out);
            } else if path.extension().is_some_and(|e| e == "md") {
                out.push(path);
            }
        }
    }

    /// Every on-disk prompt file is embedded as exactly one runtime
    /// constant, and no constant lacks an on-disk source.
    #[test]
    fn corpus_is_bijective() {
        let files = prompt_files();
        assert_eq!(files.len(), CONSTANTS.len(), "prompt file count vs runtime constants");
        let bodies: BTreeSet<String> =
            files.iter().map(|path| fs::read_to_string(path).expect("read prompt file")).collect();
        assert_eq!(bodies.len(), files.len(), "prompt bodies are distinct");
        for (constant, _) in CONSTANTS.iter().zip(prompt_files()) {
            assert!(
                bodies.contains(*constant),
                "an embedded constant has no matching prompts/ source file"
            );
        }
    }

    /// The synthesis system prompt assembles each playbook section
    /// exactly once, in the fixed citation order.
    #[test]
    fn synthesis_assembly_order() {
        let assembled = synthesize_system();
        let labels = [
            "synthesis/substeps.md",
            "synthesis/requirement-block.md",
            "synthesis/authority.md",
            "synthesis/claim-reconciliation.md",
            "synthesis/tags.md",
            "synthesis/decisions.md",
            "synthesis/spec-format.md",
        ];
        let mut cursor = 0;
        for label in labels {
            let marker = format!("<!-- reference: {label} -->");
            assert_eq!(assembled.matches(&marker).count(), 1, "{label} assembles exactly once");
            let position = assembled.find(&marker).expect("marker present");
            assert!(position > cursor, "{label} out of citation order");
            cursor = position;
        }
        assert!(assembled.starts_with(SYNTHESIZE), "the authored prompt body leads the assembly");
    }
}
