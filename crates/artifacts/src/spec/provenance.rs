//! Parser + validator for `spec.md` requirement-block provenance metadata.
//!
//! Lenient on whitespace (operators hand-edit `spec.md`), strict on the
//! closed enums and tag coherence; findings aggregate rather than fail fast.

use diagnostics::{Artifact, Diagnostic, FindingLocation};

mod checks;
mod parse;

pub use checks::validate;
pub use parse::parse_spec_md;

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

pub use super::ast::{Status as RequirementStatus, Tag as RequirementTag};

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
    /// Callers in `emery slice validate` skip the per-requirement
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
