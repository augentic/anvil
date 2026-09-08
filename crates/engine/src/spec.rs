//! # Parse `spec.md`
//!
//! Models generate `spec.md`, so its shape is verified rather than trusted.
//! A spec is a preamble followed by `### Requirement:` blocks — a heading,
//! three provenance lines, and a body with at least one scenario — and every
//! violation in the document is reported in one refusal, so a malformed spec
//! is never committed or diffed.
//!
//! Synthesis parses the draft to check the model preserved the reconciliation
//! rows; the store parses both revisions to report the re-mine diff.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display};
use std::str::FromStr;

use omnia_guest::{Error, bad_request};

use crate::is_kebab;

const HEADING: &str = "### Requirement:";
const SCENARIO: &str = "#### Scenario:";

/// A parsed `spec.md`.
#[derive(Debug)]
pub struct Spec {
    /// Requirement blocks in document order.
    pub requirements: Vec<Requirement>,
}

impl FromStr for Spec {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        let document = Document::from(text);
        let blocks = document.blocks();

        let mut findings = Findings::default();
        if blocks.is_empty() {
            findings.push(format!("the document carries no `{HEADING}` block"));
        }
        let requirements = blocks
            .iter()
            .filter_map(|block| findings.record(Requirement::try_from(block)))
            .collect();
        let spec = Self { requirements };
        findings.extend(spec.duplicates());

        findings.finish(spec).map_err(|findings| bad_request!("`spec.md` is malformed: {findings}"))
    }
}

impl Spec {
    /// Requirement blocks keyed by their stable reconciliation subject.
    #[must_use]
    pub fn by_subject(&self) -> BTreeMap<&str, &Requirement> {
        self.requirements
            .iter()
            .map(|requirement| (requirement.subject.as_str(), requirement))
            .collect()
    }

    // Ids and subjects each index the document, so both must be unique.
    fn duplicates(&self) -> Findings {
        let mut findings = Findings::default();
        let mut ids = BTreeSet::new();
        let mut subjects = BTreeSet::new();
        for requirement in &self.requirements {
            if !ids.insert(&requirement.id) {
                findings.push(format!("duplicate requirement id `{}`", requirement.id));
            }
            if !subjects.insert(&requirement.subject) {
                findings.push(format!("duplicate requirement subject `{}`", requirement.subject));
            }
        }
        findings
    }
}

/// One requirement block. The heading tag mirrors `status`, so only the
/// status is kept.
#[derive(Debug)]
pub struct Requirement {
    /// The positional id.
    pub id: ReqId,
    /// The heading name: the reconciliation subject.
    pub subject: String,
    /// The cited source keys; empty only for `Status: unknown`.
    pub sources: Sources,
    /// The `Status:` value.
    pub status: Status,
    // The text below the provenance lines, blank edges trimmed.
    body: String,
}

impl TryFrom<&Block<'_>> for Requirement {
    type Error = Findings;

    // Every fault in the block is reported, not just the first.
    fn try_from(block: &Block<'_>) -> Result<Self, Findings> {
        let mut findings = Findings::default();

        let heading = findings.record(Heading::try_from(block.heading));
        let id = findings.record(block.provenance.id());
        let sources = findings.record(block.provenance.sources());
        let status = findings.record(block.provenance.status());
        findings.record(block.scenario());

        let (Some(heading), Some(id), Some(sources), Some(status)) = (heading, id, sources, status)
        else {
            return Err(findings);
        };
        findings.record(heading.mirrors(status));
        findings.record(block.provenance.evidenced(&sources, status));

        findings.finish(Self {
            id,
            subject: heading.subject.to_string(),
            sources,
            status,
            body: block.body.text(),
        })
    }
}

// The same requirement in two revisions differs in nothing but its
// positional id, which shifts whenever a row is added or removed above it.
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
    type Err = Malformed;

    fn from_str(text: &str) -> Result<Self, Malformed> {
        let digits = text.strip_prefix(Self::PREFIX).unwrap_or_default();
        if digits.len() == 3 && digits.bytes().all(|byte| byte.is_ascii_digit()) {
            Ok(Self(text.to_string()))
        } else {
            Err(Malformed(text.to_string()))
        }
    }
}

impl Display for ReqId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The `Sources:` list: cited source keys in binding order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sources(Vec<String>);

