//! # Parse `spec.md`
//!
//! Models generate `spec.md`, so its shape is verified rather than trusted.
//! The parser splits the document into `### Requirement:` blocks, checks each
//! block's header and body, and rejects the whole document with one error
//! listing every finding, so a malformed spec is never committed or diffed.
//!
//! Synthesis parses the draft to check the model preserved the reconciliation
//! rows; the store parses both revisions to report the re-mine diff.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use omnia_guest::{Error, bad_request};

use crate::is_kebab;

const HEADING: &str = "### Requirement:";
const ID: &str = "ID";
const SOURCES: &str = "Sources";
const STATUS: &str = "Status";

/// A parsed spec with requirement blocks in document order.
#[derive(Debug, Clone)]
pub struct Spec {
    /// Requirement blocks in document order.
    pub requirements: Vec<Requirement>,
}

impl FromStr for Spec {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        let mut findings = Vec::new();
        let lines: Vec<Line<'_>> = text
            .lines()
            .enumerate()
            .map(|(i, text)| Line {
                no: i + 1,
                text: text.trim_end(),
            })
            .collect();

        // split the document into requirement blocks, each starting at a heading
        // and ending at the line before the next heading; any preamble is ignored
        let blocks: Vec<Result<Requirement, Vec<String>>> = lines
            .chunk_by(|_, next| !next.text.starts_with(HEADING))
            .filter_map(|block| {
                let first = block.first()?;
                let heading = first.text.strip_prefix(HEADING)?.trim();
                Some(Requirement::parse(first.no, heading, &block[1..]))
            })
            .collect();

        // check for at least one requirement block
        if blocks.is_empty() {
            findings.push(format!("the document carries no `{HEADING}` block"));
        }

        let mut requirements = Vec::new();
        for block in blocks {
            match block {
                Ok(requirement) => requirements.push(requirement),
                Err(issues) => findings.extend(issues),
            }
        }

        // check that the document carries no duplicate requirement ids
        let mut seen = BTreeSet::new();
        for duplicate in requirements.iter().filter(|r| !seen.insert(r.id.as_str())) {
            let id = &duplicate.id;
            findings.push(format!("duplicate requirement id `{id}`"));
        }

        // reject the document if there are any violations
        if !findings.is_empty() {
            let findings = findings.join(";\n");
            return Err(bad_request!("`spec.md` is malformed: {findings}"));
        }

        Ok(Self { requirements })
    }
}

impl Spec {
    // Requirement blocks keyed by heading subject; a repeated name keeps the
    // last block, since names are not checked for uniqueness.
    pub fn subjects(&self) -> BTreeMap<&str, &Requirement> {
        self.requirements.iter().map(|r| (r.name.as_str(), r)).collect()
    }
}

/// One fully parsed requirement block; the heading tag mirrors `status`.
#[derive(Debug, Clone)]
pub struct Requirement {
    /// The requirement id (`REQ-NNN`).
    pub id: String,
    /// The heading name with any inline tag stripped.
    pub name: String,
    /// Source keys; empty only for `Status: unknown`.
    pub sources: Vec<String>,
    /// The `Status:` value.
    pub status: Status,
    // Body text with blank edges trimmed.
    body: String,
}

impl Requirement {
    // Whether reviewable content matches, ignoring positional ids.
    pub fn same_as(&self, other: &Self) -> bool {
        self.status == other.status && self.sources == other.sources && self.body == other.body
    }

