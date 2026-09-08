//! Synthesis
//!
//! Turns the requirement rows and the extracted claims into the two
//! specification documents. The model is asked two typed questions — the
//! content of `spec.md`, keyed by row subject, then the content of
//! `design.md`, keyed by planned section — and each answer is checked against
//! the rows, the section plan, and the evidence, with findings fed back for
//! bounded repair. The engine then renders the canonical documents itself.
//!
//! Nothing the engine already knows is asked of the model: it never writes a
//! heading, an id, a `Sources:` list, a status, a note, or a type signature,
//! so it cannot drop, reorder, or quietly rewrite a requirement, invent or
//! omit a section, cite an unbound source, or paraphrase a signature.

use std::collections::BTreeMap;
use std::fmt::{self, Display, Write as _};

use emery_source::types::{Claim, ClaimKind};
use omnia_guest::{Error, Model};

use super::draft::{DesignDraft, SpecDraft};
use super::extract::SourceSet;
use super::judgment::{self, Question};
use super::provenance::Provenance;
use super::render;
use crate::artifact::{ReqId, SectionKind, Status};
use crate::store::Revision;

// Prompt order is significant.
const SPEC_PROSE: &[&str] = &[
    "synthesis/synthesise.md",
    "synthesis/authority.md",
    "synthesis/claim-landing.md",
    "synthesis/requirement-block.md",
    "synthesis/spec-format.md",
    "synthesis/tags.md",
];
const DESIGN_PROSE: &[&str] = &["synthesis/synthesise.md", "synthesis/design-format.md"];

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

/// Drafts, validates, and renders both documents.
///
/// # Errors
///
/// Returns the model failure or the exhausted draft findings.
pub async fn synthesise<M: Model>(
    model: &M, sets: &[SourceSet], rows: &[Provenance],
) -> Result<Revision, Error> {
    tracing::info!("drafting spec.md");
    let question = Question {
        system: judgment::system(SPEC_PROSE),
        name: "spec-draft",
        schema: SpecDraft::schema(),
    };
    let drafted: SpecDraft = question
        .ask(model, &spec_prompt(sets, rows), |answer| {
            let drafted: SpecDraft = judgment::parse(answer)?;
            drafted.check(rows)?;
            Ok(drafted)
        })
        .await?;
    let spec = render::spec(rows, &drafted);

    tracing::info!("drafting design.md");
    let plan = plan(sets);
    let question = Question {
        system: judgment::system(DESIGN_PROSE),
        name: "design-draft",
        schema: DesignDraft::schema(),
    };
    let drafted: DesignDraft = question
        .ask(model, &design_prompt(sets, &spec, &plan), |answer| {
            let drafted: DesignDraft = judgment::parse(answer)?;
            drafted.check(&plan, sets)?;
            Ok(drafted)
        })
        .await?;
    let design = render::design(sets, &drafted);

    Ok(Revision { spec, design })
}

/// The `design.md` section plan: one presence per section of the closed
/// vocabulary, a function of the claim kinds alone.
#[derive(Debug)]
pub struct Plan(BTreeMap<SectionKind, Presence>);

impl Plan {
    /// The plan's verdict on `kind`.
    #[must_use]
    pub fn presence(&self, kind: SectionKind) -> Presence {
        self.0.get(&kind).copied().unwrap_or(Presence::Forbidden)
    }

    /// Every section the plan requires.
    pub fn required(&self) -> impl Iterator<Item = SectionKind> + '_ {
        self.0
            .iter()
            .filter(|(_, presence)| **presence == Presence::Required)
            .map(|(kind, _)| *kind)
    }
}

/// Whether the evidence calls for a section, tolerates it, or rules it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// The section must be drafted.
    Required,
    /// The section may be drafted where claims inform it.
    Permitted,
    /// The section may not be drafted.
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

fn spec_prompt(sets: &[SourceSet], rows: &[Provenance]) -> String {
    let mut prompt = String::from("Draft `spec.md`.\n\n");
    render_claims(&mut prompt, sets);

    prompt.push_str("\n## Requirement rows (draft one entry per subject)\n\n");
    for (index, row) in rows.iter().enumerate() {
        let sources = row.sources().collect::<Vec<_>>().join(", ");
        let coverage = if row.covered() { "evidenced" } else { "not evidenced" };
        let _ = writeln!(
            prompt,
            "- {id} `{subject}` — Status: {status} — Sources: [{sources}] — acceptance criteria {coverage}",
            id = ReqId::nth(index),
            subject = row.subject(),
            status = row.status(),
        );
        for (position, class) in row.classes().iter().enumerate() {
            let role = match (row.status(), position) {
                (Status::Divergence, 0) => "winner",
                (Status::Divergence, _) => "loser",
                _ => "contributor",
            };
            for member in class {
                let _ = writeln!(
                    prompt,
                    "  - {role}: {source} ({authority}, `{claim}`): {statement}",
                    source = member.source,
                    authority = member.authority,
                    claim = member.id,
                    statement = member.statement,
                );
            }
        }
    }
    prompt
}

fn design_prompt(sets: &[SourceSet], spec: &str, plan: &Plan) -> String {
    let mut prompt = String::from("Draft `design.md`.\n\n");
    render_claims(&mut prompt, sets);

    prompt.push_str("\n## Sections\n\n");
    for (kind, presence) in &plan.0 {
        let kinds = informants(*kind).iter().map(|kind| format!("`{kind}`")).collect::<Vec<_>>();
        let reason = match (presence, kinds.is_empty()) {
            (Presence::Required, false) => format!(": {} claims are present", kinds.join(" / ")),
            (Presence::Forbidden, false) => format!(": no {} claim", kinds.join(" / ")),
            (Presence::Permitted, _) => " where claims inform it".to_string(),
            _ => String::new(),
        };
        let _ = writeln!(prompt, "- `{key}` (`## {kind}`) — {presence}{reason}", key = kind.key());
    }

    let keys: Vec<&str> =
        sets.iter().flat_map(SourceSet::types).filter_map(Claim::type_key).collect();
    if !keys.is_empty() {
        prompt.push_str(
            "\n## Type blocks\n\nReference each `type` claim exactly once under `domain-model` \
             as a `{\"type\": \"<key>\"}` block; the engine inserts its signature verbatim.\n\n",
        );
        for key in keys {
            let _ = writeln!(prompt, "- `{key}`");
        }
    }

    let _ = write!(prompt, "\n## The rendered `spec.md`\n\n{spec}");
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
