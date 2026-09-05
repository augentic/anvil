//! Fail-closed `spec.md` AST.

use omnia_guest::{Error, bad_request};

/// Markdown heading prefix opening a requirement block.
pub const HEADING: &str = "### Requirement:";

/// Closed requirement `Status:` vocabulary.
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

/// Heading tag required for every non-`agreed` status.
#[derive(Debug, Copy, Clone, PartialEq, Eq, strum::Display, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum Tag {
    /// `[unknown]`.
    Unknown,
    /// `[conflict]`.
    Conflict,
    /// `[divergence]`.
    Divergence,
}

impl Tag {
    /// The `Status:` value this tag must pair with.
    #[must_use]
    pub const fn expected_status(self) -> Status {
        match self {
            Self::Unknown => Status::Unknown,
            Self::Conflict => Status::Conflict,
            Self::Divergence => Status::Divergence,
        }
    }
}

/// A parsed spec: the preamble and requirement blocks in document order.
#[derive(Debug, Clone)]
pub struct Spec {
    /// Text before the first requirement heading.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "parsed for the AST; consumers walk requirements")
    )]
    pub preamble: String,
    /// Requirement blocks in document order.
    pub requirements: Vec<Requirement>,
}

/// One fully parsed requirement block.
#[derive(Debug, Clone)]
pub struct Requirement {
    /// The requirement id (`REQ-NNN`).
    pub id: String,
    /// The heading name with any inline tag stripped.
    pub name: String,
    /// The inline heading tag; `None` exactly when `Status: agreed`.
    pub tag: Option<Tag>,
    /// Source keys; empty only for `Status: unknown`.
    pub sources: Vec<String>,
    /// The `Status:` value.
    pub status: Status,
    /// Body text with blank edges trimmed.
    pub body: String,
}

/// Parse `text` under the fail-closed grammar.
///
/// # Errors
///
/// Returns one `BadRequest` aggregating all grammar findings.
pub fn parse(text: &str) -> Result<Spec, Error> {
    let mut findings: Vec<String> = Vec::new();
    let mut requirements: Vec<Requirement> = Vec::new();
    let mut preamble: Vec<&str> = Vec::new();
    let mut block: Option<Block> = None;

    for (idx, line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let stripped = line.trim_end();
        if let Some(rest) = stripped.strip_prefix(HEADING) {
            if let Some(done) = block.take() {
                done.finish(&mut requirements, &mut findings);
            }
            block = Some(Block::open(rest.trim(), line_no, &mut findings));
        } else if let Some(open) = block.as_mut() {
            open.line(stripped, line_no, &mut findings);
        } else {
            preamble.push(stripped);
        }
    }
    if let Some(done) = block.take() {
        done.finish(&mut requirements, &mut findings);
    }

    if requirements.is_empty() {
        findings.push(format!("the document carries no `{HEADING}` block"));
    }
    let mut seen: Vec<&str> = Vec::new();
    for requirement in &requirements {
        if seen.contains(&requirement.id.as_str()) {
            findings.push(format!("duplicate requirement id `{}`", requirement.id));
        }
        seen.push(&requirement.id);
    }

    if findings.is_empty() {
        Ok(Spec {
            preamble: preamble.join("\n"),
            requirements,
        })
    } else {
        Err(bad_request!(
            "`spec.md` must parse under the fail-closed spec AST: {}",
            findings.join("; ")
        ))
    }
}

// Metadata must precede the first body line.
struct Block {
    line_no: usize,
    name: String,
    tag: Option<Tag>,
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

