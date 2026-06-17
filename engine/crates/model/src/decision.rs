//! Decision Record parser — front-matter + Nygard body.
//!
//! A Decision Record is a YAML front-matter header (schema-shaped via
//! [`DecisionRecord`]) plus a Markdown body carrying `## Context` /
//! `## Decision` / `## Consequences`. The slice authors `slug` plus
//! `status: accepted | rejected` (and optional `supersedes:` /
//! `related:`); `specify slice merge` stamps the durable `id` / `slice`
//! / `date` and promotes the record into the append-only baseline
//! catalogue at `.specify/decisions/DEC-NNNN-<slug>.md`.
//!
//! This parser owns the *per-file* findings the refine gate raises
//! (`decision-record-schema`, `decision-record-section-missing`,
//! `decision-slug-grammar`); the cross-file `decision-slug-collision`
//! and baseline-resolved `decision-supersede-orphan` checks live in the
//! workflow validate handler, which has the sibling set and the live
//! baseline in hand. Like the spec provenance parser, findings
//! aggregate so the operator sees every problem in one pass.

use serde::{Deserialize, Serialize};
use specify_diagnostics::{Artifact, Diagnostic, FindingLocation};

#[cfg(test)]
mod tests;

/// Maximum length of a Decision Record `slug` (`decision-slug-grammar`).
pub const SLUG_MAX_LEN: usize = 64;

/// The three required Nygard body headings, in canonical order.
pub const REQUIRED_SECTIONS: [&str; 3] = ["Context", "Decision", "Consequences"];

// ---------------------------------------------------------------------------
// Public data types
// ---------------------------------------------------------------------------

/// Lifecycle status of a Decision Record.
///
/// The slice authors [`Accepted`](DecisionStatus::Accepted) or
/// [`Rejected`](DecisionStatus::Rejected);
/// [`Superseded`](DecisionStatus::Superseded) is engine-only, stamped at
/// merge when a newer record names this one under `supersedes:`.
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
pub enum DecisionStatus {
    /// The decision was taken and is in force.
    Accepted,
    /// The decision was considered and not taken.
    Rejected,
    /// Engine-only: superseded by a later record (`superseded-by`).
    Superseded,
}

/// The schema-shaped front-matter of a Decision Record.
///
/// One shape serves both the slice-authored form (engine-stamped fields
/// absent) and the promoted baseline form (`id` / `slice` / `date`
/// present). Field declaration order matches the persisted baseline
/// header so a re-serialise round-trips cleanly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionRecord {
    /// Engine-stamped durable project-global id (`DEC-NNNN`). Absent in
    /// the slice-authored form; `Some` on the persisted baseline form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Stable kebab-case slug — the only key the agent authors.
    pub slug: String,
    /// Lifecycle status (`accepted` / `rejected`; `superseded` engine-only).
    pub status: DecisionStatus,
    /// Engine-stamped slug of the slice that introduced the record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice: Option<String>,
    /// Engine-stamped merge date (`YYYY-MM-DD`, injected clock).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Records this decision supersedes — each a baseline `DEC-NNNN` id
    /// or the slug of a record merged earlier in the same slice.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
    /// Optional traceability into this slice's requirements (`REQ-NNN`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,
    /// Optional kebab-case topic slugs naming the domains this decision
    /// governs (RFC-46 D3). The plan-time `propose` join key against
    /// surveyed lead `topics[]`; absent means the decision joins nothing
    /// and never blocks. Slice-authored.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    /// Engine-stamped on a superseded record: the `DEC-NNNN` that
    /// replaced it.
    #[serde(default, rename = "superseded-by", skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
}

/// One per-file parse or validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Stable kebab-case rule identifier (e.g. `decision-slug-grammar`).
    pub rule_id: &'static str,
    /// Human-readable rule description.
    pub rule: &'static str,
    /// Specific detail naming the offending value.
    pub detail: String,
}

impl Finding {
    /// Lift a [`Finding`] into the neutral [`Diagnostic`] currency.
    /// `path_hint` (a slice-relative path) anchors the location and is
    /// prepended to the detail. Decision-record breaches are
    /// deterministic `violation` findings against the `Decisions`
    /// artifact.
    #[must_use]
    pub fn into_diagnostic(self, path_hint: &str) -> Diagnostic {
        let Self {
            rule_id,
            rule,
            detail,
        } = self;
        let location = (!path_hint.is_empty()).then(|| FindingLocation {
            path: path_hint.to_string(),
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        });
        let detail = if path_hint.is_empty() { detail } else { format!("{path_hint}: {detail}") };
        Diagnostic::violation(rule_id, rule, detail, Artifact::Decisions, location)
    }
}

