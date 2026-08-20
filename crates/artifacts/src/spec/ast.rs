//! The one fail-closed spec AST: the engine's load gate over
//! `spec.md`. Unparseable is a typed error, never a lenient pass.

use emery_error::Error;
use serde::{Deserialize, Serialize};

/// Markdown heading prefix opening a requirement block.
pub const HEADING: &str = "### Requirement:";

/// Closed `Status:` vocabulary for a requirement's provenance lines.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    strum::IntoStaticStr,
    schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
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

/// Inline heading tag paired with every non-`agreed` status —
/// honesty stays inline, nothing auto-defers.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, strum::Display, strum::EnumString, strum::IntoStaticStr,
)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    /// Text before the first requirement heading.
    pub preamble: String,
    /// Requirement blocks in document order.
    pub requirements: Vec<Requirement>,
}

/// One requirement block; every field is present or [`parse`] failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// The requirement id (`REQ-NNN`).
    pub id: String,
    /// The heading name with any inline tag stripped.
    pub name: String,
    /// The inline heading tag; `None` exactly when `Status: agreed`.
    pub tag: Option<Tag>,
    /// Source keys from the `Sources:` line; empty only with
    /// `Status: unknown`.
    pub sources: Vec<String>,
    /// The `Status:` value.
    pub status: Status,
    /// Body text below the metadata lines, blank edges trimmed.
    pub body: String,
}

/// Parse `text` as `spec.md` under the fail-closed AST.
///
/// # Errors
///
/// One `spec-invalid` validation error aggregating every finding: a
/// missing or malformed `ID:` / `Sources:` / `Status:` line, a
/// duplicate id, an unrecognised heading tag, tag–status incoherence,
/// or a document with no requirement blocks.
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
        Err(Error::validation_failed(
            "spec-invalid",
            "`spec.md` must parse under the fail-closed spec AST",
            findings.join("; "),
        ))
    }
}

// One requirement block being accumulated by the strict line scan:
// metadata lines first, then body once a non-metadata line appears.
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

// Split a trailing ` [tag]` off the heading. An unrecognised bracket
// suffix is a finding, never silently part of the name (A17).
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
