//! # Read `design.md`
//!
//! A design is a preamble followed by `## ` sections drawn from a closed
//! vocabulary in a fixed order, each with a body. The section heading is
//! what the re-mine diff keys on, and the body is what it compares.

use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::str::FromStr;

use omnia_guest::{Error, server_error};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::artifact::{Document, Line, Lines};

const MARKER: &str = "## ";
const CITATION: &str = "(from ";

/// A canonical `design.md`, read back.
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
}

impl FromStr for Design {
    type Err = Error;

    // A document the renderer did not write is corruption.
    fn from_str(text: &str) -> Result<Self, Error> {
        let document = Document::from(text);
        let sections = document
            .blocks(MARKER)
            .map(Section::read)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|detail| server_error!("`{}` is not canonical: {detail}", Self::NAME))?;
        if sections.is_empty() {
            return Err(server_error!("`{}` is not canonical: no `##` section", Self::NAME));
        }
        // Each section appears once, in the vocabulary's order.
        let ordered = sections.windows(2).all(|pair| pair[0].kind < pair[1].kind);
        if !ordered {
            return Err(server_error!("`{}` is not canonical: sections out of order", Self::NAME));
        }
        Ok(Self { sections })
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
    fn read((heading, body): (Line<'_>, Lines<'_>)) -> Result<Self, String> {
        let kind = heading.0.parse::<SectionKind>()?;
        Ok(Self {
            kind,
            body: body.text(),
        })
    }
}

// The same section in two revisions differs in nothing but its position.
impl PartialEq for Section {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.body == other.body
    }
}

/// The closed `## ` vocabulary, in document order. A draft names a section
/// by its kebab-case key (`domain-model`); the document by its title.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
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

    /// The draft key: the serde spelling of the variant.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::DomainModel => "domain-model",
            Self::Apis => "apis",
            Self::TechnicalLogic => "technical-logic",
            Self::UiLayout => "ui-layout",
            Self::Observability => "observability",
        }
    }
}

impl FromStr for SectionKind {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.title() == text)
            .ok_or_else(|| format!("unknown section `## {text}`"))
    }
}

impl Display for SectionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.title())
    }
}

/// Every `(from <key>)` key in `text`. The parenthesised text must be one
/// token: a phrase such as `(from the browser)` is prose, not a citation.
pub fn citations(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices(CITATION).filter_map(|(at, _)| {
        let (key, _) = text[at + CITATION.len()..].split_once(')')?;
        (!key.is_empty() && !key.contains(char::is_whitespace)).then_some(key)
    })
}

// Keep (entry-point-unreachable): the reader's view of a stored document
// is what the re-mine diff compares, and `body` is read by nothing else.
#[cfg(test)]
mod tests {
    use crate::artifact::{Design, SectionKind, citations};

    #[test]
    fn body_is_the_text_alone() {
        let text = "\
# Design

Preamble prose.

## Overview

One static endpoint (from docs).

## Domain model

```
interface User { id: string }
```
";
        let design: Design = text.parse().expect("two sections read");
        assert_eq!(design.sections.len(), 2);
        assert_eq!(design.sections[0].kind, SectionKind::Overview);
        assert_eq!(
            design.sections[0].body, "One static endpoint (from docs).",
            "the heading and blank edges are not body"
        );
        assert_eq!(design.sections[1].body, "```\ninterface User { id: string }\n```");
    }

    // A document this engine did not render is corruption.
    #[test]
    fn non_canonical_is_corruption() {
        for text in [
            "# Title only\n",
            "## Decisions\n\nBody.\n",
            "## Domain model\n\nTypes.\n\n## Overview\n\nBody.\n",
            "## Overview\n\nBody.\n\n## Overview\n\nAgain.\n",
        ] {
            let err = text.parse::<Design>().expect_err(text);
            assert_eq!(err.code(), "server_error", "{text}");
        }
    }

    // Prose that happens to open with `(from` is not a citation.
    #[test]
    fn phrases_are_not_citations() {
        let text = "Requests arrive (from the browser) and (from docs) they route.";
        assert_eq!(citations(text).collect::<Vec<_>>(), ["docs"]);
    }
}
