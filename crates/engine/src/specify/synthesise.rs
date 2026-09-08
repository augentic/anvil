//! Reconciliation and synthesis
//!
//! Turns extracted claims into the two specification documents. Reconciling
//! is deterministic: requirement claims about the same subject are grouped,
//! agreement and disagreement are resolved by source authority, and
//! requirements with no acceptance criterion are recorded as gaps. Synthesis
//! then asks the model to write `spec.md` and `design.md` from those rows.
//!
//! Splitting the two keeps every judgement about *which* sources win out of
//! the model's hands. The model only writes prose, and its `spec.md` is
//! parsed and compared back against the rows so it cannot drop, reorder, or
//! quietly rewrite a requirement.

use std::fmt::{Display, Write as _};

use emery_source::types::{Authority, ClaimKind};
use omnia_guest::model::{Message, Request, Role};
use omnia_guest::{Error, Model, bad_gateway, bad_request};

use super::extract::SourceSet;
use crate::spec::{ReqId, Spec, Status};
use crate::store::Revision;

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

/// Reconciles requirements by authority and appends uncovered acceptance gaps.
#[must_use]
pub fn reconcile(sets: &[SourceSet]) -> Vec<Row> {
    // Groups keep first-seen order, which is the row order.
    let mut groups: Vec<(&str, Vec<Contributor>)> = Vec::new();
    let mut criteria: Vec<&str> = Vec::new();
    for set in sets {
        for claim in &set.claims {
            let Some(id) = claim.id.as_deref() else { continue };
            match claim.kind {
                ClaimKind::Requirement => {
                    let contributor = Contributor {
                        source: set.key.clone(),
                        authority: set.authority,
                        statement: claim.statement(),
                    };
                    match groups.iter_mut().find(|(subject, _)| *subject == id) {
                        Some((_, contributors)) => contributors.push(contributor),
                        None => groups.push((id, vec![contributor])),
                    }
                }
                ClaimKind::Criterion => criteria.push(id),
                _ => {}
            }
        }
    }

    let mut rows = Vec::with_capacity(groups.len());
    let mut gaps = Vec::new();
    for (subject, mut contributors) in groups {
        // Highest authority first; the sort is stable, so binding
        // order is conserved within a class.
        contributors.sort_by_key(|contributor| contributor.authority.rank());
        rows.push(Row::resolve(subject, contributors));

        // A criterion covers its own subject or a dotted child of it.
        let covered = criteria.iter().any(|id| {
            id.strip_prefix(subject).is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
        });
        if !covered {
            gaps.push(subject);
        }
    }
    rows.extend(gaps.into_iter().map(Row::gap));
    rows
}

/// Synthesises both documents and validates the model answers.
///
/// # Errors
///
/// Returns model, AST, provenance, or empty-design failures.
pub async fn synthesise<M: Model>(
    model: &M, sets: &[SourceSet], rows: &[Row],
) -> Result<Revision, Error> {
    tracing::info!("synthesising spec.md");
    let spec = dispatch(model, SPEC_PROSE, &spec_prompt(sets, rows)).await?;
    check_rows(&spec.parse()?, rows)?;

    tracing::info!("synthesising design.md");
    let design = dispatch(model, DESIGN_PROSE, &design_prompt(sets, &spec)).await?;
    if design.trim().is_empty() {
        return Err(bad_request!("model returned an empty `design.md`"));
    }

    Ok(Revision { spec, design })
}

/// Provenance a `spec.md` requirement must preserve; the id is positional
/// and the tag, sources, and divergence winner derive from these fields.
#[derive(Debug, Clone)]
pub struct Row {
    subject: String,
    status: Status,
    contributors: Vec<Contributor>,
}

impl Row {
    // Matching statements agree; a unique top authority wins divergence;
    // disagreeing top-authority peers conflict.
    fn resolve(subject: &str, contributors: Vec<Contributor>) -> Self {
        let status = if all_equal(contributors.iter()) {
            Status::Agreed
        } else {
            let top = contributors[0].authority.rank();
            let peers = contributors.iter().take_while(|peer| peer.authority.rank() == top);
            if all_equal(peers) { Status::Divergence } else { Status::Conflict }
        };

        Self {
            subject: subject.to_string(),
            status,
            contributors,
        }
    }

