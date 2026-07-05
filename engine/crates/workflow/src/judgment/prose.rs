//! The workflow guest's embedded prompt corpus.
//!
//! Markdown stays the authoring source of truth: the prompt bodies and
//! the synthesis playbook references are embedded at compile time from
//! the repository tree (the dependency direction forbids importing the
//! adapters' `specify-prose-registry`, so the crate embeds directly).
//! The corpus is small (about 50 kilobytes), so it is pasted into the
//! system prompt rather than shelved behind an MCP route (decision D8).

/// System prompt body for the propose reconciliation leg.
pub const PROPOSE: &str = include_str!("prompts/propose.md");

/// System prompt body for the slice synthesis leg. The playbook
/// references below are appended as labeled sections.
pub const SYNTHESIZE: &str = include_str!("prompts/synthesize.md");

/// `plugins/spec/references/synthesis/substeps.md` — the synthesis
/// playbook's step ordering.
pub const SYNTHESIS_SUBSTEPS: &str =
    include_str!("../../../../../plugins/spec/references/synthesis/substeps.md");

/// `plugins/spec/references/synthesis/requirement-block.md` — the
/// requirement-block authoring contract.
pub const SYNTHESIS_REQUIREMENT_BLOCK: &str =
    include_str!("../../../../../plugins/spec/references/synthesis/requirement-block.md");

/// `plugins/spec/references/synthesis/authority.md` — authority
/// resolution order and override surface.
pub const SYNTHESIS_AUTHORITY: &str =
    include_str!("../../../../../plugins/spec/references/synthesis/authority.md");

/// `plugins/spec/references/synthesis/claim-reconciliation.md` —
/// claim-level agreement and reconciliation guidance.
pub const SYNTHESIS_CLAIM_RECONCILIATION: &str =
    include_str!("../../../../../plugins/spec/references/synthesis/claim-reconciliation.md");

/// `plugins/spec/references/synthesis/tags.md` — the `[unknown]` /
/// `[conflict]` / `[divergence]` tag vocabulary.
pub const SYNTHESIS_TAGS: &str =
    include_str!("../../../../../plugins/spec/references/synthesis/tags.md");

/// `plugins/spec/references/spec-format.md` — canonical heading
/// conventions for requirement blocks and scenario headings.
pub const SPEC_FORMAT: &str = include_str!("../../../../../plugins/spec/references/spec-format.md");

/// Assemble the synthesis system prompt: the authored prompt body plus
/// the playbook references as labeled sections, in citation order.
#[must_use]
pub fn synthesize_system() -> String {
    [
        SYNTHESIZE.to_string(),
        section("references/synthesis/substeps.md", SYNTHESIS_SUBSTEPS),
        section("references/synthesis/requirement-block.md", SYNTHESIS_REQUIREMENT_BLOCK),
        section("references/synthesis/authority.md", SYNTHESIS_AUTHORITY),
        section("references/synthesis/claim-reconciliation.md", SYNTHESIS_CLAIM_RECONCILIATION),
        section("references/synthesis/tags.md", SYNTHESIS_TAGS),
        section("references/spec-format.md", SPEC_FORMAT),
    ]
    .join("\n\n---\n\n")
}

fn section(label: &str, body: &str) -> String {
    format!("<!-- reference: {label} -->\n\n{body}")
}
