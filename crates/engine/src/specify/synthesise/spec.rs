//! The `spec.md` brief
//!
//! Asks the model for the content of `spec.md`: the preamble and, for every
//! requirement row, a body and its acceptance scenarios. The rows, their
//! headings, and their provenance lines are the engine's, so the schema
//! names the row subjects, the check holds the draft to exactly one entry
//! per row disciplined by the row's status, and the accepted pair renders
//! the canonical document.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};

use emery_source::claims::DOTTED_KEBAB_PATTERN;
use omnia_guest::model::Findings;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::artifact::{HEADING, ReqId, SCENARIO, Status};
use crate::specify::SourceEvidence;
use crate::specify::brief::{Brief, verdict};
use crate::specify::provenance::{Contributor, Provenance, normalise};
use crate::specify::synthesise::{document, line, paragraphs, render_claims};

/// The `spec.md` brief: the evidence and the requirement rows the draft is
/// held to.
pub struct SpecBrief<'a> {
    sources: &'a [SourceEvidence],
    rows: &'a [Provenance],
}

impl<'a> SpecBrief<'a> {
    /// Briefs `spec.md` over `sources` and their `rows`.
    #[must_use]
    pub const fn new(sources: &'a [SourceEvidence], rows: &'a [Provenance]) -> Self {
        Self { sources, rows }
    }
}

impl Brief for SpecBrief<'_> {
    type Answer = SpecAnswer;
    type Output = String;

    const NAME: &'static str = "spec-draft";
    // Prompt order is significant.
    const PROSE: &'static [&'static str] = &[
        "synthesis/synthesise.md",
        "synthesis/authority.md",
        "synthesis/claim-landing.md",
        "synthesis/requirement-block.md",
        "synthesis/spec-format.md",
        "synthesis/tags.md",
    ];

    // Exactly one entry per row, each subject one of the row subjects, at
    // least one scenario.
    fn hints(&self, schema: &mut Value) {
        let count = self.rows.len();
        schema["properties"]["requirements"]["minItems"] = json!(count);
        schema["properties"]["requirements"]["maxItems"] = json!(count);
        schema["$defs"]["Requirement"]["properties"]["subject"]["enum"] =
            json!(self.rows.iter().map(Provenance::subject).collect::<Vec<_>>());
        schema["$defs"]["Requirement"]["properties"]["scenarios"]["minItems"] = json!(1);
    }

    // The subject set equals the row set, a scenario per requirement, body
    // discipline per status, and no reserved marker.
    fn check(&self, answer: &SpecAnswer) -> Result<(), Findings> {
        let mut findings = Vec::new();
        paragraphs(&answer.preamble, "preamble", &mut findings);

        let by_subject: BTreeMap<&str, &Provenance> =
            self.rows.iter().map(|row| (row.subject(), row)).collect();
        let mut seen = BTreeSet::new();
        for requirement in &answer.requirements {
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
                    line(text, format_args!("{label} scenario `{field}`"), &mut findings);
                }
                for given in &scenario.given {
                    line(given, format_args!("{label} scenario `given`"), &mut findings);
                }
            }
        }

        for subject in by_subject.keys().filter(|subject| !seen.contains(*subject)) {
            findings.push(format!("- requirement row `{subject}` is not drafted"));
        }

        verdict(findings)
    }

    // Renders `spec.md`: the rows in order, each with its drafted content.
    fn conclude(self, answer: SpecAnswer) -> String {
        let rows = self.rows;
        let entries: BTreeMap<&str, &Requirement> =
            answer.requirements.iter().map(|entry| (entry.subject.as_str(), entry)).collect();

        let mut blocks: Vec<String> = vec!["# Specification".to_string()];
        blocks.extend(answer.preamble.iter().map(|paragraph| paragraph.trim().to_string()));

        for (index, row) in rows.iter().enumerate() {
            let drafted = entries.get(row.subject()).expect("the check held the draft to the rows");
            let tag = row.status().tag().map(|tag| format!(" [{tag}]")).unwrap_or_default();
            blocks.push(format!("{HEADING} {}{tag}", row.subject()));
            blocks.push(format!(
                "ID: {id}\nSources: [{sources}]\nStatus: {status}",
                id = ReqId::nth(index),
                sources = row.sources().collect::<Vec<_>>().join(", "),
                status = row.status(),
            ));

            if row.status() != Status::Conflict {
                blocks.extend(drafted.body.iter().map(|paragraph| paragraph.trim().to_string()));
            }
            if let Some(notes) = notes(row) {
                blocks.push(notes);
            }

            for scenario in &drafted.scenarios {
                blocks.push(format!("{SCENARIO} {}", scenario.name.trim()));
                let mut bullets = String::new();
                for given in &scenario.given {
                    let _ = writeln!(bullets, "- **GIVEN** {}", given.trim());
                }
                let _ = writeln!(bullets, "- **WHEN** {}", scenario.when.trim());
                let _ = write!(bullets, "- **THEN** {}", scenario.then.trim());
                blocks.push(bullets);
            }
        }

        document(&blocks)
    }
}

// The turn: every claim, then every row with its contributors in role.
impl fmt::Display for SpecBrief<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Draft `spec.md`.\n\n")?;
        render_claims(f, self.sources)?;

        f.write_str("\n## Requirement rows (draft one entry per subject)\n\n")?;
        for (index, row) in self.rows.iter().enumerate() {
            let sources = row.sources().collect::<Vec<_>>().join(", ");
            let coverage = if row.covered() { "evidenced" } else { "not evidenced" };
            writeln!(
                f,
                "- {id} `{subject}` — Status: {status} — Sources: [{sources}] — acceptance criteria {coverage}",
                id = ReqId::nth(index),
                subject = row.subject(),
                status = row.status(),
            )?;

            for (position, class) in row.classes().iter().enumerate() {
                let role = match (row.status(), position) {
                    (Status::Divergence, 0) => "winner",
                    (Status::Divergence, _) => "loser",
                    _ => "contributor",
                };

                for member in class {
                    writeln!(
                        f,
                        "  - {role}: {source} ({authority}, `{claim}`): {statement}",
                        source = member.source,
                        authority = member.authority,
                        claim = member.id,
                        statement = member.statement,
                    )?;
                }
            }
        }

        Ok(())
    }
}

/// The `spec.md` draft: preamble paragraphs and one entry per row. Only
/// what needs synthesis is asked for; every heading, provenance line, and
/// note is the renderer's.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery spec draft")]
pub struct SpecAnswer {
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

// The templated `Note:` lines: one per losing class (every class for a
// conflict, then the reconciliation line), then the acceptance gap.
fn notes(row: &Provenance) -> Option<String> {
    let mut lines = Vec::new();

    match row.status() {
        Status::Divergence => lines.extend(row.classes().iter().skip(1).map(|class| note(class))),
        Status::Conflict => {
            lines.extend(row.classes().iter().map(|class| note(class)));
            lines.push("Note: Operator reconciliation required.".to_string());
        }
        Status::Agreed | Status::Unknown => {}
    }

    if !row.covered() {
        lines.push("Note: acceptance criteria not evidenced.".to_string());
    }

    (!lines.is_empty()).then(|| lines.join("\n"))
}

// `Note: <sources> (<authority>, <id>): <statement>` for one class.
fn note(class: &[Contributor]) -> String {
    let sources = class.iter().map(|member| member.source.as_str()).collect::<Vec<_>>().join(", ");
    let lead = &class[0];

    format!(
        "Note: {sources} ({authority}, {id}): {statement}",
        authority = lead.authority,
        id = lead.id,
        statement = normalise(&lead.statement),
    )
}
