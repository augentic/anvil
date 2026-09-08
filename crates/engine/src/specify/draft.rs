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
use emery_source::types::{Claim, ClaimKind};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::extract::SourceSet;
use super::judgment::{self, Findings};
use super::provenance::Row;
use super::synthesise::{Plan, Presence};
use crate::artifact::{SectionKind, Status, citations};

/// The `spec.md` draft: preamble paragraphs and one entry per row.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpecDraft {
    /// Markdown paragraphs before the first requirement.
    pub preamble: Vec<String>,
    /// One entry per requirement row, keyed by subject; any order.
    pub requirements: Vec<Requirement>,
}

/// The drafted content of one requirement.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    /// The row's subject, exactly as listed.
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
pub struct DesignDraft {
    /// Markdown paragraphs before the first section.
    pub preamble: Vec<String>,
    /// One entry per rendered section; any order.
    pub sections: Vec<Section>,
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
#[serde(untagged, deny_unknown_fields)]
pub enum Block {
    /// One Markdown paragraph; inline `(from <source>)` citations allowed.
    Text {
        /// The paragraph.
        text: String,
    },
    /// The key of a `type` claim.
    Type {
        /// The claim's id, or its path when it has no id.
        r#type: String,
    },
}

/// The `SpecDraft` answer schema.
#[must_use]
pub fn spec_schema() -> String {
    judgment::schema::<SpecDraft>("Emery spec draft", |value| {
        let subject = value
            .pointer_mut("/$defs/Requirement/properties/subject")
            .and_then(Value::as_object_mut)
            .expect("spec draft schema carries Requirement.subject");
        subject.insert("pattern".to_string(), json!(DOTTED_KEBAB_PATTERN));
    })
}

/// The `DesignDraft` answer schema.
#[must_use]
pub fn design_schema() -> String {
    judgment::schema::<DesignDraft>("Emery design draft", |_| {})
}

// A paragraph line may not open with anything the renderer owns.
const RESERVED: &[&str] = &["#", "ID:", "Sources:", "Status:", "Note:"];

/// Holds `draft` to `rows`: the subject set equals the row set, a scenario
/// per requirement, body discipline per status, and no reserved marker.
///
/// # Errors
///
/// Returns every finding, for repair.
pub fn check_spec(draft: &SpecDraft, rows: &[Row]) -> Result<(), Findings> {
    let mut findings = Vec::new();
    paragraphs(&draft.preamble, "preamble", &mut findings);

    let by_subject: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.subject(), row)).collect();
    let mut seen = BTreeSet::new();
    for requirement in &draft.requirements {
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
        match (row.status(), requirement.body.is_empty()) {
            (Status::Conflict, false) => {
                findings.push(format!("- {label} is a conflict row and carries a body"));
            }
            (Status::Conflict, true) => {}
            (_, true) => findings.push(format!("- {label} has no body paragraph")),
            _ => {}
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

/// Holds `draft` to `plan` and `sets`: the section set, one reference per
/// type claim under `domain-model`, bound citations, and no reserved marker.
///
/// # Errors
///
/// Returns every finding, for repair.
pub fn check_design(draft: &DesignDraft, plan: &Plan, sets: &[SourceSet]) -> Result<(), Findings> {
    let mut findings = Vec::new();
    paragraphs(&draft.preamble, "preamble", &mut findings);

    let mut seen = BTreeSet::new();
    for section in &draft.sections {
        let kind = section.kind;
        if !seen.insert(kind) {
            findings.push(format!("- `## {kind}` is drafted more than once"));
        }
        if plan.presence(kind) == Presence::Forbidden {
            findings.push(format!("- `## {kind}` is present but no claim informs it"));
        }
        if section.blocks.is_empty() {
            findings.push(format!("- `## {kind}` has no block"));
        }
    }
    for kind in plan.required().filter(|kind| !seen.contains(kind)) {
        findings.push(format!("- `## {kind}` is required but absent"));
    }

    let mut references: BTreeMap<&str, usize> = BTreeMap::new();
    for section in &draft.sections {
        let kind = section.kind;
        for block in &section.blocks {
            match block {
                Block::Text { text } => {
                    line_rule(text, &format!("`## {kind}`"), &mut findings);
                    for key in citations(text).filter(|key| !sets.iter().any(|set| set.key == *key))
                    {
                        findings.push(format!(
                            "- `## {kind}` cites source `{key}`, which is not bound"
                        ));
                    }
                }
                Block::Type { r#type: key } => {
                    if kind != SectionKind::DomainModel {
                        findings.push(format!("- `## {kind}` references type `{key}`; type blocks belong under `## Domain model`"));
                    }
                    *references.entry(key.as_str()).or_default() += 1;
                }
            }
        }
    }
    let mut keys = BTreeSet::new();
    for claim in types(sets) {
        let Some(key) = type_key(claim) else { continue };
        keys.insert(key);
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

/// Every `type` claim in `sets`.
pub fn types(sets: &[SourceSet]) -> impl Iterator<Item = &Claim> {
    sets.iter().flat_map(|set| &set.claims).filter(|claim| claim.kind == ClaimKind::Type)
}

/// The key a draft references a type claim by: its id, else its path.
#[must_use]
pub fn type_key(claim: &Claim) -> Option<&str> {
    claim.id.as_deref().or(claim.path.as_deref())
}

/// The `signature` extra of a type claim, when it is a string.
#[must_use]
pub fn signature(claim: &Claim) -> Option<&str> {
    match claim.extras.get("signature") {
        Some(Value::String(signature)) => Some(signature),
        _ => None,
    }
}

// Every paragraph is non-blank and opens no line with a reserved marker.
fn paragraphs(paragraphs: &[String], label: &str, findings: &mut Findings) {
    for paragraph in paragraphs {
        line_rule(paragraph, label, findings);
    }
}

fn line_rule(paragraph: &str, label: &str, findings: &mut Findings) {
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
