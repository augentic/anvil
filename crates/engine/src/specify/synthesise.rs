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

use std::fmt::Write as _;

use emery_source::types::{Claim, ClaimKind};
use omnia_guest::model::Question;
use omnia_guest::{Error, Model};
use strum::VariantArray as _;

use crate::artifact::{ReqId, SectionKind, Status};
use crate::specify;
use crate::specify::answer::{DesignAnswer, SpecAnswer};
use crate::specify::extract::SourceSet;
use crate::specify::provenance::Provenance;
use crate::specify::render;
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

/// Drafts, validates, and renders both documents.
///
/// # Errors
///
/// Returns the model failure or the exhausted draft findings.
pub async fn synthesise<M: Model>(
    model: &M, sets: &[SourceSet], rows: &[Provenance],
) -> Result<Revision, Error> {
    tracing::info!("drafting spec.md");
    let drafted = Question::<SpecAnswer>::new("spec-draft")
        .system(specify::system(SPEC_PROSE))
        .schema(SpecAnswer::hints(rows))
        .ask(model, spec_prompt(sets, rows), None, |drafted| drafted.check(rows))
        .await?;
    let spec = render::spec(rows, &drafted);

    tracing::info!("drafting design.md");
    let plan = plan(sets);
    let drafted = Question::<DesignAnswer>::new("design-draft")
        .system(specify::system(DESIGN_PROSE))
        .schema(DesignAnswer::hints(&plan, sets))
        .ask(model, design_prompt(sets, &spec, &plan), None, |drafted| drafted.check(&plan, sets))
        .await?;
    let design = render::design(sets, &drafted);

    Ok(Revision { spec, design })
}

/// Plans `design.md`: which sections the claims require, allow, or forbid.
#[must_use]
pub fn plan(sets: &[SourceSet]) -> Plan {
    let mut kinds: Vec<ClaimKind> = Vec::new();
    for claim in sets.iter().flat_map(|set| &set.evidence.claims) {
        if !kinds.contains(&claim.kind) {
            kinds.push(claim.kind);
        }
    }

    Plan { kinds }
}

/// The `design.md` section plan: the claim kinds the run extracted, which
/// decide each section of the closed vocabulary.
#[derive(Debug)]
pub struct Plan {
    kinds: Vec<ClaimKind>,
}

impl Plan {
    /// The plan's verdict on `kind`.
    #[must_use]
    pub fn presence(&self, kind: SectionKind) -> Presence {
        let informed = informants(kind).iter().any(|claim| self.kinds.contains(claim));
        match kind {
            SectionKind::Overview => Presence::Required,
            SectionKind::Observability => Presence::Permitted,
            _ if informed => Presence::Required,
            SectionKind::TechnicalLogic => Presence::Permitted,
            _ => Presence::Forbidden,
        }
    }

    /// Every section the plan requires, in vocabulary order.
    pub fn required(&self) -> impl Iterator<Item = SectionKind> + '_ {
        SectionKind::VARIANTS
            .iter()
            .copied()
            .filter(|kind| self.presence(*kind) == Presence::Required)
    }
}

/// Whether the evidence calls for a section, tolerates it, or rules it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub enum Presence {
    /// The section must be drafted.
    Required,
    /// The section may be drafted where claims inform it.
    Permitted,
    /// The section may not be drafted.
    #[strum(to_string = "omit")]
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
    for &kind in SectionKind::VARIANTS {
        let presence = plan.presence(kind);
        let kinds = informants(kind).iter().map(|kind| format!("`{kind}`")).collect::<Vec<_>>();
        let reason = match (presence, kinds.is_empty()) {
            (Presence::Required, false) => format!(": {} claims are present", kinds.join(" / ")),
            (Presence::Forbidden, false) => format!(": no {} claim", kinds.join(" / ")),
            (Presence::Permitted, _) => " where claims inform it".to_string(),
            _ => String::new(),
        };
        let _ =
            writeln!(prompt, "- `{key}` (`## {kind}`) — {presence}{reason}", key = kind.as_ref());
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

fn render_claims(prompt: &mut String, sets: &[SourceSet]) {
    prompt.push_str("## Claims\n");

    for set in sets {
        let _ = write!(
            prompt,
            "\n### source `{key}` ({authority})\n\n",
            key = set.key,
            authority = set.evidence.authority
        );

        for claim in &set.evidence.claims {
            let id = claim.id.as_deref().unwrap_or("-");
            let synopsis = claim.synopsis.as_deref().unwrap_or("");
            let extras = serde_json::to_string(&claim.extras).unwrap_or_default();
            let _ = writeln!(prompt, "- {kind} `{id}` — {synopsis} — {extras}", kind = claim.kind);
        }
    }
}
