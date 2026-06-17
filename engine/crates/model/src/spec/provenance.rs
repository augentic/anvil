//! workflow §Requirement block contract — parser + validator for the
//! `ID:` / `Sources:` / `Status:` provenance metadata that core
//! synthesis emits at the top of every requirement in `spec.md`.
//!
//! The parser is deliberately lenient on whitespace (operators
//! hand-edit `spec.md` between `/spec:refine` and `/spec:build`) but
//! strict on the closed [`RequirementStatus`] enum and on the inline
//! heading [`RequirementTag`] coherence with the `Status:` line.
//!
//! Findings aggregate. A malformed `Sources:` line in one block does
//! not prevent later blocks from being parsed or validated, so the
//! operator sees every problem in one pass.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use specify_diagnostics::{Artifact, Diagnostic, FindingLocation};

// ---------------------------------------------------------------------------
// Public data types
// ---------------------------------------------------------------------------

/// One requirement block parsed from a `spec.md` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// The requirement id (e.g. `REQ-001`). Empty when the `ID:` line
    /// was absent — validation reports it; parsing does not.
    pub id: String,
    /// The heading name (e.g. `Password reset request`) with any
    /// inline tag stripped.
    pub name: String,
    /// Source keys from the `Sources:` line, in declaration order.
    /// Empty when the line was absent or carried no keys.
    pub sources: Vec<String>,
    /// Parsed `Status:` value, or `None` when the line was absent or
    /// carried an unrecognised value.
    pub status: Option<RequirementStatus>,
    /// Raw `Status:` value as seen on disk, useful for diagnostics
    /// that want to echo the operator's typo back. `None` when the
    /// line was absent entirely.
    pub status_raw: Option<String>,
    /// Optional inline heading tag (`[unknown]` / `[conflict]` /
    /// `[divergence]`). Other bracketed suffixes are ignored.
    pub tag: Option<RequirementTag>,
    /// `true` when the input lacked a `Sources:` line entirely (vs
    /// an empty list).
    pub sources_line_absent: bool,
    /// Body text below the metadata lines, with leading and trailing
    /// blank lines trimmed but interior formatting preserved.
    pub body: String,
    /// Source-text span anchored at the heading line; used for
    /// error reporting.
    pub span: Span,
}

/// Closed enum for the `Status:` line (workflow §Authority hierarchy).
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
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum RequirementStatus {
    /// One source, or multiple sources that agree.
    Agreed,
    /// No contributing evidence.
    Unknown,
    /// Tied top-authority disagreement; operator must reconcile.
    Conflict,
    /// Authority-resolved disagreement; loser is commentary.
    Divergence,
}

/// Inline heading tag attached to a `### Requirement:` line when the
/// `Status:` value is anything other than `agreed`.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, strum::Display, strum::EnumString, strum::IntoStaticStr,
)]
#[strum(serialize_all = "kebab-case")]
pub enum RequirementTag {
    /// `[unknown]`.
    Unknown,
    /// `[conflict]`.
    Conflict,
    /// `[divergence]`.
    Divergence,
}

impl RequirementTag {
    /// The `Status:` value this tag must pair with per the workflow contract
    /// §Authority hierarchy.
    #[must_use]
    pub const fn expected_status(self) -> RequirementStatus {
        match self {
            Self::Unknown => RequirementStatus::Unknown,
            Self::Conflict => RequirementStatus::Conflict,
            Self::Divergence => RequirementStatus::Divergence,
        }
    }
}

/// Byte-anchored source-text span. `line_start` is 1-based.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Span {
    /// Byte offset of the heading line.
    pub byte_start: usize,
    /// Byte offset one past the block's last line.
    pub byte_end: usize,
    /// 1-based line number of the heading.
    pub line_start: usize,
}

/// One parse-time or validation-time finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Stable kebab-case rule identifier (e.g.
    /// `spec.requirement-status-missing`).
    pub rule_id: &'static str,
    /// Human-readable rule description.
    pub rule: &'static str,
    /// Specific detail — typically names the offending requirement
    /// id or value.
    pub detail: String,
    /// Span into the original source.
    pub span: Span,
}

