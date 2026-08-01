//! The `spec.md` requirement-block parser: heading detection, the
//! `ID:` / `Sources:` / `Status:` metadata scan, and body assembly.
//! Structural findings accumulate; validation layers on top in
//! [`super::validate`].

use super::{Finding, ParsedSpec, Requirement, RequirementTag, Span};

const HEADING: &str = "### Requirement:";
const ID_PREFIX: &str = "ID:";
const SOURCES_PREFIX: &str = "Sources:";
const STATUS_PREFIX: &str = "Status:";

/// Parse `spec.md` into requirement blocks with provenance metadata.
///
/// Structural problems accumulate as [`ParsedSpec::findings`]; the
/// cross-validation against `plan.yaml.sources` layers on top via
/// [`super::validate`].
///
/// ```
/// use artifacts::spec::provenance::parse_spec_md;
///
/// let spec = "### Requirement: User registration accepts valid email\n\n\
///             ID: REQ-001\nSources: [legacy-monolith]\nStatus: agreed\n\n\
///             The registration request is accepted when the email is valid.\n";
/// let parsed = parse_spec_md(spec);
/// assert_eq!(parsed.requirements[0].id, "REQ-001");
/// assert_eq!(parsed.requirements[0].sources, ["legacy-monolith"]);
/// assert!(parsed.findings.is_empty());
/// ```
#[must_use]
pub fn parse_spec_md(text: &str) -> ParsedSpec {
    let mut requirements: Vec<Requirement> = Vec::new();
    let mut findings: Vec<Finding> = Vec::new();
    let mut current: Option<Block> = None;
    let mut byte_pos: usize = 0;
    let mut line_no: usize = 0;

    for raw_line in text.split_inclusive('\n') {
        line_no += 1;
        let line_start = byte_pos;
        let next_pos = byte_pos + raw_line.len();
        let stripped = raw_line.trim_end_matches('\n').trim_end_matches('\r');
        let trimmed = stripped.trim();

        if let Some(rest) = stripped.strip_prefix(HEADING) {
            if let Some(block) = current.take() {
                requirements.push(block.finalize(line_start));
            }
            let (name, tag) = split_heading_tag(rest.trim());
            current = Some(Block::new(name, tag, line_start, line_no));
            byte_pos = next_pos;
            continue;
        }

        let Some(block) = current.as_mut() else {
            byte_pos = next_pos;
            continue;
        };

        if !block.metadata_done {
            if trimmed.is_empty() {
                if block.seen_any_metadata() {
                    block.metadata_done = true;
                }
                byte_pos = next_pos;
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix(ID_PREFIX) {
                if block.id.is_some() {
                    findings.push(Finding {
                        rule_id: "spec.requirement-id-duplicate",
                        rule: "Each requirement carries at most one `ID:` line",
                        detail: "duplicate `ID:` line".to_string(),
                        span: block.span_to(line_no),
                    });
                }
                block.id = Some(rest.trim().to_string());
                byte_pos = next_pos;
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix(SOURCES_PREFIX) {
                if block.sources.is_some() {
                    findings.push(Finding {
                        rule_id: "spec.requirement-sources-duplicate",
                        rule: "Each requirement carries at most one `Sources:` line",
                        detail: "duplicate `Sources:` line".to_string(),
                        span: block.span_to(line_no),
                    });
                }
                block.sources = Some(parse_sources_value(rest));
                byte_pos = next_pos;
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix(STATUS_PREFIX) {
                if block.status_raw.is_some() {
                    findings.push(Finding {
                        rule_id: "spec.requirement-status-duplicate",
                        rule: "Each requirement carries at most one `Status:` line",
                        detail: "duplicate `Status:` line".to_string(),
                        span: block.span_to(line_no),
                    });
                }
                block.status_raw = Some(rest.trim().to_string());
                byte_pos = next_pos;
                continue;
            }
            block.metadata_done = true;
        }
        block.body_lines.push(stripped.to_string());
        byte_pos = next_pos;
    }

    if let Some(block) = current.take() {
        requirements.push(block.finalize(byte_pos));
    }

    ParsedSpec {
        requirements,
        findings,
    }
}

/// One requirement block being accumulated by the line scan.
struct Block {
    name: String,
    tag: Option<RequirementTag>,
    span_start_byte: usize,
    span_line: usize,
    id: Option<String>,
    sources: Option<Vec<String>>,
    status_raw: Option<String>,
    body_lines: Vec<String>,
    metadata_done: bool,
}

impl Block {
    const fn new(
        name: String, tag: Option<RequirementTag>, byte_start: usize, line_no: usize,
    ) -> Self {
        Self {
            name,
            tag,
            span_start_byte: byte_start,
            span_line: line_no,
            id: None,
            sources: None,
            status_raw: None,
            body_lines: Vec::new(),
            metadata_done: false,
        }
    }

    const fn seen_any_metadata(&self) -> bool {
        self.id.is_some() || self.sources.is_some() || self.status_raw.is_some()
    }

    fn span_to(&self, end_line: usize) -> Span {
        Span {
            byte_start: self.span_start_byte,
            byte_end: self.span_start_byte,
            line_start: end_line.max(self.span_line),
        }
    }

    fn finalize(self, byte_end: usize) -> Requirement {
        let Self {
            name,
            tag,
            span_start_byte,
            span_line,
            id,
            sources,
            status_raw,
            body_lines,
            ..
        } = self;
        let sources_line_absent = sources.is_none();
        let sources = sources.unwrap_or_default();
        let status = status_raw.as_deref().and_then(|s| s.parse().ok());
        Requirement {
            id: id.unwrap_or_default(),
            name,
            sources,
            status,
            status_raw,
            tag,
            sources_line_absent,
            body: trim_body(&body_lines),
            span: Span {
                byte_start: span_start_byte,
                byte_end,
                line_start: span_line,
            },
        }
    }
}

fn split_heading_tag(heading_text: &str) -> (String, Option<RequirementTag>) {
    let trimmed = heading_text.trim_end();
    if let Some(open) = trimmed.rfind(" [")
        && trimmed.ends_with(']')
    {
        let body = &trimmed[..open];
        let tag_text = &trimmed[open + 2..trimmed.len() - 1];
        if let Ok(tag) = tag_text.parse::<RequirementTag>() {
            return (body.trim_end().to_string(), Some(tag));
        }
    }
    (trimmed.to_string(), None)
}

fn parse_sources_value(rest: &str) -> Vec<String> {
    let trimmed = rest.trim();
    let inner = trimmed.strip_prefix('[').map_or(trimmed, |s| s.trim_start());
    let inner = inner.strip_suffix(']').map_or(inner, |s| s.trim_end());
    inner.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

fn trim_body(lines: &[String]) -> String {
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
