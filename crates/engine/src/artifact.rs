//! # The revision artifacts
//!
//! The two documents a `specify` run commits, `spec.md` and `design.md`, are
//! written by a model, so each is parsed against its own grammar and verified
//! rather than trusted. A document that fails its grammar is refused once,
//! whole, with every violation named, so a malformed artifact is never
//! committed or diffed.
//!
//! This module carries what the two grammars share: a document as numbered
//! lines cut into heading-led blocks, and the findings that collect every
//! violation into that one refusal. Each grammar lives in its own child.

mod design;
mod spec;

use std::fmt::{self, Display};
use std::ops::Deref;

use omnia_guest::{Error, bad_request};

pub use self::design::{Design, SectionKind};
pub use self::spec::{HEADING, ReqId, SCENARIO, Spec, Status};

/// A document as numbered, right-trimmed lines.
#[derive(Debug)]
pub struct Document<'a> {
    lines: Vec<Line<'a>>,
}

impl<'a> From<&'a str> for Document<'a> {
    fn from(text: &'a str) -> Self {
        let lines = text
            .lines()
            .enumerate()
            .map(|(index, raw)| Line {
                no: index + 1,
                text: raw.trim_end(),
            })
            .collect();
        Self { lines }
    }
}

impl<'a> Document<'a> {
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

/// One numbered line.
#[derive(Debug, Clone, Copy)]
pub struct Line<'a> {
    /// The one-based line number, as the operator reads it.
    pub no: usize,
    /// The right-trimmed text.
    pub text: &'a str,
}

impl Line<'_> {
    /// Whether the line is empty.
    #[must_use]
    pub const fn is_blank(self) -> bool {
        self.text.is_empty()
    }

    /// A violation on this line.
    pub fn fault(self, detail: impl Display) -> Finding {
        Finding(format!("line {}: {detail}", self.no))
    }

    // The heading text after `marker`; `None` for any other line.
    fn heading(self, marker: &str) -> Option<Self> {
        let text = self.text.strip_prefix(marker)?.trim();
        Some(Self { text, ..self })
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
        let text: Vec<&str> = self.iter().map(|line| line.text).collect();
        text.join("\n").trim_matches('\n').to_string()
    }
}

/// Every violation in one document, refused together.
#[derive(Debug, Default)]
pub struct Findings(Vec<Finding>);

impl Findings {
    /// Files a violation with no line of its own.
    pub fn push(&mut self, detail: impl Display) {
        self.0.push(Finding(detail.to_string()));
    }

    /// Files every finding of `other`.
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    /// Keeps the parsed part and files its fault, so parsing continues to
    /// the end of the document.
    pub fn record<T>(&mut self, result: Result<T, impl Into<Self>>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(fault) => {
                self.extend(fault.into());
                None
            }
        }
    }

    /// The parsed value, unless anything was found.
    pub fn finish<T>(self, value: T) -> Result<T, Self> {
        if self.0.is_empty() { Ok(value) } else { Err(self) }
    }

    /// The parsed artifact `name`, or the one refusal naming every finding.
    pub fn accept<T>(self, name: &str, value: T) -> Result<T, Error> {
        self.finish(value).map_err(|findings| bad_request!("`{name}` is malformed: {findings}"))
    }
}

impl From<Finding> for Findings {
    fn from(finding: Finding) -> Self {
        Self(vec![finding])
    }
}

impl FromIterator<Finding> for Findings {
    fn from_iter<I: IntoIterator<Item = Finding>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Display for Findings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let details: Vec<&str> = self.0.iter().map(|Finding(detail)| detail.as_str()).collect();
        f.write_str(&details.join(";\n"))
    }
}

/// One grammar violation, as the operator reads it.
#[derive(Debug)]
pub struct Finding(String);

/// A value that does not fit its grammar, quoted as written.
#[derive(Debug)]
pub struct Malformed(pub String);