impl Finding {
    /// Lift a [`Finding`] into the neutral [`Diagnostic`] currency.
    /// `path_hint` (a slice-relative path) anchors the diagnostic
    /// location and is prepended to the detail so the operator can
    /// locate the offending file. Structural provenance breaches are
    /// deterministic `violation` findings against the `specs` artifact.
    #[must_use]
    pub fn into_diagnostic(self, path_hint: &str) -> Diagnostic {
        let Self {
            rule_id,
            rule,
            detail,
            span,
        } = self;
        let location = (!path_hint.is_empty()).then(|| FindingLocation {
            path: path_hint.to_string(),
            line: u32::try_from(span.line_start).ok(),
            column: None,
            end_line: None,
            end_column: None,
        });
        let detail = if path_hint.is_empty() {
            format!("line {}: {detail}", span.line_start)
        } else {
            format!("{path_hint}:{}: {detail}", span.line_start)
        };
        Diagnostic::violation(rule_id, rule, detail, Artifact::Specs, location)
    }
}

/// Result of [`parse_spec_md`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedSpec {
    /// Requirement blocks in document order.
    pub requirements: Vec<Requirement>,
    /// Structural findings accumulated during parsing.
    pub findings: Vec<Finding>,
}

impl ParsedSpec {
    /// `true` when no requirement carries any `Sources:` or `Status:`
    /// metadata — interpreted as a pre-synthesis (refining) state.
    /// Callers in `specify slice validate` skip the per-requirement
    /// provenance gate in this state to keep the `refining` lifecycle
    /// observable without spurious failures.
    #[must_use]
    pub fn is_unannotated(&self) -> bool {
        self.requirements.iter().all(|r| !r.sources_line_present() && r.status_raw.is_none())
    }

    /// Non-empty `ID:` values paired with heading tags for
    /// `slice.synthesis.*` journal emission after successful validate.
    pub fn synthesis_tags(&self) -> impl Iterator<Item = (&str, RequirementTag)> + '_ {
        self.requirements.iter().filter_map(|r| {
            if r.id.is_empty() { None } else { r.tag.map(|tag| (r.id.as_str(), tag)) }
        })
    }
}

impl Requirement {
    /// `true` when the input carried an explicit `Sources:` line
    /// (regardless of whether the list was empty).
    #[must_use]
    pub const fn sources_line_present(&self) -> bool {
        !self.sources_line_absent
    }

