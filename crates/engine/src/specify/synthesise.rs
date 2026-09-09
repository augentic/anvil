//! Synthesis
//!
//! Turns the requirement rows and the extracted claims into the two
//! specification documents. The model is asked two typed questions — the
//! content of `spec.md`, keyed by row subject, then the content of
//! `design.md`, keyed by planned section — each put as a brief whose check
//! holds every candidate to the rows, the section plan, and the evidence.
//! The accepted answer, bound to its brief, then renders the canonical
//! document: every heading, provenance line, tag, note, and signature is
//! the engine's, so the stored bytes are a function of the facts and the
//! draft alone, and a changed byte re-ids every revision.
//!
//! Nothing the engine already knows is asked of the model: it never writes a
//! heading, an id, a `Sources:` list, a status, a note, or a type signature,
//! so it cannot drop, reorder, or quietly rewrite a requirement, invent or
//! omit a section, cite an unbound source, or paraphrase a signature.

mod design;
mod spec;

use std::fmt;

use omnia_guest::model::Findings;
use omnia_guest::{Error, Model};
use serde_json::Value;

use self::design::DesignBrief;
use self::spec::SpecBrief;
use crate::specify::SourceEvidence;
use crate::specify::brief::Brief as _;
use crate::specify::provenance::Provenance;
use crate::store::Revision;

// A paragraph line may not open with anything the renderer owns.
const RESERVED: &[&str] = &["#", "ID:", "Sources:", "Status:", "Note:"];

/// Drafts `spec.md` over `rows`, then `design.md` over the rendered spec,
/// and renders both canonically.
pub async fn synthesise<M: Model>(
    model: &M, sources: &[SourceEvidence], rows: &[Provenance],
) -> Result<Revision, Error> {
    let spec = SpecBrief::new(sources, rows).judge(model).await?;
    let design = DesignBrief::new(&spec, sources).judge(model).await?;

    Ok(Revision { spec, design })
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

// Every claim of every source, for a brief's turn.
fn render_claims(f: &mut fmt::Formatter<'_>, sources: &[SourceEvidence]) -> fmt::Result {
    f.write_str("## Claims\n")?;

    for source in sources {
        write!(
            f,
            "\n### source `{key}` ({authority})\n\n",
            key = source.key,
            authority = source.evidence.authority
        )?;

        for claim in &source.evidence.claims {
            let id = claim.id.as_deref().unwrap_or("-");
            let synopsis = claim.synopsis.as_deref().unwrap_or("");
            writeln!(
                f,
                "- {kind} `{id}` — {synopsis} — {extras}",
                kind = claim.kind,
                extras = Value::Object(claim.extras.clone()),
            )?;
        }
    }

    Ok(())
}

fn paragraphs(texts: &[String], label: impl fmt::Display, findings: &mut Findings) {
    for text in texts {
        paragraph(text, &label, findings);
    }
}

// A paragraph is non-blank and opens no line with a reserved marker.
fn paragraph(text: &str, label: impl fmt::Display, findings: &mut Findings) {
    if text.trim().is_empty() {
        findings.push(format!("- {label} has a blank paragraph"));
        return;
    }

    for line in text.lines() {
        let line = line.trim_start();
        if let Some(marker) = RESERVED.iter().copied().find(|marker| line.starts_with(marker)) {
            findings.push(format!(
                "- {label}: a paragraph line opens with the reserved marker `{marker}`"
            ));
        }
    }
}

// A scenario field is one non-blank line.
fn line(text: &str, label: impl fmt::Display, findings: &mut Findings) {
    if text.trim().is_empty() {
        findings.push(format!("- {label} is blank"));
    } else if text.contains('\n') {
        findings.push(format!("- {label} spans more than one line"));
    }
}

// Keep (entry-point-unreachable): the renderer and the reader are two
// halves of one grammar, and only a round trip proves they agree.
#[cfg(test)]
mod tests {
    use emery_source::types::ClaimKind;
    use serde_json::json;

    use super::design::{DesignAnswer, DesignBrief};
    use super::spec::{SpecAnswer, SpecBrief};
    use crate::artifact::{Design, SectionKind, Spec, Status};
    use crate::specify::brief::Brief;
    use crate::specify::fixture::{claim, source};
    use crate::specify::provenance::floor;

    // Every heading, provenance line, tag, note, and signature the renderer
    // writes reads back as the rows and sections it was rendered from.
    #[test]
    fn read_rendered() {
        let sources = [source(
            "docs",
            vec![
                claim(ClaimKind::Requirement, "login.flow", ("statement", "Magic link.")),
                claim(ClaimKind::Requirement, "login.flow", ("statement", "Passkey.")),
                claim(ClaimKind::Requirement, "session.timeout", ("statement", "30 minutes.")),
                claim(ClaimKind::Criterion, "session.timeout.idle", ("criterion", "Idle 30.")),
                claim(ClaimKind::Type, "session.type", ("signature", "type Session = {}")),
            ],
        )];
        assert!(sources[0].evidence.findings().is_empty(), "valid claims");
        let rows = floor(&sources);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].status(), Status::Conflict, "two docs statements tie");
        assert_eq!(rows[1].status(), Status::Agreed, "covered by its criterion");

        let answer: SpecAnswer = serde_json::from_value(json!({
            "preamble": ["Two requirements."],
            "requirements": [
                {"subject": "session.timeout", "body": ["Sessions expire."],
                 "scenarios": [{"name": "Idle", "given": ["a session"], "when": "idle", "then": "expired"}]},
                {"subject": "login.flow", "body": [],
                 "scenarios": [{"name": "Login", "when": "login", "then": "decide"}]}
            ]
        }))
        .expect("draft shape");
        let brief = SpecBrief::new(&sources, &rows);
        brief.check(&answer).expect("draft fits the rows");
        let spec = brief.conclude(answer);
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

        let answer: DesignAnswer = serde_json::from_value(json!({
            "preamble": [],
            "sections": [
                {"kind": "domain-model", "blocks": [{"type": "session.type"}]},
                {"kind": "overview", "blocks": [{"text": "Sessions (from docs)."}]}
            ]
        }))
        .expect("draft shape");
        let brief = DesignBrief::new(&spec, &sources);
        brief.check(&answer).expect("draft fits the plan");
        let design = brief.conclude(answer);
        let read: Design = design.parse().expect("the rendering is canonical");
        let kinds: Vec<SectionKind> = read.sections.iter().map(|s| s.kind).collect();
        assert_eq!(kinds, [SectionKind::Overview, SectionKind::DomainModel], "vocabulary order");
        assert!(design.contains("```\ntype Session = {}\n```"), "{design}");
    }
}