impl Sources {
    /// The cited keys, in order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    /// Whether no source is cited.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromStr for Sources {
    type Err = MalformedSources;

    // `[a, b]`: bracketed, every key kebab-case.
    fn from_str(text: &str) -> Result<Self, MalformedSources> {
        let inner = text
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
            .ok_or_else(|| MalformedSources::List(Malformed(text.to_string())))?;
        let (keys, malformed): (Vec<&str>, Vec<&str>) = inner
            .split(',')
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .partition(|key| is_kebab(key));
        if !malformed.is_empty() {
            let keys = malformed.into_iter().map(|key| Malformed(key.to_string())).collect();
            return Err(MalformedSources::Keys(keys));
        }
        Ok(Self(keys.into_iter().map(str::to_string).collect()))
    }
}

/// A `Sources:` value that does not fit its grammar.
#[derive(Debug)]
pub enum MalformedSources {
    /// Not a bracketed list, quoted as written.
    List(Malformed),
    /// Every key that is not kebab-case.
    Keys(Vec<Malformed>),
}

/// The closed `Status:` vocabulary.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Status {
    /// One source, or multiple sources that agree.
    Agreed,
    /// No contributing evidence.
    Unknown,
    /// Tied top-authority disagreement; the operator must reconcile.
    Conflict,
    /// Authority-resolved disagreement; the loser is commentary.
    Divergence,
}

impl Status {
    /// The heading tag this status must pair with; `agreed` carries none.
    #[must_use]
    pub const fn tag(self) -> Option<Tag> {
        match self {
            Self::Agreed => None,
            Self::Unknown => Some(Tag::Unknown),
            Self::Conflict => Some(Tag::Conflict),
            Self::Divergence => Some(Tag::Divergence),
        }
    }
}

impl From<Tag> for Status {
    fn from(tag: Tag) -> Self {
        match tag {
            Tag::Unknown => Self::Unknown,
            Tag::Conflict => Self::Conflict,
            Tag::Divergence => Self::Divergence,
        }
    }
}

impl FromStr for Status {
    type Err = Malformed;

    fn from_str(text: &str) -> Result<Self, Malformed> {
        match text {
            "agreed" => Ok(Self::Agreed),
            "unknown" => Ok(Self::Unknown),
            "conflict" => Ok(Self::Conflict),
            "divergence" => Ok(Self::Divergence),
            _ => Err(Malformed(text.to_string())),
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Agreed => "agreed",
            Self::Unknown => "unknown",
            Self::Conflict => "conflict",
            Self::Divergence => "divergence",
        })
    }
}

/// The closed heading `[tag]` vocabulary: every status but `agreed`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Tag {
    /// Mirrors `Status: unknown`.
    Unknown,
    /// Mirrors `Status: conflict`.
    Conflict,
    /// Mirrors `Status: divergence`.
    Divergence,
}

// A tag is spelled as the status it mirrors.
impl FromStr for Tag {
    type Err = Malformed;

    fn from_str(text: &str) -> Result<Self, Malformed> {
        text.parse::<Status>()?.tag().ok_or_else(|| Malformed(text.to_string()))
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&Status::from(*self), f)
    }
}

/// A value that does not fit its grammar, quoted as written.
#[derive(Debug)]
pub struct Malformed(String);

// `spec.md` as numbered lines.
#[derive(Debug)]
struct Document<'a> {
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
    // Every run of lines led by a heading; the preamble is skipped.
    fn blocks(&'a self) -> Vec<Block<'a>> {
        self.lines.chunk_by(|_, next| next.heading().is_none()).filter_map(Block::new).collect()
    }
}

// One `### Requirement:` block: the heading line less its marker, the
// provenance lines beneath it, then the body.
#[derive(Debug)]
struct Block<'a> {
    heading: Line<'a>,
    provenance: Provenance<'a>,
    body: Lines<'a>,
}

impl<'a> Block<'a> {
    // `None` for the preamble, the one run of lines not led by a heading.
    fn new(run: &'a [Line<'a>]) -> Option<Self> {
        let [heading, rest @ ..] = run else { return None };
        let heading = heading.heading()?;

        // Provenance ends at the first non-blank line that is not a field.
        let split =
            rest.iter().take_while(|line| line.is_blank() || line.field().is_some()).count();
        let (header, body) = rest.split_at(split);
        Some(Self {
            heading,
            provenance: Provenance {
                heading,
                fields: Lines(header).fields(),
            },
            body: Lines(body),
        })
    }

    // Every requirement carries at least one scenario.
    fn scenario(&self) -> Result<(), Finding> {
        if self.body.has_scenario() {
            return Ok(());
        }
        Err(self.heading.fault(format!("no `{SCENARIO}` heading")))
    }
}