    fn id_or_name(&self) -> String {
        if self.id.is_empty() { format!("\"{}\"", self.name) } else { self.id.clone() }
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

const HEADING: &str = "### Requirement:";
const ID_PREFIX: &str = "ID:";
const SOURCES_PREFIX: &str = "Sources:";
const STATUS_PREFIX: &str = "Status:";

/// Parse `spec.md` into requirement blocks with provenance metadata.
///
/// Structural problems accumulate as [`ParsedSpec::findings`]; the
/// cross-validation against `plan.yaml.sources` layers on top via
/// [`validate`].
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

/// Validate parsed requirements against the slice's plan-level source
/// keys. Pass an empty `source_keys` set to skip the cross-validation
/// (structural rules still run).
#[must_use]
pub fn validate(parsed: &ParsedSpec, source_keys: &BTreeSet<String>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for req in &parsed.requirements {
        check_id(req, &mut findings);
        check_sources(req, source_keys, &mut findings);
        check_status(req, &mut findings);
    }
    findings
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn check_id(req: &Requirement, out: &mut Vec<Finding>) {
    if req.id.is_empty() {
        out.push(Finding {
            rule_id: "spec.requirement-id-missing",
            rule: "Every requirement carries an `ID:` line",
            detail: format!("requirement {} has no `ID:` line", req.id_or_name()),
            span: req.span,
        });
    } else if !is_valid_req_id(&req.id) {
        out.push(Finding {
            rule_id: "spec.requirement-id-malformed",
            rule: "Requirement `ID:` matches `REQ-NNN` (three ASCII digits)",
            detail: format!("requirement {} has malformed id `{}`", req.id_or_name(), req.id),
            span: req.span,
        });
    }
}

fn check_sources(req: &Requirement, source_keys: &BTreeSet<String>, out: &mut Vec<Finding>) {
    if req.sources_line_absent {
        out.push(Finding {
            rule_id: "spec.requirement-sources-missing",
            rule: "Every requirement carries a `Sources:` line",
            detail: format!("requirement {} has no `Sources:` line", req.id_or_name()),
            span: req.span,
        });
        return;
    }
    if req.sources.is_empty() {
        // Contract: `Sources: []` is legal exactly when `Status: unknown`
        // — an evidence-less requirement (e.g. a reconciliation-inserted
        // bootstrap slice) has no contributing source to cite.
        if req.status != Some(RequirementStatus::Unknown) {
            out.push(Finding {
                rule_id: "spec.requirement-sources-empty",
                rule: "`Sources:` lists at least one key (empty only with `Status: unknown`)",
                detail: format!(
                    "requirement {} has an empty `Sources:` line but is not `Status: unknown`",
                    req.id_or_name()
                ),
                span: req.span,
            });
        }
        return;
    }
    for key in &req.sources {
        if !is_valid_source_key(key) {
            out.push(Finding {
                rule_id: "spec.requirement-source-malformed",
                rule: "Each `Sources:` key is kebab-case (`[a-z][a-z0-9-]*`)",
                detail: format!(
                    "requirement {} has malformed source key `{key}`",
                    req.id_or_name()
                ),
                span: req.span,
            });
            continue;
        }
        if !source_keys.is_empty() && !source_keys.contains(key) {
            out.push(Finding {
                rule_id: "spec.requirement-source-undefined",
                rule: "Each `Sources:` key resolves to a slice-level plan binding",
                detail: format!(
                    "requirement {} references source key `{key}`, which is not declared on the slice's plan entry",
                    req.id_or_name()
                ),
                span: req.span,
            });
        }
    }
}

fn check_status(req: &Requirement, out: &mut Vec<Finding>) {
    match (req.status, req.status_raw.as_deref()) {
        (None, None) => out.push(Finding {
            rule_id: "spec.requirement-status-missing",
            rule: "Every requirement carries a `Status:` line",
            detail: format!("requirement {} has no `Status:` line", req.id_or_name()),
            span: req.span,
        }),
        (None, Some(raw)) => out.push(Finding {
            rule_id: "spec.requirement-status-unknown-value",
            rule: "`Status:` is one of `agreed | unknown | conflict | divergence`",
            detail: format!("requirement {} has unrecognised `Status: {raw}`", req.id_or_name()),
            span: req.span,
        }),
        (Some(status), _) => {
            if let Some(tag) = req.tag {
                if status != tag.expected_status() {
                    out.push(Finding {
                        rule_id: "spec.requirement-tag-status-mismatch",
                        rule: "Heading tag agrees with `Status:` value",
                        detail: format!(
                            "requirement {} carries heading tag `[{tag}]` but `Status: {status}`",
                            req.id_or_name(),
                        ),
                        span: req.span,
                    });
                }
            } else if status != RequirementStatus::Agreed {
                out.push(Finding {
                    rule_id: "spec.requirement-tag-status-mismatch",
                    rule: "Heading tag agrees with `Status:` value",
                    detail: format!(
                        "requirement {} has `Status: {status}` but no `[{status}]` heading tag",
                        req.id_or_name(),
                    ),
                    span: req.span,
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

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

fn is_valid_req_id(id: &str) -> bool {
    id.strip_prefix("REQ-")
        .is_some_and(|tail| tail.len() == 3 && tail.bytes().all(|b| b.is_ascii_digit()))
}

fn is_valid_source_key(s: &str) -> bool {
    let mut bytes = s.bytes();
    let Some(first) = bytes.next() else { return false };
    if !first.is_ascii_lowercase() {
        return false;
    }
    let mut prev_dash = false;
    for b in bytes {
        if b == b'-' {
            if prev_dash {
                return false;
            }
            prev_dash = true;
        } else if b.is_ascii_lowercase() || b.is_ascii_digit() {
            prev_dash = false;
        } else {
            return false;
        }
    }
    !prev_dash
}

#[cfg(test)]
mod tests;
