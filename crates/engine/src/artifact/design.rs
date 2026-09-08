//! # Parse `design.md`
//!
//! A design is a preamble followed by `## ` sections drawn from a closed
//! vocabulary in a fixed order, each with a body. The section set is what
//! synthesis compares against the plan the claims dictate, and the section
//! heading is what the re-mine diff keys on.

use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::str::FromStr;

use omnia_guest::Error;

use super::{
    Document, Finding, Findings, HEADING as REQUIREMENT, Line, Lines, Malformed, SCENARIO,
};
use crate::is_kebab;

const MARKER: &str = "## ";
const CITATION: &str = "(from ";

/// A parsed `design.md`.
#[derive(Debug)]
pub struct Design {
    /// Sections in document order.
    pub sections: Vec<Section>,
}

impl Design {
    /// The artifact's file name.
    pub const NAME: &str = "design.md";

    /// Sections keyed by their heading, the stable diff identity.
    #[must_use]
    pub fn by_kind(&self) -> BTreeMap<SectionKind, &Section> {
        self.sections.iter().map(|section| (section.kind, section)).collect()
    }

    // Each section appears once, in the vocabulary's order.
    fn ordered(&self) -> Findings {
        let mut findings = Findings::default();
        let mut seen: Vec<SectionKind> = Vec::new();
        for section in &self.sections {
            let kind = section.kind;
            if seen.contains(&kind) {
                findings.push(format!("duplicate section `## {kind}`"));
            } else if let Some(later) = seen.iter().find(|earlier| **earlier > kind) {
                findings.push(format!("`## {kind}` must precede `## {later}`"));
            }
            seen.push(kind);
        }
        findings
    }
}

impl FromStr for Design {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        let document = Document::from(text);
        let blocks: Vec<Block<'_>> = document.blocks(MARKER).map(Block::new).collect();

        let mut findings = Findings::default();
        if blocks.is_empty() {
            findings.push("the document carries no `##` section");
        }
        let sections =
            blocks.iter().filter_map(|block| findings.record(Section::try_from(block))).collect();
        let design = Self { sections };
        findings.extend(design.ordered());

        findings.accept(Self::NAME, design)
    }
}

/// One `## ` section.
#[derive(Debug)]
pub struct Section {
    /// The heading, from the closed vocabulary.
    pub kind: SectionKind,
    // The text below the heading, blank edges trimmed.
    body: String,
}

impl Section {
    /// The section text below its heading.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// The key of every `(from <key>)` citation, in document order.
    pub fn citations(&self) -> impl Iterator<Item = &str> {
        citations(&self.body)
    }
}

impl TryFrom<&Block<'_>> for Section {
    type Error = Findings;

    // Every fault in the section is reported, not just the first.
    fn try_from(block: &Block<'_>) -> Result<Self, Findings> {
        let mut findings = Findings::default();

        let kind = findings.record(block.kind());
        findings.record(block.filled());
        findings.extend(block.leaks());
        findings.extend(block.citations());

        let Some(kind) = kind else { return Err(findings) };
        findings.finish(Self {
            kind,
            body: block.body.text(),
        })
    }
}

// The same section in two revisions differs in nothing but its position.
impl PartialEq for Section {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.body == other.body
    }
}

/// The closed `## ` vocabulary, in document order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SectionKind {
    /// `## Overview`: what the system is and why.
    Overview,
    /// `## Domain model`: types and identifiers.
    DomainModel,
    /// `## APIs and integrations`: external surfaces.
    Apis,
    /// `## Technical logic`: delegation, validation, errors.
    TechnicalLogic,
    /// `## UI / layout`: the spatial tree.
    UiLayout,
    /// `## Observability`: metrics, traces, logs.
    Observability,
}

impl SectionKind {
    /// Every section, in document order.
    pub const ALL: [Self; 6] = [
        Self::Overview,
        Self::DomainModel,
        Self::Apis,
        Self::TechnicalLogic,
        Self::UiLayout,
        Self::Observability,
    ];

    const fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::DomainModel => "Domain model",
            Self::Apis => "APIs and integrations",
            Self::TechnicalLogic => "Technical logic",
            Self::UiLayout => "UI / layout",
            Self::Observability => "Observability",
        }
    }
}

impl FromStr for SectionKind {
    type Err = Malformed;

    fn from_str(text: &str) -> Result<Self, Malformed> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.title() == text)
            .ok_or_else(|| Malformed(text.to_string()))
    }
}

impl Display for SectionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.title())
    }
}

// One `## ` section: the heading line less its marker, then the body.
#[derive(Debug)]
struct Block<'a> {
    heading: Line<'a>,
    body: Lines<'a>,
}

impl<'a> Block<'a> {
    const fn new((heading, body): (Line<'a>, Lines<'a>)) -> Self {
        Self { heading, body }
    }

    fn kind(&self) -> Result<SectionKind, Finding> {
        self.heading
            .text
            .parse::<SectionKind>()
            .map_err(|Malformed(title)| self.heading.fault(format!("unknown section `## {title}`")))
    }