// `<subject>[ [<tag>]]`: a heading line after its marker.
#[derive(Debug)]
struct Heading<'a> {
    subject: &'a str,
    tag: Option<Tag>,
    line: Line<'a>,
}

impl<'a> TryFrom<Line<'a>> for Heading<'a> {
    type Error = Finding;

    fn try_from(line: Line<'a>) -> Result<Self, Finding> {
        let (subject, token) = line
            .text
            .strip_suffix(']')
            .and_then(|inner| inner.rsplit_once(" ["))
            .map_or((line.text, None), |(subject, token)| (subject.trim_end(), Some(token)));
        if subject.is_empty() {
            return Err(line.fault("requirement heading has no name"));
        }
        let tag = token
            .map(str::parse::<Tag>)
            .transpose()
            .map_err(|Malformed(token)| line.fault(format!("unknown heading tag `[{token}]`")))?;

        Ok(Self { subject, tag, line })
    }
}

impl Heading<'_> {
    // The tag must mirror `Status:`; `agreed` carries none.
    fn mirrors(&self, status: Status) -> Result<(), Finding> {
        if self.tag == status.tag() {
            return Ok(());
        }
        let found = self
            .tag
            .map_or_else(|| "no heading tag".to_string(), |tag| format!("heading tag `[{tag}]`"));
        Err(self.line.fault(format!("{found} does not mirror `Status: {status}`")))
    }
}

// The provenance lines under a heading: `ID:`, `Sources:`, `Status:`.
#[derive(Debug)]
struct Provenance<'a> {
    heading: Line<'a>,
    fields: Vec<Field<'a>>,
}

impl<'a> Provenance<'a> {
    fn id(&self) -> Result<ReqId, Finding> {
        let field = self.field(Key::Id)?;
        field.value.parse::<ReqId>().map_err(|Malformed(id)| {
            field.line.fault(format!("malformed id `{id}` (expected `REQ-NNN`)"))
        })
    }

    // A list fault is one finding; a key fault names every malformed key.
    fn sources(&self) -> Result<Sources, Findings> {
        let field = self.field(Key::Sources)?;
        field.value.parse::<Sources>().map_err(|malformed| match malformed {
            MalformedSources::List(Malformed(list)) => field
                .line
                .fault(format!("malformed `Sources: {list}` (expected `[<source>, …]`)"))
                .into(),
            MalformedSources::Keys(keys) => keys
                .into_iter()
                .map(|Malformed(key)| field.line.fault(format!("malformed source key `{key}`")))
                .collect(),
        })
    }

    fn status(&self) -> Result<Status, Finding> {
        let field = self.field(Key::Status)?;
        field
            .value
            .parse::<Status>()
            .map_err(|Malformed(status)| field.line.fault(format!("unknown `Status: {status}`")))
    }

    // `Sources: []` is legal exactly when `Status: unknown`; an
    // evidence-less requirement has no contributing source to cite.
    fn evidenced(&self, sources: &Sources, status: Status) -> Result<(), Finding> {
        if sources.is_empty() && status != Status::Unknown {
            return Err(self.heading.fault("empty `Sources:` but not `Status: unknown`"));
        }
        Ok(())
    }

    // Exactly one line per key.
    fn field(&self, key: Key) -> Result<Field<'a>, Finding> {
        let matching: Vec<Field<'a>> =
            self.fields.iter().copied().filter(|field| field.key == key).collect();
        match matching.as_slice() {
            [] => Err(self.heading.fault(format!("no `{key}:` line"))),
            [field] => Ok(*field),
            [_, duplicate, ..] => Err(duplicate.line.fault(format!("duplicate `{key}:` line"))),
        }
    }
}

// One provenance line, `<Key>: <value>`.
#[derive(Debug, Clone, Copy)]
struct Field<'a> {
    key: Key,
    value: &'a str,
    line: Line<'a>,
}

// The provenance keys, matched exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Key {
    Id,
    Sources,
    Status,
}

