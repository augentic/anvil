//! Content drafts
//!
//! The two typed answers the model writes the documents' content as, and the
//! checks that hold each to the rows and the section plan. A draft carries
//! only what needs synthesis — paragraphs, scenarios, and type references —
//! keyed by the engine's subjects; every heading, provenance line, note, and
//! signature is the renderer's. What the schema cannot express — that the
//! subject set equals the row set, that a conflict row has no body, that
//! every type claim is referenced once — is checked here, and a miss is fed
//! back for repair.

use std::collections::{BTreeMap, BTreeSet};

use emery_source::claims::DOTTED_KEBAB_PATTERN;
use emery_source::types::Claim;
use schemars::JsonSchema;
use serde::Deserialize;

use super::extract::SourceSet;
use super::judgment::Findings;
use super::provenance::Provenance;
use super::synthesise::{Plan, Presence};
use crate::artifact::{SectionKind, Status, citations};

// A paragraph line may not open with anything the renderer owns.
const RESERVED: &[&str] = &["#", "ID:", "Sources:", "Status:", "Note:"];

/// The `spec.md` draft: preamble paragraphs and one entry per row.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery spec draft")]
pub struct SpecDraft {
    /// Markdown paragraphs before the first requirement.
    pub preamble: Vec<String>,
    /// One entry per requirement row, keyed by subject; any order.
    pub requirements: Vec<Requirement>,
}

impl SpecDraft {
    /// Holds the draft to `rows`: the subject set equals the row set, a
    /// scenario per requirement, body discipline per status, and no reserved
    /// marker.
    ///
    /// # Errors
    ///
    /// Returns every finding, for repair.
    pub fn check(&self, rows: &[Provenance]) -> Result<(), Findings> {
        let mut findings = Vec::new();
        paragraphs(&self.preamble, "preamble", &mut findings);

        let by_subject: BTreeMap<&str, &Provenance> =
            rows.iter().map(|row| (row.subject(), row)).collect();
        let mut seen = BTreeSet::new();
        for requirement in &self.requirements {
            let subject = requirement.subject.as_str();
            if !seen.insert(subject) {
                findings.push(format!("- `{subject}` is drafted more than once"));
                continue;
            }

            let Some(row) = by_subject.get(subject) else {
                findings.push(format!("- `{subject}` is not a requirement row"));
                continue;
            };

            let label = format!("`{subject}`");
            paragraphs(&requirement.body, &label, &mut findings);

            // A conflict row's statements are the renderer's notes, so its
            // body would assert what the operator has yet to reconcile.
            let conflict = row.status() == Status::Conflict;
            if conflict && !requirement.body.is_empty() {
                findings.push(format!("- {label} is a conflict row and carries a body"));
            } else if !conflict && requirement.body.is_empty() {
                findings.push(format!("- {label} has no body paragraph"));
            }

            if requirement.scenarios.is_empty() {
                findings.push(format!("- {label} has no scenario"));
            }

            for scenario in &requirement.scenarios {
                for (field, text) in
                    [("name", &scenario.name), ("when", &scenario.when), ("then", &scenario.then)]
                {
                    line(text, &format!("{label} scenario `{field}`"), &mut findings);
                }
                for given in &scenario.given {
                    line(given, &format!("{label} scenario `given`"), &mut findings);
                }
            }
        }

        for subject in by_subject.keys().filter(|subject| !seen.contains(*subject)) {
            findings.push(format!("- requirement row `{subject}` is not drafted"));
        }

        if findings.is_empty() { Ok(()) } else { Err(findings) }
    }
}

/// The drafted content of one requirement.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    /// The row's subject, exactly as listed.
    #[schemars(regex(pattern = DOTTED_KEBAB_PATTERN))]
    pub subject: String,
    /// Markdown paragraphs; empty for a conflict row.
    pub body: Vec<String>,
    /// At least one scenario.
    pub scenarios: Vec<Scenario>,
}

/// One acceptance scenario.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// The scenario heading name.
    pub name: String,
    /// Optional GIVEN context, one line each.
    #[serde(default)]
    pub given: Vec<String>,
    /// The WHEN trigger, one line.
    pub when: String,
    /// The THEN outcome, one line.
    pub then: String,
}

