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

// A paragraph line may not open with anything the renderer owns.
const RESERVED: &[&str] = &["#", "ID:", "Sources:", "Status:", "Note:"];

/// Takes evidence from all queried sources and asks the model to synthesise 
/// into a single specification set containing both the specification and 
/// design documents.
/// 
/// # Errors
/// 
/// Returns the model failure or the synthesis findings.
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
        .map(|b| b.lines().map(str::trim_end).collect::<Vec<_>>().join("\n"))
        .collect::<Vec<_>>()
        .join("\n\n");
    text.push('\n');
    text
}

// Every claim of every source, for a brief's turn.
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

// A paragraph is non-blank and opens no line with a reserved marker.
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

// A scenario field is one non-blank line.
fn line(text: &str, label: impl Display, findings: &mut Findings) {
    if text.trim().is_empty() {
        findings.push(format!("- {label} is blank"));
    } else if text.contains('\n') {
        findings.push(format!("- {label} spans more than one line"));
    }
}
