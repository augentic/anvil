//! Canonical rendering
//!
//! Projects the rows and a validated draft into `spec.md`, and the plan and a
//! validated draft into `design.md`. The renderer owns every heading,
//! provenance line, tag, note, and signature, so the stored bytes are a
//! function of the rows and the draft alone: the same inputs render the same
//! document, and the re-mine diff compares engine formatting, never the
//! model's.
//!
//! The output is part of the artifact contract — a changed byte re-ids every
//! revision — and is frozen by the root scenario fixtures.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use strum::VariantArray as _;

use crate::artifact::{HEADING, ReqId, SCENARIO, SectionKind, Status};
use crate::specify::answer::{Block, DesignAnswer, Requirement, SpecAnswer};
use crate::specify::extract::SourceEvidence;
use crate::specify::provenance::{Contributor, Provenance, normalise};

/// Renders `spec.md` from `rows` and their drafted content.
#[must_use]
pub fn spec(rows: &[Provenance], draft: &SpecAnswer) -> String {
    let entries: BTreeMap<&str, &Requirement> =
        draft.requirements.iter().map(|entry| (entry.subject.as_str(), entry)).collect();

    let mut blocks: Vec<String> = vec!["# Specification".to_string()];
    blocks.extend(draft.preamble.iter().map(|paragraph| paragraph.trim().to_string()));

    for (index, row) in rows.iter().enumerate() {
        let drafted = entries.get(row.subject()).expect("the validated draft carries every row");
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

/// Renders `design.md` from the drafted sections and the type claims of
/// `sources`, sections in vocabulary order.
#[must_use]
pub fn design(sources: &[SourceEvidence], draft: &DesignAnswer) -> String {
    let signatures: BTreeMap<&str, &str> = sources
        .iter()
        .flat_map(|source| source.evidence.types())
        .filter_map(|claim| Some((claim.type_key()?, claim.signature()?)))
        .collect();

    let mut blocks: Vec<String> = vec!["# Design".to_string()];
    blocks.extend(draft.preamble.iter().map(|paragraph| paragraph.trim().to_string()));

    for &kind in SectionKind::VARIANTS {
        let Some(section) = draft.sections.iter().find(|section| section.kind == kind) else {
            continue;
        };

        blocks.push(format!("## {kind}"));
        for block in &section.blocks {
            match block {
                Block::Text(text) => blocks.push(text.trim().to_string()),
                Block::Type(key) => {
                    let signature = signatures
                        .get(key.as_str())
                        .expect("the validated draft references type claims alone");
                    blocks.push(format!("```\n{}\n```", signature.trim_end()));
                }
            }
        }
    }

    document(&blocks)
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

// One blank line between blocks, no trailing spaces, a trailing newline.
fn document(blocks: &[String]) -> String {
    let mut text = blocks
        .iter()
        .map(|block| block.lines().map(str::trim_end).collect::<Vec<_>>().join("\n"))
        .collect::<Vec<_>>()
        .join("\n\n");
    text.push('\n');
    text
}

// Keep (entry-point-unreachable): the renderer and the reader are two
// halves of one grammar, and only a round trip proves they agree.
#[cfg(test)]
mod tests {
    use emery_source::types::{Authority, Claim, ClaimKind, Evidence};
    use serde_json::json;

    use crate::artifact::{Design, SectionKind, Spec, Status};
    use crate::specify::answer::{DesignAnswer, SpecAnswer};
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

    fn source(key: &str, authority: Authority, claims: Vec<Claim>) -> SourceEvidence {
        let evidence = Evidence { authority, claims };
        evidence.validate().expect("valid claims");
        SourceEvidence {
            key: key.to_string(),
            evidence,
        }
    }

    // Every heading, provenance line, tag, note, and signature the renderer
    // writes reads back as the rows and sections it was rendered from.
    #[test]
    fn rendered_documents_read_back() {
        let sources = [source(
            "docs",
            Authority::Documentation,
            vec![
                claim(ClaimKind::Requirement, "login.flow", ("statement", "Magic link.")),
                claim(ClaimKind::Requirement, "login.flow", ("statement", "Passkey.")),
                claim(ClaimKind::Requirement, "session.timeout", ("statement", "30 minutes.")),
                claim(ClaimKind::Criterion, "session.timeout.idle", ("criterion", "Idle 30.")),
                claim(ClaimKind::Type, "session.type", ("signature", "type Session = {}")),
            ],
        )];
        let rows = floor(&sources);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].status(), Status::Conflict, "two docs statements tie");
        assert_eq!(rows[1].status(), Status::Agreed, "covered by its criterion");

        let spec_draft: SpecAnswer = serde_json::from_value(json!({
            "preamble": ["Two requirements."],
            "requirements": [
                {"subject": "session.timeout", "body": ["Sessions expire."],
                 "scenarios": [{"name": "Idle", "given": ["a session"], "when": "idle", "then": "expired"}]},
                {"subject": "login.flow", "body": [],
                 "scenarios": [{"name": "Login", "when": "login", "then": "decide"}]}
            ]
        }))
        .expect("draft shape");
        spec_draft.check(&rows).expect("draft fits the rows");
        let spec = crate::specify::render::spec(&rows, &spec_draft);
        let read: Spec = spec.parse().expect("the rendering is canonical");
        let subjects: Vec<&str> = read.requirements.iter().map(|r| r.subject.as_str()).collect();
        assert_eq!(subjects, ["login.flow", "session.timeout"], "row order");
        assert_eq!(read.requirements[0].status, Status::Conflict);
        assert_eq!(read.requirements[0].sources, ["docs", "docs"]);
        assert_eq!(read.requirements[1].status, Status::Agreed);
        assert!(spec.contains("Note: Operator reconciliation required."), "{spec}");
        assert!(
            spec.contains("- **GIVEN** a session\n- **WHEN** idle\n- **THEN** expired"),
            "{spec}"
        );

        let design_draft: DesignAnswer = serde_json::from_value(json!({
            "preamble": [],
            "sections": [
                {"kind": "domain-model", "blocks": [{"type": "session.type"}]},
                {"kind": "overview", "blocks": [{"text": "Sessions (from docs)."}]}
            ]
        }))
        .expect("draft shape");
        design_draft.check(&plan(&sources), &sources).expect("draft fits the plan");
        let design = crate::specify::render::design(&sources, &design_draft);
        let read: Design = design.parse().expect("the rendering is canonical");
        let kinds: Vec<SectionKind> = read.sections.iter().map(|s| s.kind).collect();
        assert_eq!(kinds, [SectionKind::Overview, SectionKind::DomainModel], "vocabulary order");
        assert!(design.contains("```\ntype Session = {}\n```"), "{design}");
    }
}
