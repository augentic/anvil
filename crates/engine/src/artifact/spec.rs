//! # Read `spec.md`
//!
//! A spec is a preamble followed by `### Requirement:` blocks — a heading,
//! three provenance lines, and a body. The heading subject is what the
//! re-mine diff keys on, and the provenance and body are what it compares.

use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::str::FromStr;

use omnia_guest::{Error, server_error};

use crate::artifact::{Document, Line, Lines, Text};

/// The requirement heading marker.
pub const HEADING: &str = "### Requirement:";
/// The scenario heading marker.
pub const SCENARIO: &str = "#### Scenario:";

/// A canonical `spec.md`, read back.
#[derive(Debug)]
pub struct Spec {
    /// Requirement blocks in document order.
    pub requirements: Vec<Requirement>,
}

impl Spec {
    const NAME: &str = Document::Spec.file();

    /// Requirement blocks keyed by their subject.
    #[must_use]
    pub fn by_subject(&self) -> BTreeMap<&str, &Requirement> {
        self.requirements
            .iter()
            .map(|requirement| (requirement.subject.as_str(), requirement))
            .collect()
    }
}

impl FromStr for Spec {
    type Err = Error;

    // A document the renderer did not write is corruption.
    fn from_str(text: &str) -> Result<Self, Error> {
        let requirements = Text::from(text)
            .blocks(HEADING)
            .map(Requirement::read)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|detail| server_error!("`{}` is not canonical: {detail}", Self::NAME))?;
        if requirements.is_empty() {
            return Err(server_error!("`{}` is not canonical: no `{HEADING}` block", Self::NAME));
        }
        let spec = Self { requirements };
        if spec.by_subject().len() != spec.requirements.len() {
            return Err(server_error!("`{}` is not canonical: repeated subject", Self::NAME));
        }
        Ok(spec)
    }
}

/// One requirement block. The heading tag mirrors `status`, and the
/// positional id shifts with the rows above it, so neither is kept.
#[derive(Debug)]
pub struct Requirement {
    /// The heading name: the row's subject.
    pub subject: String,
    /// The cited source keys, in order.
    pub sources: Vec<String>,
    /// The `Status:` value.
    pub status: Status,
    // The text below the provenance lines, blank edges trimmed.
    body: String,
}

impl Requirement {
    // Heading, then `ID:` / `Sources:` / `Status:` in that order, then the
    // body; anything else is not this engine's rendering.
    fn read((heading, rest): (Line<'_>, Lines<'_>)) -> Result<Self, String> {
        let (subject, tag) = heading
            .0
            .strip_suffix(']')
            .and_then(|inner| inner.rsplit_once(" ["))
            .map_or((heading.0, None), |(subject, tag)| (subject.trim_end(), Some(tag)));
        if subject.is_empty() {
            return Err("a requirement heading has no subject".to_string());
        }

        // The three provenance lines are the first non-blank lines.
        let mut cursor = 0;
        let mut field = |key: &str| -> Result<&str, String> {
            while rest.get(cursor).is_some_and(|line| line.is_blank()) {
                cursor += 1;
            }
            let value = rest
                .get(cursor)
                .and_then(|line| line.0.strip_prefix(key)?.strip_prefix(':'))
                .map(str::trim)
                .ok_or_else(|| format!("`{subject}`: no `{key}:` line where one is expected"))?;
            cursor += 1;
            Ok(value)
        };
        field("ID")?.parse::<ReqId>()?;
        let sources = field("Sources")?;
        let sources = sources
            .strip_prefix('[')
            .and_then(|inner| inner.strip_suffix(']'))
            .ok_or_else(|| format!("`{subject}`: malformed `Sources: {sources}`"))?
            .split(',')
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(str::to_string)
            .collect();
        let status = field("Status")?;
        let status = status
            .parse::<Status>()
            .map_err(|_unknown| format!("`{subject}`: unknown `Status: {status}`"))?;
        if tag != status.tag().map(|tag| tag.to_string()).as_deref() {
            return Err(format!("`{subject}`: heading tag does not mirror `Status: {status}`"));
        }

        Ok(Self {
            subject: subject.to_string(),
            sources,
            status,
            body: Lines(&rest[cursor..]).text(),
        })
    }
}

// Two readings of one requirement compare by everything the reader keeps.
impl PartialEq for Requirement {
    fn eq(&self, other: &Self) -> bool {
        self.subject == other.subject
            && self.status == other.status
            && self.sources == other.sources
            && self.body == other.body
    }
}

/// A requirement id, `REQ-NNN`. Ids are positional: the first row is `REQ-001`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReqId(String);

impl ReqId {
    const PREFIX: &str = "REQ-";

    /// The id minted for the row at zero-based `index`.
    #[must_use]
    pub fn nth(index: usize) -> Self {
        Self(format!("{}{:03}", Self::PREFIX, index + 1))
    }
}

impl FromStr for ReqId {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let digits = text.strip_prefix(Self::PREFIX).unwrap_or_default();
        if digits.len() == 3 && digits.bytes().all(|byte| byte.is_ascii_digit()) {
            Ok(Self(text.to_string()))
        } else {
            Err(format!("malformed id `{text}`"))
        }
    }
}

impl Display for ReqId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The closed `Status:` vocabulary; every status but `agreed` doubles as
/// the heading `[tag]`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, strum::Display, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum Status {
    /// One class of contributors, covered by a criterion.
    Agreed,
    /// One class of contributors, no acceptance criterion in evidence.
    Unknown,
    /// Tied top-authority disagreement; the operator must reconcile.
    Conflict,
    /// Authority-resolved disagreement; the losers are notes.
    Divergence,
}

impl Status {
    /// The heading tag this status pairs with; `agreed` carries none.
    #[must_use]
    pub fn tag(self) -> Option<Self> {
        (self != Self::Agreed).then_some(self)
    }
}

// Keep (entry-point-unreachable): the reader's view of a stored document
// is what the re-mine diff compares, and `body` is read by nothing else.
#[cfg(test)]
mod tests {
    use crate::artifact::Spec;

    #[test]
    fn text_body() {
        let text = "\
# Specification

Preamble.

### Requirement: session.timeout [divergence]

ID: REQ-001
Sources: [intent, docs]
Status: divergence

Sessions expire after 30 minutes.

Note: code (behaviour, session-expiry): Sessions expire after 15 minutes.

#### Scenario: Session expires

- **WHEN** a session is idle for 30 minutes
- **THEN** it expires
";
        let spec: Spec = text.parse().expect("one block reads");
        assert_eq!(spec.requirements.len(), 1);
        let requirement = &spec.requirements[0];
        assert_eq!(requirement.subject, "session.timeout");
        assert_eq!(requirement.sources, ["intent", "docs"]);
        assert_eq!(
            requirement.body,
            "Sessions expire after 30 minutes.\n\n\
             Note: code (behaviour, session-expiry): Sessions expire after 15 minutes.\n\n\
             #### Scenario: Session expires\n\n\
             - **WHEN** a session is idle for 30 minutes\n\
             - **THEN** it expires",
            "provenance lines and blank edges are not body"
        );
    }

    // A document this engine did not render is corruption.
    #[test]
    fn non_canonical() {
        for text in [
            "# Title only\n",
            "### Requirement: greeting\n\nBody without provenance.\n",
            "### Requirement: greeting [conflict]\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n",
        ] {
            let err = text.parse::<Spec>().expect_err(text);
            assert_eq!(err.code(), "server_error", "{text}");
        }
    }
}