    // Parse one `### Requirement:` block (heading text plus the lines under
    // it) as far as it goes, so every finding is reported, not just the first.
    fn parse(line_no: usize, heading: &str, rest: &[Line<'_>]) -> Result<Self, Vec<String>> {
        let mut issues = Vec::new();

        // the name and the optional `[tag]`; an unknown tag reads as no tag
        let (name, token) = split_tag(heading);
        if name.is_empty() {
            issues.push(format!("line {line_no}: requirement heading has no name"));
        }
        let tag = token.and_then(|token| {
            let tag = token.parse::<Status>().ok().and_then(Status::tag);
            if tag.is_none() {
                issues.push(format!("line {line_no}: unknown heading tag `[{token}]`"));
            }
            tag
        });

        // header lines run until the first non-blank line that is not one;
        // everything from there on is body
        let mut fields = BTreeMap::new();
        let mut body = rest;
        while let [line, tail @ ..] = body {
            let text = line.text.trim();
            if !text.is_empty() {
                let Some((key @ (ID | SOURCES | STATUS), value)) = text.split_once(':') else {
                    break;
                };
                let field = Line {
                    no: line.no,
                    text: value.trim(),
                };
                if fields.insert(key, field).is_some() {
                    let no = line.no;
                    issues.push(format!("line {no}: duplicate `{key}:` line"));
                }
            }
            body = tail;
        }
        let body: Vec<&str> = body.iter().map(|line| line.text).collect();

        let mut take = |key: &str| {
            let field = fields.remove(key);
            if field.is_none() {
                issues.push(format!("line {line_no}: no `{key}:` line"));
            }
            field
        };
        let (id, sources, status) = (take(ID), take(SOURCES), take(STATUS));

        let id = id.map_or_else(String::new, |Line { no, text: id }| {
            if !is_req_id(id) {
                issues.push(format!("line {no}: malformed id `{id}` (expected `REQ-NNN`)"));
            }
            id.to_string()
        });
        let sources = sources.map_or_else(Vec::new, |Line { no, text: raw }| {
            let keys: Vec<String> = source_keys(raw).map(str::to_string).collect();
            for key in keys.iter().filter(|key| !is_kebab(key)) {
                issues.push(format!("line {no}: malformed source key `{key}`"));
            }
            keys
        });
        let Some(status) = status.and_then(|Line { no, text: raw }| {
            let status = raw.parse::<Status>().ok();
            if status.is_none() {
                issues.push(format!("line {no}: unknown `Status: {raw}`"));
            }
            status
        }) else {
            return Err(issues);
        };

        // `Sources: []` is legal exactly when `Status: unknown` — an
        // evidence-less requirement has no contributing source to cite.
        if sources.is_empty() && status != Status::Unknown {
            issues.push(format!("line {line_no}: empty `Sources:` but not `Status: unknown`"));
        }
        if tag != status.tag() {
            let found = tag.map_or_else(
                || "no heading tag".to_string(),
                |tag| format!("heading tag `[{tag}]`"),
            );
            issues.push(format!("line {line_no}: {found} does not mirror `Status: {status}`"));
        }
        if !issues.is_empty() {
            return Err(issues);
        }

        Ok(Self {
            id,
            name: name.to_string(),
            sources,
            status,
            body: body.join("\n").trim_matches('\n').to_string(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Line<'a> {
    no: usize,
    text: &'a str,
}

/// Closed requirement `Status:` vocabulary. Every status but `agreed`
/// doubles as the `[tag]` its heading must carry.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Status {
    /// One source, or multiple sources that agree.
    Agreed,
    /// No contributing evidence.
    Unknown,
    /// Tied top-authority disagreement; operator must reconcile.
    Conflict,
    /// Authority-resolved disagreement; loser is commentary.
    Divergence,
}

impl Status {
    /// The heading tag this status must pair with; `None` for `agreed`.
    pub const fn tag(self) -> Option<Self> {
        match self {
            Self::Agreed => None,
            tagged => Some(tagged),
        }
    }
}

impl FromStr for Status {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "agreed" => Ok(Self::Agreed),
            "unknown" => Ok(Self::Unknown),
            "conflict" => Ok(Self::Conflict),
            "divergence" => Ok(Self::Divergence),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Agreed => "agreed",
            Self::Unknown => "unknown",
            Self::Conflict => "conflict",
            Self::Divergence => "divergence",
        };
        f.write_str(text)
    }
}

// A trailing ` [token]` is split off the name; the caller decides what it means.
fn split_tag(heading: &str) -> (&str, Option<&str>) {
    heading
        .strip_suffix(']')
        .and_then(|inner| inner.rsplit_once(" ["))
        .map_or((heading, None), |(name, token)| (name.trim_end(), Some(token)))
}

// `[a, b]`; the brackets are optional so a bare list still reads.
fn source_keys(raw: &str) -> impl Iterator<Item = &str> {
    let inner = raw.strip_prefix('[').unwrap_or(raw);
    let inner = inner.strip_suffix(']').unwrap_or(inner);
    inner.split(',').map(str::trim).filter(|key| !key.is_empty())
}

fn is_req_id(id: &str) -> bool {
    id.strip_prefix("REQ-")
        .is_some_and(|tail| tail.len() == 3 && tail.bytes().all(|b| b.is_ascii_digit()))
}

// Collapse (dense private parse matrix): fail-closed spec AST edges
// are a closed (markdown → Spec / BadRequest) table; a root port
// would be one synthesis fixture per grammar finding.
#[cfg(test)]
mod tests {
    use super::{Spec, Status};

    const REVIEWABLE: &str = "\
# Session handling

Scope: the session lifecycle.

### Requirement: Sessions expire after inactivity [divergence]

ID: REQ-001
Sources: [intent, docs]
Status: divergence

Sessions must expire after 30 minutes of inactivity.

> [divergence] docs say 30 minutes; behaviour shows 15. Intent wins.

### Requirement: Session renewal on activity

ID: REQ-002
Sources: [docs]
Status: agreed

Activity within the window renews the session.

### Requirement: Concurrent session limit [unknown]

ID: REQ-003
Sources: []
Status: unknown

[unknown] No source states a concurrent-session limit.
";

    #[test]
    fn parses_correctly() {
        let spec: Spec = REVIEWABLE.parse().expect("the reviewable set parses");
        assert_eq!(spec.requirements.len(), 3);

        let first = &spec.requirements[0];
        assert_eq!(first.id, "REQ-001");
        assert_eq!(first.name, "Sessions expire after inactivity");
        assert_eq!(first.status, Status::Divergence);
        assert_eq!(first.sources, ["intent", "docs"]);
        assert!(first.body.starts_with("Sessions must expire"), "blank edges trimmed");
        assert!(first.body.contains("Intent wins"));

        let third = &spec.requirements[2];
        assert_eq!(third.status, Status::Unknown);
        assert!(third.sources.is_empty(), "unknown may cite no sources");
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
                "### Requirement: Bad key\n\nID: REQ-001\nSources: [Docs!]\nStatus: agreed\n\nBody.\n",
                "malformed source key `Docs!`",
            ),
            (
                "### Requirement: One\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nBody.\n\n\
             ### Requirement: Two\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nBody.\n",
                "duplicate requirement id `REQ-001`",
            ),
            (
                "### Requirement: Doubled\n\nID: REQ-001\nID: REQ-002\nSources: [a]\nStatus: agreed\n\nBody.\n",
                "duplicate `ID:` line",
            ),
        ];

        for (text, fragment) in cases {
            let err = text.parse::<Spec>().expect_err(fragment);
            assert_eq!(err.code(), "bad_request", "typed code for {fragment}");
            let message = err.description();
            assert!(message.contains(fragment), "expected `{fragment}` in: {message}");
        }
    }

    #[test]
    fn findings_aggregate() {
        let text = "### Requirement: Two faults [wip]\n\nID: REQ-1\nSources: [a]\n\nBody.\n";
        let message = text.parse::<Spec>().expect_err("two faults").description();
        for fragment in ["unknown heading tag `[wip]`", "malformed id `REQ-1`", "no `Status:` line"]
        {
            assert!(message.contains(fragment), "expected `{fragment}` in: {message}");
        }
    }
}