impl FromStr for Key {
    type Err = Malformed;

    fn from_str(text: &str) -> Result<Self, Malformed> {
        match text {
            "ID" => Ok(Self::Id),
            "Sources" => Ok(Self::Sources),
            "Status" => Ok(Self::Status),
            _ => Err(Malformed(text.to_string())),
        }
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Id => "ID",
            Self::Sources => "Sources",
            Self::Status => "Status",
        })
    }
}

// A run of lines.
#[derive(Debug, Clone, Copy)]
struct Lines<'a>(&'a [Line<'a>]);

impl<'a> Lines<'a> {
    // Joined text with blank edges trimmed.
    fn text(self) -> String {
        let text: Vec<&str> = self.0.iter().map(|line| line.text).collect();
        text.join("\n").trim_matches('\n').to_string()
    }

    fn has_scenario(self) -> bool {
        self.0.iter().any(|line| line.is_scenario())
    }

    fn fields(self) -> Vec<Field<'a>> {
        self.0.iter().copied().filter_map(Line::field).collect()
    }
}

// One numbered line, right-trimmed.
#[derive(Debug, Clone, Copy)]
struct Line<'a> {
    no: usize,
    text: &'a str,
}

impl<'a> Line<'a> {
    const fn is_blank(self) -> bool {
        self.text.is_empty()
    }

    // The heading text after `### Requirement:`; `None` for any other line.
    fn heading(self) -> Option<Self> {
        let text = self.text.strip_prefix(HEADING)?.trim();
        Some(Self { text, ..self })
    }

    fn is_scenario(self) -> bool {
        self.text.starts_with(SCENARIO)
    }

    // `<Key>: <value>` for one of the provenance keys.
    fn field(self) -> Option<Field<'a>> {
        let (key, value) = self.text.trim().split_once(':')?;
        Some(Field {
            key: key.parse().ok()?,
            value: value.trim(),
            line: self,
        })
    }

    fn fault(self, detail: impl Display) -> Finding {
        Finding(format!("line {}: {detail}", self.no))
    }
}

// Every violation in one document, refused together.
#[derive(Debug, Default)]
struct Findings(Vec<Finding>);

impl Findings {
    // A violation with no line of its own.
    fn push(&mut self, detail: impl Display) {
        self.0.push(Finding(detail.to_string()));
    }

    fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    // Keeps the parsed part and files its fault, so parsing continues to
    // the end of the document.
    fn record<T>(&mut self, result: Result<T, impl Into<Self>>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(fault) => {
                self.extend(fault.into());
                None
            }
        }
    }

    // The parsed value, unless anything was found.
    fn finish<T>(self, value: T) -> Result<T, Self> {
        if self.0.is_empty() { Ok(value) } else { Err(self) }
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

// One grammar violation, as the operator reads it.
#[derive(Debug)]
struct Finding(String);

// `tests/specify.rs` owns what the operator observes — the typed refusal
// that commits nothing, and the re-mine diff — over representative breaches.
// The parser is private and its grammar edges are a case-per-cell explosion
// at that entry point, so the per-fault messages stay here.
#[cfg(test)]
mod tests {
    use super::Spec;

    // Nothing outside the parser reads `body`; every public field is proven
    // through the product, which refuses a misread row.
    #[test]
    fn body_is_the_text_alone() {
        let text = "\
# Preamble

### Requirement: Sessions expire [divergence]

ID: REQ-001
Sources: [intent, docs]
Status: divergence

Sessions expire after 30 minutes.

> [divergence] docs say 30; behaviour shows 15. Intent wins.

#### Scenario: Session expires

- **WHEN** a session is idle for 30 minutes
- **THEN** it expires

";
        let spec: Spec = text.parse().expect("one block parses");
        assert_eq!(spec.requirements.len(), 1);
        assert_eq!(
            spec.requirements[0].body,
            "Sessions expire after 30 minutes.\n\n\
             > [divergence] docs say 30; behaviour shows 15. Intent wins.\n\n\
             #### Scenario: Session expires\n\n\
             - **WHEN** a session is idle for 30 minutes\n\
             - **THEN** it expires",
            "provenance lines and blank edges are not body"
        );
    }

    #[test]
    fn violations_fail() {
        let cases: &[(&str, &str)] = &[
            ("# Title only, no blocks\n", "no `### Requirement:` block"),
            (
                "### Requirement:\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nBody.\n",
                "requirement heading has no name",
            ),
            ("### Requirement: No id\n\nSources: [a]\nStatus: agreed\n\nBody.\n", "no `ID:` line"),
            (
                "### Requirement: Bad id\n\nID: REQ-1\nSources: [a]\nStatus: agreed\n\nBody.\n",
                "malformed id `REQ-1`",
            ),
            (
                "### Requirement: No sources\n\nID: REQ-001\nStatus: agreed\n\nBody.\n",
                "no `Sources:` line",
            ),
            (
                "### Requirement: No status\n\nID: REQ-001\nSources: [a]\n\nBody.\n",
                "no `Status:` line",
            ),
            (
                "### Requirement: Bad status\n\nID: REQ-001\nSources: [a]\nStatus: resolved\n\nBody.\n",
                "unknown `Status: resolved`",
            ),
            (
                "### Requirement: Untagged conflict\n\nID: REQ-001\nSources: [a, b]\nStatus: conflict\n\nBody.\n",
                "no heading tag does not mirror `Status: conflict`",
            ),
            (
                "### Requirement: Mistagged [conflict]\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nBody.\n",
                "heading tag `[conflict]` does not mirror `Status: agreed`",
            ),
            (
                "### Requirement: Stray tag [wip]\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nBody.\n",
                "unknown heading tag `[wip]`",
            ),
            (
                "### Requirement: Agreed is untagged [agreed]\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nBody.\n",
                "unknown heading tag `[agreed]`",
            ),
            (
                "### Requirement: Evidence-less but agreed\n\nID: REQ-001\nSources: []\nStatus: agreed\n\nBody.\n",
                "empty `Sources:` but not `Status: unknown`",
            ),
            (
                "### Requirement: Unbracketed\n\nID: REQ-001\nSources: a, b\nStatus: agreed\n\nBody.\n",
                "malformed `Sources: a, b` (expected `[<source>, …]`)",
            ),
            (
                "### Requirement: Bad key\n\nID: REQ-001\nSources: [Docs!]\nStatus: agreed\n\nBody.\n",
                "malformed source key `Docs!`",
            ),
            (
                "### Requirement: One\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\n\
                 Body.\n\n#### Scenario: One\n\n- **WHEN** one\n- **THEN** one\n\n\
                 ### Requirement: Two\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\n\
                 Body.\n\n#### Scenario: Two\n\n- **WHEN** two\n- **THEN** two\n",
                "duplicate requirement id `REQ-001`",
            ),
            (
                "### Requirement: Same\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\n\
                 Body.\n\n#### Scenario: One\n\n- **WHEN** one\n- **THEN** one\n\n\
                 ### Requirement: Same\n\nID: REQ-002\nSources: [a]\nStatus: agreed\n\n\
                 Body.\n\n#### Scenario: Two\n\n- **WHEN** two\n- **THEN** two\n",
                "duplicate requirement subject `Same`",
            ),
            (
                "### Requirement: Doubled\n\nID: REQ-001\nID: REQ-002\nSources: [a]\nStatus: agreed\n\nBody.\n",
                "duplicate `ID:` line",
            ),
            (
                "### Requirement: No scenario\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nBody.\n",
                "no `#### Scenario:` heading",
            ),
        ];

        for (text, fragment) in cases {
            let message = text.parse::<Spec>().expect_err(fragment).description();
            assert!(message.contains(fragment), "expected `{fragment}` in: {message}");
        }
    }

    // Every fault is reported at once; the rules that relate two parts wait
    // until both parsed, so a missing part is reported exactly once.
    #[test]
    fn findings_aggregate() {
        let text = "### Requirement: Two faults [wip]\n\nID: REQ-1\nSources: [a, B, c!]\n\nBody.\n";
        let message = text.parse::<Spec>().expect_err("several faults").description();
        for fragment in [
            "unknown heading tag `[wip]`",
            "malformed id `REQ-1`",
            "malformed source key `B`",
            "malformed source key `c!`",
            "no `Status:` line",
        ] {
            assert!(message.contains(fragment), "expected `{fragment}` in: {message}");
        }
        assert!(!message.contains("does not mirror"), "the tag rule waits for a status: {message}");
        assert!(
            !message.contains("empty `Sources:`"),
            "the sources rule waits for both: {message}"
        );
    }
}