/// The `design.md` draft: preamble paragraphs and one entry per section.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery design draft")]
pub struct DesignDraft {
    /// Markdown paragraphs before the first section.
    pub preamble: Vec<String>,
    /// One entry per rendered section; any order.
    pub sections: Vec<Section>,
}

impl DesignDraft {
    /// Holds the draft to `plan` and `sets`: the section set, one reference
    /// per type claim under `domain-model`, bound citations, and no reserved
    /// marker.
    ///
    /// # Errors
    ///
    /// Returns every finding, for repair.
    pub fn check(&self, plan: &Plan, sets: &[SourceSet]) -> Result<(), Findings> {
        let mut findings = Vec::new();
        paragraphs(&self.preamble, "preamble", &mut findings);

        let bound: BTreeSet<&str> = sets.iter().map(|set| set.key.as_str()).collect();
        let mut seen = BTreeSet::new();
        let mut references: BTreeMap<&str, usize> = BTreeMap::new();
        for section in &self.sections {
            let kind = section.kind;
            let label = format!("`## {kind}`");
            if !seen.insert(kind) {
                findings.push(format!("- {label} is drafted more than once"));
            }
            if plan.presence(kind) == Presence::Forbidden {
                findings.push(format!("- {label} is present but no claim informs it"));
            }
            if section.blocks.is_empty() {
                findings.push(format!("- {label} has no block"));
            }

            for block in &section.blocks {
                match block {
                    Block::Text(text) => {
                        paragraph(text, &label, &mut findings);
                        for key in citations(text).filter(|key| !bound.contains(key)) {
                            findings.push(format!(
                                "- {label} cites source `{key}`, which is not bound"
                            ));
                        }
                    }
                    Block::Type(key) => {
                        if kind != SectionKind::DomainModel {
                            findings.push(format!(
                                "- {label} references type `{key}`; type blocks belong under \
                                 `## Domain model`"
                            ));
                        }
                        *references.entry(key.as_str()).or_default() += 1;
                    }
                }
            }
        }

        for kind in plan.required().filter(|kind| !seen.contains(kind)) {
            findings.push(format!("- `## {kind}` is required but absent"));
        }

        let keys: BTreeSet<&str> =
            sets.iter().flat_map(SourceSet::types).filter_map(Claim::type_key).collect();
        for key in &keys {
            match references.get(key).copied().unwrap_or_default() {
                1 => {}
                0 => findings.push(format!("- type `{key}` is never referenced")),
                n => findings.push(format!("- type `{key}` is referenced {n} times")),
            }
        }

        for key in references.keys().filter(|key| !keys.contains(*key)) {
            findings.push(format!("- type `{key}` is not a type claim"));
        }

        if findings.is_empty() { Ok(()) } else { Err(findings) }
    }
}

/// The drafted content of one section.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Section {
    /// The section, from the closed vocabulary.
    pub kind: SectionKind,
    /// At least one block, in reading order.
    pub blocks: Vec<Block>,
}

/// One design block: a paragraph, or a reference to a `type` claim whose
/// signature the renderer inserts verbatim.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Block {
    /// One Markdown paragraph; inline `(from <source>)` citations allowed.
    Text(String),
    /// The key of a `type` claim.
    Type(String),
}

fn paragraphs(texts: &[String], label: &str, findings: &mut Findings) {
    for text in texts {
        paragraph(text, label, findings);
    }
}

// A paragraph is non-blank and opens no line with a reserved marker.
fn paragraph(paragraph: &str, label: &str, findings: &mut Findings) {
    if paragraph.trim().is_empty() {
        findings.push(format!("- {label} has a blank paragraph"));
        return;
    }
    
    for text in paragraph.lines() {
        let text = text.trim_start();
        if let Some(marker) = RESERVED.iter().find(|marker| text.starts_with(**marker)) {
            findings.push(format!(
                "- {label}: a paragraph line opens with the reserved marker `{marker}`"
            ));
        }
    }
}

// A scenario field is one non-blank line.
fn line(text: &str, label: &str, findings: &mut Findings) {
    if text.trim().is_empty() {
        findings.push(format!("- {label} is blank"));
    } else if text.contains('\n') {
        findings.push(format!("- {label} spans more than one line"));
    }
}
