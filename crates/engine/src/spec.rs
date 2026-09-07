//! Parsed `spec.md` requirements
//!
//! Parses `spec.md` into requirement blocks, each of which must carry:
//!
//! - a `### Requirement:` heading whose `[tag]` mirrors its `Status:`
//! - an `ID:` / `Sources:` / `Status:` header
//! - a body
//!
//! Parsing collects every finding and rejects the whole document with one
//! error, so a malformed spec is never committed or diffed.
//!
//! Models write `spec.md`, so the engine cannot entirely trust it. Synthesis parses
//! the draft to check the model preserved the reconciliation rows, and the store
//! parses both revisions to report the re-mine diff.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use omnia_guest::{Error, bad_request};

use crate::is_kebab;

// Markdown heading prefix opening a requirement block.
const HEADING: &str = "### Requirement:";

/// A parsed spec: the requirement blocks in document order.
#[derive(Debug, Clone)]
pub struct Spec {
    /// Requirement blocks in document order.
    pub requirements: Vec<Requirement>,
}

impl FromStr for Spec {
    type Err = Error;

    /// Parses `text` under the fail-closed grammar.
    ///
    /// # Errors
    ///
    /// Returns one `BadRequest` aggregating all grammar findings.
    fn from_str(text: &str) -> Result<Self, Error> {
        let mut findings: Vec<String> = Vec::new();
        let mut requirements: Vec<Requirement> = Vec::new();
        let mut block: Option<Block> = None;

        // Text before the first heading is preamble, carried by the
        // document bytes alone.
        for (idx, line) in text.lines().enumerate() {
            let line_no = idx + 1;
            let line = line.trim_end();
            if let Some(heading) = line.strip_prefix(HEADING) {
                if let Some(done) = block.take() {
                    done.finish(&mut requirements, &mut findings);
                }
                block = Some(Block::open(heading.trim(), line_no, &mut findings));
            } else if let Some(open) = block.as_mut() {
                open.line(line, line_no, &mut findings);
            }
        }
        if let Some(done) = block.take() {
            done.finish(&mut requirements, &mut findings);
        }

        if requirements.is_empty() {
            findings.push(format!("the document carries no `{HEADING}` block"));
        }
        let mut seen = BTreeSet::new();
        for requirement in &requirements {
            if !seen.insert(requirement.id.as_str()) {
                let id = &requirement.id;
                findings.push(format!("duplicate requirement id `{id}`"));
            }
        }

        if findings.is_empty() {
            Ok(Self { requirements })
        } else {
            let findings = findings.join("; ");
            Err(bad_request!("`spec.md` is malformed: {findings}"))
        }
    }
}

impl Spec {
    // Requirement blocks keyed by heading subject.
    pub fn subjects(&self) -> BTreeMap<&str, &Requirement> {
        self.requirements
            .iter()
            .map(|requirement| (requirement.name.as_str(), requirement))
            .collect()
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
}

/// Closed requirement `Status:` vocabulary. Every status but `agreed`
/// doubles as the `[tag]` its heading must carry.
#[derive(Debug, Copy, Clone, PartialEq, Eq, strum::Display, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
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

// Metadata must precede the first body line.
struct Block {
    line_no: usize,
    name: String,
    tag: Option<Status>,
    id: Option<String>,
    sources: Option<Vec<String>>,
    status: Option<Status>,
    body: Vec<String>,
    in_metadata: bool,
}

impl Block {
    fn open(heading: &str, line_no: usize, findings: &mut Vec<String>) -> Self {
        let (name, tag) = split_tag(heading, line_no, findings);
        if name.is_empty() {
            findings.push(format!("line {line_no}: requirement heading has no name"));
        }
        Self {
            line_no,
            name,
            tag,
            id: None,
            sources: None,
            status: None,
            body: Vec::new(),
            in_metadata: true,
        }
    }