/// Result of [`parse_decision`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedDecision {
    /// The deserialized front-matter, or `None` when the front-matter
    /// was missing or failed to deserialize (a `decision-record-schema`
    /// finding is recorded in that case).
    pub record: Option<DecisionRecord>,
    /// The first `# ` (H1) heading text in the body, or `None` when
    /// absent. Projected into routing identity as the
    /// decision title.
    pub title: Option<String>,
    /// Per-file findings accumulated during parsing.
    pub findings: Vec<Finding>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parse one Decision Record file into its front-matter, H1 title, and
/// per-file findings (`decision-record-schema`,
/// `decision-record-section-missing`, `decision-slug-grammar`).
#[must_use]
pub fn parse_decision(text: &str) -> ParsedDecision {
    let mut findings: Vec<Finding> = Vec::new();

    let (record, body) = if let Some((front, body)) = split_frontmatter(text) {
        match serde_saphyr::from_str::<DecisionRecord>(front) {
            Ok(record) => (Some(record), body),
            Err(err) => {
                findings.push(Finding {
                    rule_id: "decision-record-schema",
                    rule: "Decision Record front-matter matches `decision.schema.json`",
                    detail: format!("front-matter failed to parse: {err}"),
                });
                (None, body)
            }
        }
    } else {
        findings.push(Finding {
            rule_id: "decision-record-schema",
            rule: "Decision Record front-matter matches `decision.schema.json`",
            detail: "missing `---` YAML front-matter block".to_string(),
        });
        (None, text)
    };

    if let Some(record) = &record {
        check_slug_grammar(&record.slug, &mut findings);
    }

    let title = h1_title(body);
    check_sections(body, &mut findings);

    ParsedDecision {
        record,
        title,
        findings,
    }
}

/// `true` when `slug` matches `^[a-z][a-z0-9-]*$` and is at most
/// [`SLUG_MAX_LEN`] characters (`decision-slug-grammar`).
#[must_use]
pub fn is_valid_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > SLUG_MAX_LEN {
        return false;
    }
    let mut chars = slug.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn check_slug_grammar(slug: &str, out: &mut Vec<Finding>) {
    if !is_valid_slug(slug) {
        out.push(Finding {
            rule_id: "decision-slug-grammar",
            rule: "`slug` matches `^[a-z][a-z0-9-]*$` (≤ 64 chars)",
            detail: format!(
                "slug `{slug}` is not a valid kebab-case slug (≤ {SLUG_MAX_LEN} chars)"
            ),
        });
    }
}

fn check_sections(body: &str, out: &mut Vec<Finding>) {
    for section in REQUIRED_SECTIONS {
        if !has_section(body, section) {
            out.push(Finding {
                rule_id: "decision-record-section-missing",
                rule: "Body carries `## Context` / `## Decision` / `## Consequences`",
                detail: format!("missing required `## {section}` section"),
            });
        }
    }
}

/// `true` when `body` contains a `## <name>` heading line (the leading
/// `#` count is exactly two; trailing text after the name is allowed so
/// `## Context (forces)` still satisfies `Context`).
fn has_section(body: &str, name: &str) -> bool {
    body.lines().any(|line| {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("##") else {
            return false;
        };
        // Reject deeper headings (`###`): the next char must not be `#`.
        if rest.starts_with('#') {
            return false;
        }
        rest.trim() == name || rest.trim().starts_with(&format!("{name} "))
    })
}

/// First `# ` (H1) heading text, trimmed. Deeper headings are ignored.
fn h1_title(body: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix("# ")?;
        let title = rest.trim();
        (!title.is_empty()).then(|| title.to_string())
    })
}

/// Split a leading `---\n … \n---\n` YAML front-matter block from the
/// body. Accepts `\r\n` line endings and a trailing `\n---` at EOF.
/// Returns `None` when no opening or closing delimiter is present.
///
/// Exposed so the merge promotion kernel can re-serialise a record's
/// front-matter while preserving its Markdown body verbatim.
///
/// A sibling `Result`-returning copy lives at `specify-standards`'
/// `rules::parse::split_frontmatter`; the `specify-standards` ⊥
/// `specify-model` dependency-direction invariant blocks one shared impl.
#[must_use]
pub fn split_frontmatter(content: &str) -> Option<(&str, &str)> {
    let rest = content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n"))?;

    let mut search_from = 0;
    while let Some(rel) = rest[search_from..].find("\n---") {
        let pos = search_from + rel;
        let after = pos + "\n---".len();
        let tail = &rest[after..];
        if tail.is_empty() {
            return Some((&rest[..pos], ""));
        }
        if let Some(body) = tail.strip_prefix('\n') {
            return Some((&rest[..pos], body));
        }
        if let Some(body) = tail.strip_prefix("\r\n") {
            return Some((&rest[..pos], body));
        }
        search_from = after;
    }
    None
}
