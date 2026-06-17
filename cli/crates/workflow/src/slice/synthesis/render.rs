//! `spec.md` provenance-line rendering.
//!
//! The render step is the third synthesis phase: given a projected
//! [`SliceModel`], it emits one `specs/<domain>/spec.md` per owning domain,
//! injecting the kernel-owned `ID:` / `Sources:` / `Status:` provenance
//! lines (and the inline status tag) under each requirement heading.
//!
//! **Parser-symmetry decision.** The RFC's §"Rendering" sketch shows an
//! `## <title>` h2 heading, but that example is illustrative. This
//! renderer instead follows the requirement-block shape that
//! [`specify_model::spec::provenance::parse_spec_md`] consumes —
//! `### Requirement: <title>` plus an inline `[unknown]` / `[conflict]`
//! / `[divergence]` tag suffix when the status is not `agreed`, then the
//! three metadata lines, a blank line, and the body. Render and parse
//! stay symmetric so the C9 `slice-spec-provenance-stale` check (which
//! parses the on-disk `spec.md` with that same parser) and
//! `specify slice validate` round-trip the output cleanly. The tag is
//! the status text itself (`divergence`, …), so the parser's
//! tag↔status coherence rule is satisfied by construction.
//!
//! The body is rendered purely from the model's behavioral prose
//! (`statement`, `scenarios`, `notes`) — the kernel fully owns the
//! provenance lines and the block structure is deterministic.

use std::collections::HashMap;
use std::fmt::Write as _;

use specify_model::spec::SCENARIO_HEADING;
use specify_model::spec::provenance::RequirementStatus;

use crate::slice::model::{ModelRequirement, SliceModel};

/// Domain key used to group requirements that carry no `domain` field.
const DEFAULT_DOMAIN: &str = "default";

const HEADING_PREFIX: &str = "### Requirement:";

/// One rendered `specs/<domain>/spec.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedSpec {
    /// Owning domain (the `specs/<domain>/spec.md` directory segment).
    pub domain: String,
    /// Full rendered Markdown content, provenance lines injected.
    pub content: String,
}

/// The canonical `(ID, Sources, Status)` provenance triplet a single
/// requirement renders, paired with its owning domain.
///
/// [`expected_provenance_lines`] returns one per requirement; the C9
/// staleness check (`slice-spec-provenance-stale`) parses the on-disk
/// `spec.md` with [`specify_model::spec::provenance::parse_spec_md`] and
/// compares each parsed requirement's `id` / `sources` / `status`
/// against this set. The field types mirror the parser's
/// [`specify_model::spec::provenance::Requirement`] so the comparison is
/// direct: `id` is a plain `String` (empty when unprojected), `status`
/// is the optional enum the parser yields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedRequirement {
    /// Owning domain.
    pub domain: String,
    /// Projected `REQ-NNN` id (empty when the model carried none).
    pub id: String,
    /// Rendered source list, highest authority first.
    pub sources: Vec<String>,
    /// Projected status, or `None` when the model carried none.
    pub status: Option<RequirementStatus>,
}

/// Render every `specs/<domain>/spec.md` from a projected [`SliceModel`].
///
/// Requirements are grouped by `domain` (declaration order of first
/// appearance), and within a domain render in declaration order. Each
/// requirement becomes one `### Requirement:` block carrying the
/// injected provenance lines followed by its behavioral body.
#[must_use]
pub fn render_spec_files(model: &SliceModel) -> Vec<RenderedSpec> {
    let mut order: Vec<String> = Vec::new();
    let mut blocks: HashMap<String, Vec<String>> = HashMap::new();
    for req in &model.requirements {
        let domain = domain_of(req);
        if !blocks.contains_key(&domain) {
            order.push(domain.clone());
        }
        blocks.entry(domain).or_default().push(render_block(req));
    }
    order
        .into_iter()
        .map(|domain| {
            let domain_blocks = blocks.remove(&domain).unwrap_or_default();
            let mut content = domain_blocks.join("\n\n");
            content.push('\n');
            RenderedSpec { domain, content }
        })
        .collect()
}

/// The canonical provenance triplet per requirement, in declaration
/// order, for the C9 staleness comparison against on-disk `spec.md`.
#[must_use]
pub fn expected_provenance_lines(model: &SliceModel) -> Vec<ExpectedRequirement> {
    model
        .requirements
        .iter()
        .map(|req| ExpectedRequirement {
            domain: domain_of(req),
            id: req.id.clone().unwrap_or_default(),
            sources: req.sources.clone(),
            status: req.status,
        })
        .collect()
}

fn domain_of(req: &ModelRequirement) -> String {
    req.domain.clone().unwrap_or_else(|| DEFAULT_DOMAIN.to_string())
}

fn render_block(req: &ModelRequirement) -> String {
    let mut out = String::new();
    out.push_str(HEADING_PREFIX);
    out.push(' ');
    out.push_str(&req.title);
    if let Some(status) = req.status
        && status != RequirementStatus::Agreed
    {
        let _ = write!(out, " [{status}]");
    }
    out.push('\n');
    let _ = writeln!(out, "ID: {}", req.id.as_deref().unwrap_or_default());
    // Contract: an evidence-less (`unknown`) requirement renders the
    // literal `Sources: []`, never a bare line with trailing space.
    if req.sources.is_empty() {
        out.push_str("Sources: []\n");
    } else {
        let _ = writeln!(out, "Sources: {}", req.sources.join(", "));
    }
    if let Some(status) = req.status {
        let _ = writeln!(out, "Status: {status}");
    }
    out.push('\n');
    out.push_str(&render_body(req));
    out
}

fn render_body(req: &ModelRequirement) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !req.statement.is_empty() {
        parts.push(req.statement.clone());
    }
    // Each scenario renders as a `#### Scenario:` H4 heading so the spec
    // parser (and the `specs.requirements-have-scenarios` rule) recognises
    // it. A bare-name entry yields a heading-only scenario; a multi-line
    // entry keeps its WHEN/THEN body under the heading.
    for scenario in &req.scenarios {
        parts.push(format!("{SCENARIO_HEADING} {scenario}"));
    }
    if let Some(notes) = req.notes.as_deref().filter(|n| !n.is_empty()) {
        parts.push(notes.to_string());
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests;
