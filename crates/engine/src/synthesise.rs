//! Deterministic reconciliation and fail-closed model synthesis.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use emery_artifacts::evidence::{AuthorityClass, Claim, ClaimKind};
use emery_artifacts::spec::ast::{self, Status, Tag};
use emery_error::Error;
use omnia_guest::Model;
use omnia_guest::model::{Message, Request, Role};

use crate::extract::SourceSet;

/// Validated synthesis output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Documents {
    /// Behavioural specification.
    pub spec: String,
    /// Technical design.
    pub design: String,
}

/// Provenance a `spec.md` requirement must preserve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The minted requirement id (`REQ-NNN`).
    pub id: String,
    /// Claim-group subject or appended gap description.
    pub subject: String,
    /// The resolved status.
    pub status: Status,
    /// Heading tag mirroring `status`.
    pub tag: Option<Tag>,
    /// Contributing source keys, highest authority first.
    pub sources: Vec<String>,
    /// Winning contributor index for a divergence.
    pub winner: Option<usize>,
    /// Every contributing requirement claim.
    pub contributors: Vec<Contributor>,
}

/// One source's contribution to a requirement group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contributor {
    /// The contributing binding key.
    pub source: String,
    /// The source's authority class.
    pub authority: AuthorityClass,
    /// The claim's required `statement` extra.
    pub statement: String,
}

/// Reconciles requirements by authority and appends uncovered acceptance gaps.
#[must_use]
pub fn reconcile(sets: &[SourceSet]) -> Vec<Row> {
    let mut order: Vec<&str> = Vec::new();
    let mut groups: BTreeMap<&str, Vec<Contributor>> = BTreeMap::new();
    let mut criteria: Vec<&str> = Vec::new();
    for set in sets {
        for claim in &set.claims {
            let Some(id) = claim.id.as_deref() else { continue };
            match claim.kind {
                ClaimKind::Requirement => {
                    if !groups.contains_key(id) {
                        order.push(id);
                    }
                    groups.entry(id).or_default().push(Contributor {
                        source: set.key.clone(),
                        authority: set.authority,
                        statement: statement(claim),
                    });
                }
                ClaimKind::Criterion => criteria.push(id),
                _ => {}
            }
        }
    }

    let mut rows: Vec<Row> = Vec::new();
    let mut gaps: Vec<&str> = Vec::new();
    for subject in order {
        let mut contributors = groups.remove(subject).unwrap_or_default();
        // Highest authority first; the sort is stable, so binding
        // order is conserved within a class.
        contributors.sort_by_key(|contributor| rank(contributor.authority));
        rows.push(resolve(subject, contributors));
        let covered =
            criteria.iter().any(|id| *id == subject || id.starts_with(&format!("{subject}.")));
        if !covered {
            gaps.push(subject);
        }
    }
    for subject in gaps {
        rows.push(Row {
            id: String::new(),
            subject: format!("{subject} acceptance criteria"),
            status: Status::Unknown,
            tag: Some(Tag::Unknown),
            sources: Vec::new(),
            winner: None,
            contributors: Vec::new(),
        });
    }
    for (index, row) in rows.iter_mut().enumerate() {
        row.id = format!("REQ-{:03}", index + 1);
    }
    rows
}

// Matching statements agree; a unique top authority wins divergence;
// disagreeing top-authority peers conflict.
fn resolve(subject: &str, contributors: Vec<Contributor>) -> Row {
    let sources: Vec<String> =
        contributors.iter().map(|contributor| contributor.source.clone()).collect();
    let normalised: Vec<String> =
        contributors.iter().map(|contributor| normalise(&contributor.statement)).collect();
    let agreed = normalised.iter().all(|value| value == &normalised[0]);
    let (status, tag, winner) = if agreed {
        (Status::Agreed, None, None)
    } else {
        let top = rank(contributors[0].authority);
        let top_values: Vec<&String> = contributors
            .iter()
            .zip(&normalised)
            .filter(|(contributor, _)| rank(contributor.authority) == top)
            .map(|(_, value)| value)
            .collect();
        if top_values.iter().all(|value| *value == top_values[0]) {
            (Status::Divergence, Some(Tag::Divergence), Some(0))
        } else {
            (Status::Conflict, Some(Tag::Conflict), None)
        }
    };
    Row {
        id: String::new(),
        subject: subject.to_string(),
        status,
        tag,
        sources,
        winner,
        contributors,
    }
}

/// Synthesises both documents and validates the model answers.
///
/// # Errors
///
/// Returns model, AST, provenance, or empty-design failures.
pub async fn synthesise<M: Model>(
    model: &M, sets: &[SourceSet], rows: &[Row],
) -> Result<Documents, Error> {
    let spec = dispatch(model, SPEC_PROSE, &spec_prompt(sets, rows)).await?;
    let parsed = ast::parse(&spec)?;
    check_rows(&parsed, rows)?;
    let design = dispatch(model, DESIGN_PROSE, &design_prompt(sets, &spec)).await?;
    if design.trim().is_empty() {
        return Err(Error::validation_failed(
            "design-empty",
            "`design.md` must carry the rebuild design",
            "the model answered an empty document",
        ));
    }
    Ok(Documents { spec, design })
}