    // An acceptance gap: unknown, with no contributing source to cite.
    fn gap(subject: &str) -> Self {
        Self {
            subject: format!("{subject} acceptance criteria"),
            status: Status::Unknown,
            contributors: Vec::new(),
        }
    }

    fn sources(&self) -> impl Iterator<Item = &str> {
        self.contributors.iter().map(|contributor| contributor.source.as_str())
    }
}

// One source's contribution to a requirement group.
#[derive(Debug, Clone)]
struct Contributor {
    source: String,
    authority: Authority,
    statement: String,
}

impl Contributor {
    // Statements compare with whitespace collapsed.
    fn normalised(&self) -> String {
        self.statement.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

fn all_equal<'a>(mut contributors: impl Iterator<Item = &'a Contributor>) -> bool {
    let Some(first) = contributors.next() else { return true };
    let first = first.normalised();
    contributors.all(|contributor| contributor.normalised() == first)
}

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
    let reply = Model::complete(model, request).await.map_err(|err| bad_gateway!(err))?;
    Ok(reply.answer)
}

fn spec_prompt(sets: &[SourceSet], rows: &[Row]) -> String {
    let mut prompt = String::from("Author `spec.md`.\n\n");
    render_claims(&mut prompt, sets);

    prompt.push_str("\n## Reconciliation rows (render exactly, in order)\n\n");
    for (index, row) in rows.iter().enumerate() {
        let tag = row.status.tag().map(|tag| format!(" [{tag}]")).unwrap_or_default();
        let sources = row.sources().collect::<Vec<_>>().join(", ");
        let _ = writeln!(
            prompt,
            "- {id} — heading `### Requirement: {subject}{tag}` — Status: {status} — Sources: [{sources}]",
            id = ReqId::nth(index),
            subject = row.subject,
            status = row.status,
        );
        // Authority resolves a divergence in favour of the top contributor.
        let divergence = row.status == Status::Divergence;
        for (position, contributor) in row.contributors.iter().enumerate() {
            let role = if divergence && position == 0 { "winner" } else { "contributor" };
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
            let extras = serde_json::to_string(&claim.extras).unwrap_or_default();
            let _ = writeln!(prompt, "- {kind} `{id}` — {synopsis} — {extras}", kind = claim.kind);
        }
    }
}

// The model may not drop, reorder, or rewrite reconciliation rows.
fn check_rows(spec: &Spec, rows: &[Row]) -> Result<(), Error> {
    if spec.requirements.len() != rows.len() {
        let expected = rows.len();
        let found = spec.requirements.len();
        return Err(mismatch(format!("expected {expected} requirement blocks, found {found}")));
    }

    for (index, (requirement, row)) in spec.requirements.iter().zip(rows).enumerate() {
        let id = ReqId::nth(index);
        if requirement.id != id {
            let found = &requirement.id;
            return Err(mismatch(format!("expected `{id}`, found `{found}`")));
        }
        // Headings are the reconciliation and re-mine-diff identity.
        if requirement.subject != row.subject {
            let subject = &row.subject;
            let found = &requirement.subject;
            return Err(mismatch(format!(
                "`{id}` must head its subject `{subject}`, found `{found}`"
            )));
        }
        if requirement.status != row.status {
            let status = row.status;
            return Err(mismatch(format!(
                "`{id}` must carry `Status: {status}` and its mirroring tag"
            )));
        }
        if !requirement.sources.keys().eq(row.sources()) {
            let sources = row.sources().collect::<Vec<_>>().join(", ");
            return Err(mismatch(format!("`{id}` must cite `Sources: [{sources}]`")));
        }
    }

    Ok(())
}

fn mismatch(detail: impl Display) -> Error {
    bad_request!("model `spec.md` does not match the reconciliation rows: {detail}")
}
