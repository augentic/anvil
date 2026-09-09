//! The `design.md` brief
//!
//! Asks the model for the content of `design.md`: the preamble and the blocks
//! of each section. Which sections of the closed vocabulary a run calls for
//! is decided by the claim kinds it extracted, so the schema names that
//! subset, the check holds the draft to the plan, to the bound sources it
//! may cite, and to the `type` claims whose signatures the engine inserts,
//! and the accepted pair renders the canonical document.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use emery_source::types::{Claim, ClaimKind};
use omnia_guest::model::Findings;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use strum::VariantArray as _;

use crate::artifact::{SectionKind, citations};
use crate::specify::SourceEvidence;
use crate::specify::brief::{Brief, verdict};
use crate::specify::synthesise::{document, paragraph, paragraphs, render_claims};

/// The `design.md` brief: the rendered `spec.md` and the plan the draft is
/// held to.
pub struct DesignBrief<'a> {
    spec: &'a str,
    plan: Plan<'a>,
}

impl<'a> DesignBrief<'a> {
    /// Briefs `design.md` over the rendered `spec` and the plan `sources`
    /// call for.
    #[must_use]
    pub fn new(spec: &'a str, sources: &'a [SourceEvidence]) -> Self {
        Self {
            spec,
            plan: Plan::collect(sources),
        }
    }
}

impl Brief for DesignBrief<'_> {
    type Answer = DesignAnswer;
    type Output = String;

    const NAME: &'static str = "design-draft";
    const PROSE: &'static [&'static str] =
        &["synthesis/synthesise.md", "synthesis/design-format.md"];

    // At least every required section, section kinds the plan does not
    // forbid, and type blocks naming this run's `type` claims.
    fn hints(&self, schema: &mut Value) {
        schema["properties"]["sections"]["minItems"] = json!(self.plan.required().count());

        // The derived `kind` refers to the whole vocabulary; the run's
        // subset replaces the reference in place.
        let kinds = SectionKind::VARIANTS
            .iter()
            .filter(|kind| self.plan.presence(**kind) != Presence::Forbidden)
            .map(AsRef::as_ref)
            .collect::<Vec<_>>();
        if let Some(kind) = schema["$defs"]["Section"]["properties"]["kind"].as_object_mut() {
            kind.remove("$ref");
            kind.insert("type".to_string(), json!("string"));
            kind.insert("enum".to_string(), json!(kinds));
        }
        if let Some(defs) = schema["$defs"].as_object_mut() {
            defs.remove("SectionKind");
        }

        if !self.plan.keys.is_empty()
            && let Some(block) = type_block(schema)
        {
            block["properties"]["type"]["enum"] = json!(self.plan.keys);
        }
    }

    // The section set, one reference per type claim under `domain-model`,
    // bound citations, and no reserved marker.
    fn check(&self, answer: &DesignAnswer) -> Result<(), Findings> {
        let mut findings = Vec::new();
        paragraphs(&answer.preamble, "preamble", &mut findings);

        let bound: BTreeSet<&str> =
            self.plan.sources.iter().map(|source| source.key.as_str()).collect();
        let mut seen = BTreeSet::new();
        let mut references: BTreeMap<&str, usize> = BTreeMap::new();
        for section in &answer.sections {
            let kind = section.kind;
            let label = format!("`## {kind}`");
            if !seen.insert(kind) {
                findings.push(format!("- {label} is drafted more than once"));
            }
            if self.plan.presence(kind) == Presence::Forbidden {
                findings.push(format!("- {label} is present but no claim informs it"));
            }
            if section.blocks.is_empty() {
                findings.push(format!("- {label} has no block"));
            }

            for block in &section.blocks {
                match block {
                    Block::Text(text) => {
                        paragraph(text, &label, &mut findings);
                        for key in citations(text).filter(|key| !bound.contains(key)) {
                            findings.push(format!(
                                "- {label} cites source `{key}`, which is not bound"
                            ));
                        }
                    }
                    Block::Type(key) => {
                        if kind != SectionKind::DomainModel {
                            findings.push(format!(
                                "- {label} references type `{key}`; type blocks belong under \
                                 `## Domain model`"
                            ));
                        }
                        *references.entry(key.as_str()).or_default() += 1;
                    }
                }
            }
        }

        for kind in self.plan.required().filter(|kind| !seen.contains(kind)) {
            findings.push(format!("- `## {kind}` is required but absent"));
        }

        let keys = &self.plan.keys;
        for key in keys {
            match references.get(key).copied().unwrap_or_default() {
                1 => {}
                0 => findings.push(format!("- type `{key}` is never referenced")),
                n => findings.push(format!("- type `{key}` is referenced {n} times")),
            }
        }

        for key in references.keys().filter(|key| !keys.contains(*key)) {
            findings.push(format!("- type `{key}` is not a type claim"));
        }

        verdict(findings)
    }

    // Renders `design.md`: the drafted sections in vocabulary order, each
    // `type` block replaced by the claim's signature.
    fn conclude(self, answer: DesignAnswer) -> String {
        let signatures: BTreeMap<&str, &str> = self
            .plan
            .sources
            .iter()
            .flat_map(|source| source.evidence.types())
            .filter_map(|claim| Some((claim.type_key()?, claim.signature()?)))
            .collect();

        let mut blocks: Vec<String> = vec!["# Design".to_string()];
        blocks.extend(answer.preamble.iter().map(|paragraph| paragraph.trim().to_string()));

        for &kind in SectionKind::VARIANTS {
            let Some(section) = answer.sections.iter().find(|section| section.kind == kind) else {
                continue;
            };

            blocks.push(format!("## {kind}"));
            for block in &section.blocks {
                match block {
                    Block::Text(text) => blocks.push(text.trim().to_string()),
                    Block::Type(key) => {
                        let signature = signatures
                            .get(key.as_str())
                            .expect("the check held the draft to the type claims");
                        blocks.push(format!("```\n{}\n```", signature.trim_end()));
                    }
                }
            }
        }

        document(&blocks)
    }
}