    fn line(&mut self, line: &str, line_no: usize, findings: &mut Vec<String>) {
        if self.in_metadata {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return;
            }
            match trimmed.split_once(':') {
                Some(("ID", rest)) => {
                    set_once(&mut self.id, rest.trim().to_string(), "ID:", line_no, findings);
                    return;
                }
                Some(("Sources", rest)) => {
                    let keys = parse_sources(rest, line_no, findings);
                    set_once(&mut self.sources, keys, "Sources:", line_no, findings);
                    return;
                }
                Some(("Status", rest)) => {
                    let raw = rest.trim();
                    match raw.parse::<Status>() {
                        Ok(status) => {
                            set_once(&mut self.status, status, "Status:", line_no, findings);
                        }
                        Err(_) => findings.push(format!(
                            "line {line_no}: unknown `Status: {raw}` (expected agreed, unknown, conflict, or divergence)"
                        )),
                    }
                    return;
                }
                _ => self.in_metadata = false,
            }
        }
        self.body.push(line.to_string());
    }

    fn finish(self, requirements: &mut Vec<Requirement>, findings: &mut Vec<String>) {
        let Self {
            line_no,
            name,
            tag,
            id,
            sources,
            status,
            body,
            ..
        } = self;
        let subject = format!("requirement at line {line_no}");

        let id = id.unwrap_or_else(|| {
            findings.push(format!("{subject}: no `ID:` line"));
            String::new()
        });
        if !id.is_empty() && !is_req_id(&id) {
            findings.push(format!("{subject}: malformed id `{id}` (expected `REQ-NNN`)"));
        }
        let sources = sources.unwrap_or_else(|| {
            findings.push(format!("{subject}: no `Sources:` line"));
            Vec::new()
        });
        let Some(status) = status else {
            findings.push(format!("{subject}: no `Status:` line"));
            return;
        };

        // `Sources: []` is legal exactly when `Status: unknown` — an
        // evidence-less requirement has no contributing source to cite.
        if sources.is_empty() && status != Status::Unknown {
            findings.push(format!("{subject}: empty `Sources:` but not `Status: unknown`"));
        }
        match tag {
            Some(tag) if tag != status => findings.push(format!(
                "{subject}: heading tag `[{tag}]` disagrees with `Status: {status}`"
            )),
            None if status != Status::Agreed => findings.push(format!(
                "{subject}: `Status: {status}` without the `[{status}]` heading tag"
            )),
            _ => {}
        }

        // Lines are already right-trimmed, so blank edges are bare newlines.
        requirements.push(Requirement {
            id,
            name,
            sources,
            status,
            body: body.join("\n").trim_matches('\n').to_string(),
        });
    }
}

// A trailing `[tag]` that names no tagged status is a finding, not
// part of the name.
fn split_tag(
    heading: &str, line_no: usize, findings: &mut Vec<String>,
) -> (String, Option<Status>) {
    if let Some(open) = heading.rfind(" [")
        && heading.ends_with(']')
    {
        let token = &heading[open + 2..heading.len() - 1];
        let tag = token.parse::<Status>().ok().and_then(Status::tag);
        if tag.is_none() {
            findings.push(format!("line {line_no}: unknown heading tag `[{token}]`"));
        }
        return (heading[..open].trim_end().to_string(), tag);
    }
    (heading.to_string(), None)
}

fn set_once<T>(
    slot: &mut Option<T>, value: T, label: &str, line_no: usize, findings: &mut Vec<String>,
) {
    if slot.is_some() {
        findings.push(format!("line {line_no}: duplicate `{label}` line"));
    }
    *slot = Some(value);
}

fn parse_sources(rest: &str, line_no: usize, findings: &mut Vec<String>) -> Vec<String> {
    let trimmed = rest.trim();
    let inner = trimmed.strip_prefix('[').map_or(trimmed, str::trim_start);
    let inner = inner.strip_suffix(']').map_or(inner, str::trim_end);
    let keys: Vec<String> =
        inner.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect();
    for key in &keys {
        if !is_kebab(key) {
            findings.push(format!("line {line_no}: malformed source key `{key}`"));
        }
    }
    keys
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
    fn reviewable_set_parses() {
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
    fn violations_fail_typed() {
        let cases: &[(&str, &str)] = &[
            ("# Title only, no blocks\n", "no `### Requirement:` block"),
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
                "without the `[conflict]` heading tag",
            ),
            (
                "### Requirement: Mistagged [conflict]\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nBody.\n",
                "disagrees with `Status: agreed`",
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
}