// Prompt order is significant.
const SPEC_PROSE: &[&str] = &[
    "synthesis/synthesise.md",
    "synthesis/authority.md",
    "synthesis/claim-reconciliation.md",
    "synthesis/requirement-block.md",
    "synthesis/spec-format.md",
    "synthesis/tags.md",
];

const DESIGN_PROSE: &[&str] = &["synthesis/synthesise.md", "synthesis/design-format.md"];

async fn dispatch<M: Model>(model: &M, prose: &[&str], user: &str) -> Result<String, Error> {
    let system =
        prose.iter().map(|path| crate::prose::body(path)).collect::<Vec<_>>().join("\n\n---\n\n");
    let request = Request::builder()
        .system(system)
        .messages(vec![Message {
            role: Role::User,
            content: user.to_string(),
        }])
        .build();
    let reply = model.create(request).await.map_err(|err| Error::Diag {
        code: "synthesis-model-failed",
        detail: err.to_string(),
    })?;
    Ok(reply.answer)
}

fn spec_prompt(sets: &[SourceSet], rows: &[Row]) -> String {
    let mut prompt = String::from("Author `spec.md`.\n\n");
    render_claims(&mut prompt, sets);
    prompt.push_str("\n## Reconciliation rows (render exactly, in order)\n\n");
    for row in rows {
        let tag = row.tag.map(|tag| format!(" [{tag}]")).unwrap_or_default();
        let sources = row.sources.join(", ");
        let _ = writeln!(
            prompt,
            "- {id}{tag} — subject `{subject}` — Status: {status} — Sources: [{sources}]",
            id = row.id,
            subject = row.subject,
            status = row.status,
        );
        for (index, contributor) in row.contributors.iter().enumerate() {
            let role = if row.winner == Some(index) { "winner" } else { "contributor" };
            let _ = writeln!(
                prompt,
                "  - {role}: {source} ({authority}): {statement}",
                source = contributor.source,
                authority = contributor.authority,
                statement = contributor.statement,
            );
        }
    }
    prompt
}

fn design_prompt(sets: &[SourceSet], spec: &str) -> String {
    let mut prompt = String::from("Author `design.md`.\n\n");
    render_claims(&mut prompt, sets);
    let _ = write!(prompt, "\n## The validated `spec.md`\n\n{spec}");
    prompt
}

fn render_claims(prompt: &mut String, sets: &[SourceSet]) {
    prompt.push_str("## Claims\n");
    for set in sets {
        let _ = write!(
            prompt,
            "\n### source `{key}` ({authority})\n\n",
            key = set.key,
            authority = set.authority
        );
        for claim in &set.claims {
            let id = claim.id.as_deref().unwrap_or("-");
            let synopsis = claim.synopsis.as_deref().unwrap_or("");
            let extras = serde_json::Value::Object(claim.extras.clone());
            let _ = writeln!(prompt, "- {kind} `{id}` — {synopsis} — {extras}", kind = claim.kind);
        }
    }
}

// The model may not drop, reorder, or rewrite reconciliation rows.
fn check_rows(parsed: &ast::Spec, rows: &[Row]) -> Result<(), Error> {
    if parsed.requirements.len() != rows.len() {
        return Err(mismatch(format!(
            "expected {} requirement blocks, found {}",
            rows.len(),
            parsed.requirements.len()
        )));
    }
    for (requirement, row) in parsed.requirements.iter().zip(rows) {
        if requirement.id != row.id {
            return Err(mismatch(format!("expected `{}`, found `{}`", row.id, requirement.id)));
        }
        // Headings are reconciliation and re-mine-diff identity.
        if requirement.name != row.subject {
            return Err(mismatch(format!(
                "`{}` must head its subject `{}`, found `{}`",
                row.id, row.subject, requirement.name
            )));
        }
        if requirement.status != row.status || requirement.tag != row.tag {
            return Err(mismatch(format!(
                "`{}` must carry `Status: {}` and its mirroring tag",
                row.id, row.status
            )));
        }
        if requirement.sources != row.sources {
            return Err(mismatch(format!(
                "`{}` must cite `Sources: [{}]`",
                row.id,
                row.sources.join(", ")
            )));
        }
    }
    Ok(())
}

fn mismatch(detail: String) -> Error {
    Error::validation_failed(
        "spec-provenance-mismatch",
        "the model answer must render every reconciliation row verbatim",
        detail,
    )
}

// The extract gate guarantees this extra exists.
fn statement(claim: &Claim) -> String {
    match claim.extras.get("statement") {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

fn normalise(statement: &str) -> String {
    statement.split_whitespace().collect::<Vec<_>>().join(" ")
}

// Lower ranks outrank higher ranks.
const fn rank(authority: AuthorityClass) -> u8 {
    match authority {
        AuthorityClass::Intent => 0,
        AuthorityClass::Documentation => 1,
        AuthorityClass::Behaviour => 2,
    }
}
