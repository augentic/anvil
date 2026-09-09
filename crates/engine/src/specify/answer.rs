//! Content answers
//!
//! Typed answers from the model when requested to synthesise extracted claims.
//!
//! An answer carries only what needs synthesis — paragraphs, scenarios, and
//! type references — keyed by the engine's subjects; every heading,
//! provenance line, note, and signature is the renderer's. The schema fixes
//! the shape and, tightened by each answer's `hints`, steers the provider
//! toward this run's subjects and sections; what no schema expresses — that
//! the subject set equals the row set, that a conflict row has no body, that
//! every type claim is referenced once — is checked here, and a miss is fed
//! back for repair.

use std::collections::{BTreeMap, BTreeSet};

use emery_source::claims::DOTTED_KEBAB_PATTERN;
use emery_source::types::Claim;
use omnia_guest::model::Findings;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use strum::VariantArray as _;

use crate::artifact::{SectionKind, Status, citations};
use crate::specify::extract::SourceEvidence;
use crate::specify::provenance::Provenance;
use crate::specify::synthesise::{Plan, Presence};

// A paragraph line may not open with anything the renderer owns.
const RESERVED: &[&str] = &["#", "ID:", "Sources:", "Status:", "Note:"];

/// The `spec.md` draft: preamble paragraphs and one entry per row.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery spec draft")]
pub struct SpecAnswer {
    /// Markdown paragraphs before the first requirement.
    pub preamble: Vec<String>,
    /// One entry per requirement row, keyed by subject; any order.
    pub requirements: Vec<Requirement>,
}

impl SpecAnswer {
    /// Steers the draft schema toward `rows`: exactly one entry per row,
    /// each subject one of the row subjects, at least one scenario. Hints
    /// for the provider; [`Self::check`] is the gate.
    pub fn hints(rows: &[Provenance]) -> impl FnOnce(&mut Value) {
        let subjects: Vec<&str> = rows.iter().map(Provenance::subject).collect();
        let subjects = json!(subjects);
        let count = json!(rows.len());
        move |schema| {
            schema["properties"]["requirements"]["minItems"] = count.clone();
            schema["properties"]["requirements"]["maxItems"] = count;
            schema["$defs"]["Requirement"]["properties"]["subject"]["enum"] = subjects;
            schema["$defs"]["Requirement"]["properties"]["scenarios"]["minItems"] = json!(1);
        }
    }

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
pub struct DesignAnswer {
    /// Markdown paragraphs before the first section.
    pub preamble: Vec<String>,
    /// One entry per rendered section; any order.
    pub sections: Vec<Section>,
}

impl DesignAnswer {
    /// Steers the draft schema toward `plan` and `sources`: at least every
    /// required section, section kinds the plan does not forbid, and type
    /// blocks naming this run's `type` claims. Hints for the provider;
    /// [`Self::check`] is the gate.
    pub fn hints(plan: &Plan, sources: &[SourceEvidence]) -> impl FnOnce(&mut Value) {
        let required = json!(plan.required().count());
        let kinds: Vec<&str> = SectionKind::VARIANTS
            .iter()
            .filter(|kind| plan.presence(**kind) != Presence::Forbidden)
            .map(AsRef::as_ref)
            .collect();
        let kinds = json!(kinds);
        let keys: Vec<&str> = sources
            .iter()
            .flat_map(|source| source.evidence.types())
            .filter_map(Claim::type_key)
            .collect();
        let keys = (!keys.is_empty()).then(|| json!(keys));
        move |schema| {
            schema["properties"]["sections"]["minItems"] = required;
            // The derived `kind` refers to the whole vocabulary; the run's
            // subset replaces the reference in place.
            let kind = &mut schema["$defs"]["Section"]["properties"]["kind"];
            if let Some(kind) = kind.as_object_mut() {
                kind.remove("$ref");
                kind.insert("type".to_string(), json!("string"));
                kind.insert("enum".to_string(), kinds);
            }
            if let Some(defs) = schema["$defs"].as_object_mut() {
                defs.remove("SectionKind");
            }
            if let Some(keys) = keys
                && let Some(block) = schema
                    .pointer_mut("/$defs/Block/oneOf")
                    .and_then(Value::as_array_mut)
                    .and_then(|variants| {
                        variants.iter_mut().find(|variant| variant["required"] == json!(["type"]))
                    })
            {
                block["properties"]["type"]["enum"] = keys;
            }
        }
    }

