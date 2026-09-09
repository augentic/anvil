//! # The revision artifacts
//!
//! The two documents a `specify` run commits, `spec.md` and `design.md`, are
//! rendered by the engine from validated drafts, so a stored document is
//! canonical output. This module reads it back — for the re-mine diff, which
//! compares two revisions by requirement subject and section heading — and
//! carries the vocabulary the renderer and the reader share: the heading
//! markers, the positional requirement id, the closed status and tag sets,
//! and the closed section vocabulary.
//!
//! A stored document that does not fit its grammar was not rendered by this
//! engine; the reader reports it as corruption, not as a grammar finding.

mod design;
mod spec;

use std::ops::Deref;

use serde::{Deserialize, Serialize};

pub use self::design::{Design, SectionKind, citations};
pub use self::spec::{HEADING, ReqId, SCENARIO, Spec, Status};

/// The reviewable documents of one revision, in digest order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum::VariantArray)]
#[serde(rename_all = "kebab-case")]
pub enum Document {
    /// The behavioural specification document.
    Spec,
    /// The rebuild design document.
    Design,
}

impl Document {
    /// The document's file name in the revision store.
    #[must_use]
    pub const fn file(self) -> &'static str {
        match self {
            Self::Spec => "spec.md",
            Self::Design => "design.md",
        }
    }
}

/// A document's text as right-trimmed lines.
#[derive(Debug)]
pub struct Text<'a> {
    lines: Vec<Line<'a>>,
}

impl<'a> From<&'a str> for Text<'a> {
    fn from(text: &'a str) -> Self {
        let lines = text.lines().map(|raw| Line(raw.trim_end())).collect();
        Self { lines }
    }
}

impl<'a> Text<'a> {
    /// Every run of lines led by a `marker` heading: the heading line with
    /// its marker stripped, then the body. The preamble before the first
    /// heading is skipped.
    pub fn blocks(&'a self, marker: &'a str) -> impl Iterator<Item = (Line<'a>, Lines<'a>)> {
        self.lines.chunk_by(move |_, next| next.heading(marker).is_none()).filter_map(move |run| {
            let [first, body @ ..] = run else { return None };
            Some((first.heading(marker)?, Lines(body)))
        })
    }
}

/// One right-trimmed line.
#[derive(Debug, Clone, Copy)]
pub struct Line<'a>(pub &'a str);

impl Line<'_> {
    /// Whether the line is empty.
    #[must_use]
    pub const fn is_blank(self) -> bool {
        self.0.is_empty()
    }

    // The heading text after `marker`; `None` for any other line.
    fn heading(self, marker: &str) -> Option<Self> {
        Some(Self(self.0.strip_prefix(marker)?.trim()))
    }
}

/// A run of lines; derefs to the slice.
#[derive(Debug, Clone, Copy)]
pub struct Lines<'a>(pub &'a [Line<'a>]);

impl<'a> Deref for Lines<'a> {
    type Target = [Line<'a>];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl Lines<'_> {
    /// The joined text with blank edges trimmed.
    #[must_use]
    pub fn text(self) -> String {
        let text: Vec<&str> = self.iter().map(|line| line.0).collect();
        text.join("\n").trim_matches('\n').to_string()
    }
}
