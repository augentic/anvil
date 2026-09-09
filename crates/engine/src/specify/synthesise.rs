//! Synthesis
//!
//! Turns the requirement rows and the extracted claims into the two
//! specification documents. The model is asked two typed questions in turn —
//! the content of `spec.md`, then the content of `design.md` — each put as a
//! brief that verifies every candidate answer against the rows, the section
//! plan, and the evidence before the engine renders the accepted answer into
//! the canonical document. Every heading, provenance line, tag, note, and
//! signature is the engine's, so the stored bytes are a function of the facts
//! and the draft alone, and a changed byte re-ids every revision.
//!
//! Nothing the engine already knows is asked of the model: it never writes a
//! heading, an id, a `Sources:` list, a status, a note, or a type signature,
//! so it cannot drop, reorder, or quietly rewrite a requirement, invent or
//! omit a section, cite an unbound source, or paraphrase a signature.

mod design;
mod spec;

use std::fmt::{self, Display, Formatter};

use omnia_guest::model::Findings;
use omnia_guest::{Error, Model};
use serde_json::Value;

use self::design::DesignBrief;
use self::spec::SpecBrief;
use crate::specify::SourceEvidence;
use crate::specify::brief::Brief as _;
use crate::specify::provenance::Provenance;
use crate::store::Revision;

// Line openers the renderer reserves for its own markup; no drafted paragraph
// may start a line with one of them.
const RESERVED: &[&str] = &["#", "ID:", "Sources:", "Status:", "Note:"];

/// Takes evidence from all queried sources, with the requirement rows derived
/// from it, and asks the model to synthesise them into a single specification
/// set containing both the specification and design documents.
///
/// # Errors
///
/// A model failure is `bad_gateway`; an answer outside the schema, or a draft
/// the backend could not repair within its rounds, is `bad_request`.
pub async fn synthesise<M: Model>(
    model: &M, sources: &[SourceEvidence], rows: &[Provenance],
) -> Result<Revision, Error> {
    let spec = SpecBrief::new(sources, rows).judge(model).await?;
    let design = DesignBrief::new(&spec, sources).judge(model).await?;
    Ok(Revision { spec, design })
}

// Joins rendered blocks into the document text: one blank line between
// blocks, trailing spaces stripped from every line, one trailing newline.
fn document(blocks: &[String]) -> String {
    let mut text = blocks
        .iter()
        .map(|b| b.lines().map(str::trim_end).collect::<Vec<_>>().join("\n"))
        .collect::<Vec<_>>()
        .join("\n\n");
    text.push('\n');
    text
}

// Writes the `## Claims` section of a brief's prompt: every claim of every
// source, grouped under the source's key and authority, so the model sees
// the whole body of evidence it must draft from.
fn render_claims(f: &mut Formatter<'_>, sources: &[SourceEvidence]) -> fmt::Result {
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

fn paragraphs(texts: &[String], label: impl Display, findings: &mut Findings) {
    for text in texts {
        paragraph(text, &label, findings);
    }
}

// Checks one drafted paragraph, recording a finding when it is blank or
// when any of its lines opens with a marker the renderer reserves.
fn paragraph(text: &str, label: impl Display, findings: &mut Findings) {
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

// Checks one scenario field, recording a finding when it is blank or spans
// more than one line.
fn line(text: &str, label: impl Display, findings: &mut Findings) {
    if text.trim().is_empty() {
        findings.push(format!("- {label} is blank"));
    } else if text.contains('\n') {
        findings.push(format!("- {label} spans more than one line"));
    }
}
