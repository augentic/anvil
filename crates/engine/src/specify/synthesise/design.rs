//! The `design.md` brief
//!
//! Asks the model for the content of `design.md`: the preamble and the blocks
//! of each section. Which sections of the closed vocabulary a run calls for
//! is decided by the claim kinds it extracted: the schema names that subset,
//! every candidate draft is verified against the plan, the bound sources it
//! may cite, and the `type` claims whose signatures the engine inserts, and
//! the engine renders the accepted draft into the canonical document.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display};

use emery_source::types::{Claim, ClaimKind};
use omnia_guest::model::Findings;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use strum::VariantArray as _;

use crate::artifact::{SectionKind, citations};
use crate::specify::SourceEvidence;
use crate::specify::brief::Brief;
use crate::specify::synthesise::{document, paragraph, paragraphs, render_claims};

/// What the engine needs to ask the model for `design.md` and to verify its
/// draft: the rendered `spec.md` and the section plan.
pub struct DesignBrief<'a> {
    spec: &'a str,
    plan: Plan<'a>,
}

impl<'a> DesignBrief<'a> {
    /// Creates the brief for `design.md` from the rendered `spec` and a
    /// section plan derived from the claim kinds in `sources`.
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

    // Tightens the derived schema to this run's plan: at least as many
    // sections as the plan requires, `kind` limited to the kinds the plan does
    // not forbid, and `type` blocks limited to this run's `type` claim keys.
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

    // Verifies a candidate draft against the plan: every required section
    // present, none forbidden, duplicated, or empty; each `type` claim placed
    // once, only under `## Domain model`; citations bound; no reserved opener.
    fn verify(&self, answer: &DesignAnswer) -> Result<(), Findings> {
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

        if !findings.is_empty() {
            return Err(findings);
        }

        Ok(())
    }

    // Renders `design.md`: the drafted sections in vocabulary order, each
    // `type` block replaced by the claim's signature.
    fn into_output(self, answer: DesignAnswer) -> Self::Output {
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

// Renders the user turn of the prompt: every claim of every source, the
// plan's verdict on each section kind with its reason, the `type` claims to
// place, and the rendered `spec.md` the design must follow.
impl Display for DesignBrief<'_> {
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

// The facts a design draft is verified against: the kinds of every extracted
// claim (which decide the sections this run requires, permits, or forbids),
// the bound sources it may cite, and the `type` claim keys it must reference.
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

    // Decides whether section `kind` is required, permitted, or forbidden:
    // `Overview`, and any kind with an informant claim present, is required;
    // uninformed `Observability` / `TechnicalLogic` permitted; the rest forbidden.
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

    // Lists every section the plan requires, in vocabulary order.
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

// Maps a section kind to the claim kinds whose presence requires it.
// `Overview` and `Observability` have none: the first is always required, the
// second only ever permitted.
const fn informants(kind: SectionKind) -> &'static [ClaimKind] {
    match kind {
        SectionKind::Overview | SectionKind::Observability => &[],
        SectionKind::DomainModel => &[ClaimKind::Type],
        SectionKind::Apis => &[ClaimKind::Call, ClaimKind::Contract],
        SectionKind::TechnicalLogic => &[ClaimKind::Excerpt],
        SectionKind::UiLayout => &[ClaimKind::Region, ClaimKind::Container, ClaimKind::Leaf],
    }
}

// Finds the `{"type": …}` variant of the `oneOf` schemars derives for `Block`,
// so `hints` can restrict its `enum` to this run's type keys.
fn type_block(schema: &mut Value) -> Option<&mut Value> {
    schema
        .pointer_mut("/$defs/Block/oneOf")?
        .as_array_mut()?
        .iter_mut()
        .find(|variant| variant["required"] == json!(["type"]))
}