// The turn: every claim, the plan's verdict per section, the type blocks to
// place, and the rendered `spec.md`.
impl fmt::Display for DesignBrief<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Draft `design.md`.\n\n")?;
        render_claims(f, self.plan.sources)?;

        f.write_str("\n## Sections\n\n")?;
        for &kind in SectionKind::VARIANTS {
            let presence = self.plan.presence(kind);
            let kinds = informants(kind).iter().map(|kind| format!("`{kind}`")).collect::<Vec<_>>();
            let reason = match (presence, kinds.is_empty()) {
                (Presence::Required, false) => {
                    format!(": {} claims are present", kinds.join(" / "))
                }
                (Presence::Forbidden, false) => format!(": no {} claim", kinds.join(" / ")),
                (Presence::Permitted, _) => " where claims inform it".to_string(),
                _ => String::new(),
            };
            writeln!(f, "- `{key}` (`## {kind}`) — {presence}{reason}", key = kind.as_ref())?;
        }

        if !self.plan.keys.is_empty() {
            f.write_str(
                "\n## Type blocks\n\nReference each `type` claim exactly once under \
                 `domain-model` as a `{\"type\": \"<key>\"}` block; the engine inserts its \
                 signature verbatim.\n\n",
            )?;
            for key in &self.plan.keys {
                writeln!(f, "- `{key}`")?;
            }
        }

        write!(f, "\n## The rendered `spec.md`\n\n{spec}", spec = self.spec)
    }
}

/// The `design.md` draft: preamble paragraphs and one entry per section.
/// Only what needs synthesis is asked for; every heading and signature is
/// the renderer's.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery design draft")]
pub struct DesignAnswer {
    /// Markdown paragraphs before the first section.
    pub preamble: Vec<String>,
    /// One entry per rendered section; any order.
    pub sections: Vec<Section>,
}

/// The drafted content of one section.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Section {
    /// The section, from the closed vocabulary.
    pub kind: SectionKind,
    /// At least one block, in reading order.
    pub blocks: Vec<Block>,
}

/// One design block: a paragraph, or a reference to a `type` claim whose
/// signature the renderer inserts verbatim.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Block {
    /// One Markdown paragraph; inline `(from <source>)` citations allowed.
    Text(String),
    /// The key of a `type` claim.
    Type(String),
}

// Everything the engine holds the draft to: the kind of every extracted
// claim, which decides each section of the closed vocabulary; the bound
// sources; and the `type` claims to reference.
struct Plan<'a> {
    kinds: Vec<ClaimKind>,
    sources: &'a [SourceEvidence],
    keys: BTreeSet<&'a str>,
}

impl<'a> Plan<'a> {
    fn collect(sources: &'a [SourceEvidence]) -> Self {
        let kinds =
            sources.iter().flat_map(|source| &source.evidence.claims).map(|claim| claim.kind);
        let keys = sources
            .iter()
            .flat_map(|source| source.evidence.types())
            .filter_map(Claim::type_key)
            .collect();

        Self {
            kinds: kinds.collect(),
            sources,
            keys,
        }
    }

    // The plan's verdict on `kind`.
    fn presence(&self, kind: SectionKind) -> Presence {
        let informed = informants(kind).iter().any(|claim| self.kinds.contains(claim));
        match (kind, informed) {
            (SectionKind::Overview, _) | (_, true) => Presence::Required,
            (SectionKind::Observability | SectionKind::TechnicalLogic, false) => {
                Presence::Permitted
            }
            (_, false) => Presence::Forbidden,
        }
    }

    // Every section the plan requires, in vocabulary order.
    fn required(&self) -> impl Iterator<Item = SectionKind> + '_ {
        SectionKind::VARIANTS
            .iter()
            .copied()
            .filter(|kind| self.presence(*kind) == Presence::Required)
    }
}

// Whether the evidence calls for a section, tolerates it, or rules it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "lowercase")]
enum Presence {
    Required,
    Permitted,
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

// The `{"type": …}` variant of the derived `Block` `oneOf`.
fn type_block(schema: &mut Value) -> Option<&mut Value> {
    schema
        .pointer_mut("/$defs/Block/oneOf")?
        .as_array_mut()?
        .iter_mut()
        .find(|variant| variant["required"] == json!(["type"]))
}