    /// Holds the draft to `plan` and `sources`: the section set, one
    /// reference per type claim under `domain-model`, bound citations, and no
    /// reserved marker.
    ///
    /// # Errors
    ///
    /// Returns every finding, for repair.
    pub fn check(&self, plan: &Plan, sources: &[SourceEvidence]) -> Result<(), Findings> {
        let mut findings = Vec::new();
        paragraphs(&self.preamble, "preamble", &mut findings);

        let bound: BTreeSet<&str> = sources.iter().map(|source| source.key.as_str()).collect();
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

        let keys: BTreeSet<&str> = sources
            .iter()
            .flat_map(|source| source.evidence.types())
            .filter_map(Claim::type_key)
            .collect();
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

// The hints edit the schema `schemars` derives; a derive change that moves
// a definition would silently turn a hint into a no-op, so each pointer is
// held here.
#[cfg(test)]
mod tests {
    use emery_source::types::{Authority, Claim, ClaimKind, Evidence};
    use omnia_guest::model::{Format, Question};
    use schemars::JsonSchema;
    use serde::de::DeserializeOwned;
    use serde_json::{Value, json};

    use super::{DesignAnswer, SpecAnswer};
    use crate::specify::extract::SourceEvidence;
    use crate::specify::provenance::floor;
    use crate::specify::synthesise::plan;

    fn claim(kind: ClaimKind, id: &str, extra: (&str, &str)) -> Claim {
        let mut extras = serde_json::Map::new();
        extras.insert(extra.0.to_string(), json!(extra.1));
        Claim {
            kind,
            id: Some(id.to_string()),
            path: None,
            synopsis: None,
            backing: None,
            extras,
        }
    }

    fn sources() -> Vec<SourceEvidence> {
        vec![SourceEvidence {
            key: "docs".to_string(),
            evidence: Evidence {
                authority: Authority::Documentation,
                claims: vec![
                    claim(ClaimKind::Requirement, "auth.login", ("statement", "Users log in.")),
                    claim(ClaimKind::Requirement, "auth.logout", ("statement", "Users log out.")),
                    claim(ClaimKind::Type, "auth.session", ("signature", "struct Session;")),
                ],
            },
        }]
    }

    fn schema<T: JsonSchema + DeserializeOwned + Send>(question: &Question<T>) -> Value {
        let Format::Schema(spec) = &question.request().format else {
            panic!("a question steers by schema");
        };
        serde_json::from_str(&spec.schema).expect("the steering schema is JSON")
    }

    #[test]
    fn spec_hints_land() {
        let sources = sources();
        let rows = floor(&sources);
        let question = Question::<SpecAnswer>::new("spec-draft").schema(SpecAnswer::hints(&rows));
        let schema = schema(&question);

        assert!(question.request().check, "the check is the gate");
        assert_eq!(schema["properties"]["requirements"]["minItems"], json!(2));
        assert_eq!(schema["properties"]["requirements"]["maxItems"], json!(2));
        let requirement = &schema["$defs"]["Requirement"]["properties"];
        assert_eq!(requirement["subject"]["enum"], json!(["auth.login", "auth.logout"]));
        assert_eq!(requirement["subject"]["type"], json!("string"), "the derive is intact");
        assert_eq!(requirement["scenarios"]["minItems"], json!(1));
    }

    #[test]
    fn design_hints_land() {
        let sources = sources();
        let plan = plan(&sources);
        let question = Question::<DesignAnswer>::new("design-draft")
            .schema(DesignAnswer::hints(&plan, &sources));
        let schema = schema(&question);

        // Overview and the type-informed domain model are required; the
        // uninformed spatial and API sections are forbidden.
        assert_eq!(schema["properties"]["sections"]["minItems"], json!(2));
        let kind = &schema["$defs"]["Section"]["properties"]["kind"];
        assert_eq!(
            kind["enum"],
            json!(["overview", "domain-model", "technical-logic", "observability"])
        );
        assert_eq!(kind["type"], json!("string"));
        assert!(kind.get("$ref").is_none() && schema["$defs"].get("SectionKind").is_none());
        let variants = schema["$defs"]["Block"]["oneOf"].as_array().expect("Block is a oneOf");
        let block = variants
            .iter()
            .find(|variant| variant["required"] == json!(["type"]))
            .expect("the type block variant");
        assert_eq!(block["properties"]["type"]["enum"], json!(["auth.session"]));
    }
}
