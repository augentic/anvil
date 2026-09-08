//! Reconciliation and synthesis
//!
//! Turns extracted claims into the two specification documents. Reconciling
//! is deterministic: requirement claims about the same subject are grouped,
//! agreement and disagreement are resolved by source authority, and
//! requirements with no acceptance criterion are recorded as gaps. Synthesis
//! then asks the model to write `spec.md` and `design.md` from those rows.
//!
//! Splitting the two keeps every judgement about *which* sources win out of
//! the model's hands. The model only writes prose: its `spec.md` is parsed
//! and compared back against the rows so it cannot drop, reorder, or quietly
//! rewrite a requirement, and its `design.md` is parsed and compared against
//! the section plan the claims dictate so it cannot invent or omit a section,
//! cite an unbound source, or paraphrase a type signature.

use std::collections::BTreeMap;
use std::fmt::{self, Display, Write as _};

use emery_source::types::{Authority, ClaimKind};
use omnia_guest::model::{Message, Request, Role};
use omnia_guest::{Error, Model, bad_gateway, bad_request};
use serde_json::Value;

use super::extract::SourceSet;
use crate::artifact::{Design, ReqId, SectionKind, Spec, Status};
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

/// Plans `design.md`: which sections the claims require, allow, or forbid.
#[must_use]
pub fn plan(sets: &[SourceSet]) -> Plan {
    let present = |kinds: &[ClaimKind]| {
        sets.iter().flat_map(|set| &set.claims).any(|claim| kinds.contains(&claim.kind))
    };
    let sections = SectionKind::ALL
        .into_iter()
        .map(|kind| {
            let presence = match kind {
                SectionKind::Overview => Presence::Required,
                SectionKind::Observability => Presence::Permitted,
                _ if present(informants(kind)) => Presence::Required,
                SectionKind::TechnicalLogic => Presence::Permitted,
                _ => Presence::Forbidden,
            };
            (kind, presence)
        })
        .collect();
    Plan(sections)
}

/// Synthesises both documents and validates the model answers.
///
/// # Errors
///
/// Returns model, AST, provenance, or section-plan failures.
pub async fn synthesise<M: Model>(
    model: &M, sets: &[SourceSet], rows: &[Row],
) -> Result<Revision, Error> {
    tracing::info!("synthesising spec.md");
    let spec = dispatch(model, SPEC_PROSE, &spec_prompt(sets, rows)).await?;
    check_rows(&spec.parse()?, rows)?;

    tracing::info!("synthesising design.md");
    let plan = plan(sets);
    let design = dispatch(model, DESIGN_PROSE, &design_prompt(sets, &spec, &plan)).await?;
    check_design(&design.parse()?, &plan, sets)?;

    Ok(Revision { spec, design })
}

/// The `design.md` section plan: one presence per section of the closed
/// vocabulary, a function of the claim kinds alone.
#[derive(Debug)]
pub struct Plan(BTreeMap<SectionKind, Presence>);

// Whether the evidence calls for a section, tolerates it, or rules it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presence {
    Required,
    Permitted,
    Forbidden,
}

// The claim kinds whose presence requires a section; `Overview` and
// `Observability` have no deterministic informant.
const fn informants(kind: SectionKind) -> &'static [ClaimKind] {
    match kind {
        SectionKind::Overview | SectionKind::Observability => &[],
        SectionKind::DomainModel => &[ClaimKind::Type],
        SectionKind::Apis => &[ClaimKind::Call, ClaimKind::Contract],
        SectionKind::TechnicalLogic => &[ClaimKind::Excerpt],
        SectionKind::UiLayout => &[ClaimKind::Region, ClaimKind::Container, ClaimKind::Leaf],
    }
}

// Whitespace-collapsed text, so a reflowed quotation still matches.
fn normalise(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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
        normalise(&self.statement)
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

fn design_prompt(sets: &[SourceSet], spec: &str, plan: &Plan) -> String {
    let mut prompt = String::from("Author `design.md`.\n\n");
    render_claims(&mut prompt, sets);

    prompt.push_str("\n## Sections (render exactly, in order)\n\n");
    for (kind, presence) in &plan.0 {
        let kinds = informants(*kind).iter().map(|kind| format!("`{kind}`")).collect::<Vec<_>>();
        let reason = match (presence, kinds.is_empty()) {
            (Presence::Required, false) => format!(": {} claims are present", kinds.join(" / ")),
            (Presence::Forbidden, false) => format!(": no {} claim", kinds.join(" / ")),
            (Presence::Permitted, _) => " where claims inform it".to_string(),
            _ => String::new(),
        };
        let _ = writeln!(prompt, "- `## {kind}` — {presence}{reason}");
    }

    let _ = write!(prompt, "\n## The validated `spec.md`\n\n{spec}");
    prompt
}

impl Display for Presence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Required => "required",
            Self::Permitted => "permitted",
            Self::Forbidden => "omit",
        })
    }
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

// The model may not omit a required section, pad an uninformed one, cite
// a source that is not bound, or paraphrase a type signature.
fn check_design(design: &Design, plan: &Plan, sets: &[SourceSet]) -> Result<(), Error> {
    let sections = design.by_kind();
    for (kind, presence) in &plan.0 {
        match (presence, sections.contains_key(kind)) {
            (Presence::Required, false) => {
                return Err(unevidenced(format!("`## {kind}` is required but absent")));
            }
            (Presence::Forbidden, true) => {
                return Err(unevidenced(format!("`## {kind}` is present but no claim informs it")));
            }
            _ => {}
        }
    }

    for section in &design.sections {
        if let Some(key) = section.citations().find(|key| !sets.iter().any(|set| set.key == *key)) {
            let kind = section.kind;
            return Err(unevidenced(format!(
                "`## {kind}` cites source `{key}`, which is not bound"
            )));
        }
    }

    // A quoted signature survives reflowing, never rewording.
    let domain = sections.get(&SectionKind::DomainModel).map(|section| normalise(section.body()));
    let types =
        sets.iter().flat_map(|set| &set.claims).filter(|claim| claim.kind == ClaimKind::Type);
    for claim in types {
        let Some(Value::String(signature)) = claim.extras.get("signature") else { continue };
        let quoted = domain.as_deref().is_some_and(|body| body.contains(&normalise(signature)));
        if !quoted {
            let label = claim.id.as_deref().or(claim.path.as_deref()).unwrap_or("<unnamed>");
            return Err(unevidenced(format!(
                "`## Domain model` must quote the signature of `type` `{label}` verbatim"
            )));
        }
    }

    Ok(())
}

fn unevidenced(detail: impl Display) -> Error {
    bad_request!("model `design.md` does not match the evidence: {detail}")
}