    fn line(&mut self, stripped: &str, line_no: usize, findings: &mut Vec<String>) {
        let trimmed = stripped.trim();
        if self.in_metadata {
            if trimmed.is_empty() {
                return;
            }
            if let Some(rest) = trimmed.strip_prefix("ID:") {
                set_once(&mut self.id, rest.trim().to_string(), "ID:", line_no, findings);
                return;
            }
            if let Some(rest) = trimmed.strip_prefix("Sources:") {
                let keys = parse_sources(rest, line_no, findings);
                set_once(&mut self.sources, keys, "Sources:", line_no, findings);
                return;
            }
            if let Some(rest) = trimmed.strip_prefix("Status:") {
                let raw = rest.trim();
                match raw.parse::<Status>() {
                    Ok(status) => {
                        set_once(&mut self.status, status, "Status:", line_no, findings);
                    }
                    Err(_) => findings.push(format!(
                        "line {line_no}: unrecognised `Status: {raw}` (one of `agreed | unknown | conflict | divergence`)"
                    )),
                }
                return;
            }
            self.in_metadata = false;
        }
        self.body.push(stripped.to_string());
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
            Some(tag) if tag.expected_status() != status => findings.push(format!(
                "{subject}: heading tag `[{tag}]` disagrees with `Status: {status}`"
            )),
            None if status != Status::Agreed => findings.push(format!(
                "{subject}: `Status: {status}` without the `[{status}]` heading tag"
            )),
            _ => {}
        }

        requirements.push(Requirement {
            id,
            name,
            tag,
            sources,
            status,
            body: trim_edges(&body),
        });
    }
}

// An unknown trailing `[tag]` is a finding, not part of the name.
fn split_tag(heading: &str, line_no: usize, findings: &mut Vec<String>) -> (String, Option<Tag>) {
    if let Some(open) = heading.rfind(" [")
        && heading.ends_with(']')
    {
        let token = &heading[open + 2..heading.len() - 1];
        let tag = token.parse::<Tag>().ok();
        if tag.is_none() {
            findings.push(format!("line {line_no}: unrecognised heading tag `[{token}]`"));
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
        if !is_source_key(key) {
            findings.push(format!("line {line_no}: malformed source key `{key}`"));
        }
    }
    keys
}

fn is_req_id(id: &str) -> bool {
    id.strip_prefix("REQ-")
        .is_some_and(|tail| tail.len() == 3 && tail.bytes().all(|b| b.is_ascii_digit()))
}

// Kebab-case source-key grammar: `[a-z][a-z0-9-]*`, no doubled or
// trailing dash.
fn is_source_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    let Some(first) = bytes.next() else { return false };
    if !first.is_ascii_lowercase() {
        return false;
    }
    let mut prev_dash = false;
    for byte in bytes {
        if byte == b'-' {
            if prev_dash {
                return false;
            }
            prev_dash = true;
        } else if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            prev_dash = false;
        } else {
            return false;
        }
    }
    !prev_dash
}

fn trim_edges(lines: &[String]) -> String {
    let mut start = 0;
    let mut end = lines.len();
    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    lines[start..end].join("\n")
}

// Collapse (dense private parse matrix): fail-closed spec AST edges
// are a closed (markdown → Spec / BadRequest) table; a root port
// would be one synthesis fixture per grammar finding.
#[cfg(test)]
mod tests {
    use super::{Status, Tag, parse};

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
        let spec = parse(REVIEWABLE).expect("the reviewable set parses");
        assert!(spec.preamble.starts_with("# Session handling"));
        assert_eq!(spec.requirements.len(), 3);

        let first = &spec.requirements[0];
        assert_eq!(first.id, "REQ-001");
        assert_eq!(first.name, "Sessions expire after inactivity");
        assert_eq!(first.tag, Some(Tag::Divergence));
        assert_eq!(first.status, Status::Divergence);
        assert_eq!(first.sources, ["intent", "docs"]);
        assert!(first.body.contains("Intent wins"));

        let third = &spec.requirements[2];
        assert_eq!(third.tag, Some(Tag::Unknown));
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
                "unrecognised `Status: resolved`",
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
                "unrecognised heading tag `[wip]`",
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
            let err = parse(text).expect_err(fragment);
            assert_eq!(err.code(), "bad_request", "typed code for {fragment}");
            let message = err.description();
            assert!(message.contains(fragment), "expected `{fragment}` in: {message}");
        }
    }
}