    // Every section says something; an empty one is padding.
    fn filled(&self) -> Result<(), Finding> {
        if self.body.iter().all(|line| line.is_blank()) {
            return Err(self.heading.fault("section has no body"));
        }
        Ok(())
    }

    // Requirement blocks and scenarios belong to `spec.md` alone.
    fn leaks(&self) -> Findings {
        self.body
            .iter()
            .filter_map(|line| {
                [REQUIREMENT, SCENARIO]
                    .into_iter()
                    .find(|marker| line.text.starts_with(marker))
                    .map(|marker| line.fault(format!("`{marker}` belongs in `spec.md`")))
            })
            .collect()
    }

    // Every citation key is kebab-case; whether it is bound is
    // synthesis's rule, where the bindings are known.
    fn citations(&self) -> Findings {
        self.body
            .iter()
            .flat_map(|line| {
                citations(line.text)
                    .filter(|key| !is_kebab(key))
                    .map(|key| line.fault(format!("malformed citation `(from {key})`")))
            })
            .collect()
    }
}

// Every `(from <key>)` key in `text`. The parenthesised text must be one
// token: a phrase such as `(from the browser)` is prose, not a citation.
fn citations(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices(CITATION).filter_map(|(at, _)| {
        let (key, _) = text[at + CITATION.len()..].split_once(')')?;
        (!key.is_empty() && !key.contains(char::is_whitespace)).then_some(key)
    })
}

// `tests/specify.rs` owns what the operator observes — the typed refusal
// that commits nothing, and the re-mine diff — over representative breaches.
// The parser is private and its grammar edges are a case-per-cell explosion
// at that entry point, so the per-fault messages stay here.
#[cfg(test)]
mod tests {
    use super::{Design, SectionKind};

    // Nothing outside the parser reads the section text raw; the public
    // fields are proven through the product, which refuses a misread plan.
    #[test]
    fn body_is_the_text_alone() {
        let text = "\
# Design

Preamble prose.

## Overview

One static endpoint (from docs).

## Domain model

`interface User { id: string }`

";
        let design: Design = text.parse().expect("two sections parse");
        assert_eq!(design.sections.len(), 2);
        assert_eq!(design.sections[0].kind, SectionKind::Overview);
        assert_eq!(
            design.sections[0].body(),
            "One static endpoint (from docs).",
            "the heading and blank edges are not body"
        );
        assert_eq!(design.sections[0].citations().collect::<Vec<_>>(), ["docs"]);
        assert_eq!(design.sections[1].body(), "`interface User { id: string }`");
    }

    #[test]
    fn violations_fail() {
        let cases: &[(&str, &str)] = &[
            ("# Title only, no sections\n", "no `##` section"),
            ("   ", "no `##` section"),
            ("## Decisions\n\nBody.\n", "unknown section `## Decisions`"),
            ("## Overview [unknown]\n\nBody.\n", "unknown section `## Overview [unknown]`"),
            ("## Overview\n\nBody.\n\n## Overview\n\nAgain.\n", "duplicate section `## Overview`"),
            (
                "## Domain model\n\nTypes.\n\n## Overview\n\nBody.\n",
                "`## Overview` must precede `## Domain model`",
            ),
            ("## Overview\n\n\n## Domain model\n\nTypes.\n", "line 1: section has no body"),
            (
                "## Overview\n\n### Requirement: greeting\n\nID: REQ-001\n",
                "line 3: `### Requirement:` belongs in `spec.md`",
            ),
            (
                "## Overview\n\n#### Scenario: Greeting\n\n- **WHEN** greeted\n",
                "line 3: `#### Scenario:` belongs in `spec.md`",
            ),
            ("## Overview\n\nCookie-bound (from Docs!).\n", "malformed citation `(from Docs!)`"),
        ];

        for (text, fragment) in cases {
            let message = text.parse::<Design>().expect_err(fragment).description();
            assert!(message.contains(fragment), "expected `{fragment}` in: {message}");
        }
    }

    // Prose that happens to open with `(from` is not a citation.
    #[test]
    fn phrases_are_not_citations() {
        let text =
            "## Overview\n\nRequests arrive (from the browser) and (from docs) they route.\n";
        let design: Design = text.parse().expect("prose parses");
        assert_eq!(design.sections[0].citations().collect::<Vec<_>>(), ["docs"]);
    }

    // Every fault is reported at once, including faults in a section whose
    // heading is unknown.
    #[test]
    fn findings_aggregate() {
        let text = "## Decisions\n\n### Requirement: leaked\n\n(from Docs!)\n\n## Overview\n\n";
        let message = text.parse::<Design>().expect_err("several faults").description();
        for fragment in [
            "unknown section `## Decisions`",
            "`### Requirement:` belongs in `spec.md`",
            "malformed citation `(from Docs!)`",
            "line 7: section has no body",
        ] {
            assert!(message.contains(fragment), "expected `{fragment}` in: {message}");
        }
    }
}
