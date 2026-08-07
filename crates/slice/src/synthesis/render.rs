//! Renders projected slice models as domain `spec.md` files.
//!
//! Output deliberately matches
//! [`artifacts::spec::provenance::parse_spec_md`]: requirement headings,
//! inline status tags, and provenance lines round-trip through validation.

use std::collections::HashMap;
use std::fmt::Write as _;

use artifacts::spec::SCENARIO_HEADING;
use artifacts::spec::provenance::RequirementStatus;

use crate::model::{ModelRequirement, SliceModel};
use crate::synthesis::baseline::{BaselineIndex, DomainKind};

/// Domain used when a requirement has no explicit owner.
const DEFAULT_DOMAIN: &str = "default";

const HEADING_PREFIX: &str = "### Requirement:";
const ADDED_SECTION: &str = "## ADDED Requirements";
const MODIFIED_SECTION: &str = "## MODIFIED Requirements";

/// One rendered `specs/<domain>/spec.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedSpec {
    /// Owning domain (the `specs/<domain>/spec.md` directory segment).
    pub domain: String,
    /// Full rendered Markdown content.
    pub content: String,
}

/// Expected provenance for one rendered requirement.
///
/// Its field types mirror the parser output so staleness checks compare
/// values directly.
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

/// Render one `specs/<domain>/spec.md` body per domain.
#[must_use]
pub fn render_spec_files(model: &SliceModel, baseline_index: &BaselineIndex) -> Vec<RenderedSpec> {
    let mut order: Vec<String> = Vec::new();
    let mut by_domain: HashMap<String, Vec<&ModelRequirement>> = HashMap::new();
    for req in &model.requirements {
        let domain = domain_of(req);
        if !by_domain.contains_key(&domain) {
            order.push(domain.clone());
        }
        by_domain.entry(domain).or_default().push(req);
    }
    order
        .into_iter()
        .map(|domain| {
            let reqs = by_domain.remove(&domain).unwrap_or_default();
            let content = if baseline_index.domain_kind(&domain) == DomainKind::Modified {
                render_modified_domain(&reqs)
            } else {
                render_flat_domain(&reqs)
            };
            RenderedSpec { domain, content }
        })
        .collect()
}

fn render_flat_domain(reqs: &[&ModelRequirement]) -> String {
    let mut content = reqs.iter().map(|req| render_block(req)).collect::<Vec<_>>().join("\n\n");
    content.push('\n');
    content
}

fn render_modified_domain(reqs: &[&ModelRequirement]) -> String {
    let mut added: Vec<String> = Vec::new();
    let mut modified: Vec<String> = Vec::new();
    for req in reqs {
        // Slice-local ids never match baseline numbers; `baseline-id`
        // marks a MODIFIED row until wave commit remaps identity.
        if req.baseline_id.is_some() {
            modified.push(render_block(req));
        } else {
            added.push(render_block(req));
        }
    }

    let mut sections: Vec<String> = Vec::new();
    if !added.is_empty() {
        let mut section = String::from(ADDED_SECTION);
        section.push_str("\n\n");
        section.push_str(&added.join("\n\n"));
        sections.push(section);
    }
    if !modified.is_empty() {
        let mut section = String::from(MODIFIED_SECTION);
        section.push_str("\n\n");
        section.push_str(&modified.join("\n\n"));
        sections.push(section);
    }
    let mut content = sections.join("\n\n");
    content.push('\n');
    content
}

/// Return expected provenance in requirement declaration order.
#[must_use]
pub fn provenance_lines(model: &SliceModel) -> Vec<ExpectedRequirement> {
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
    // The parser recognizes `Sources: []` as an explicitly empty list.
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
    // Keep scenario headings symmetric with the spec parser.
    for scenario in &req.scenarios {
        parts.push(format!("{SCENARIO_HEADING} {scenario}"));
    }
    if let Some(notes) = req.notes.as_deref().filter(|n| !n.is_empty()) {
        parts.push(notes.to_string());
    }
    parts.join("\n\n")
}
