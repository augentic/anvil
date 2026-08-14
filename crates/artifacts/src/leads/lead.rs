//! One catalog row: identity is the `(source, lead)` pair.

use error::Error;
use serde::{Deserialize, Serialize};

/// One raw, unmerged lead in `leads.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Lead {
    /// Stable kebab-case identifier, unique only within this lead's
    /// `source`. Re-survey of that source replaces the block by
    /// `(source, lead)`.
    pub lead: String,
    /// Source binding key that surfaced this lead. Matches a
    /// top-level `plan.yaml.sources.<key>` binding.
    pub source: String,
    /// Content-bearing per-source synopsis. Plan-time headline
    /// material only — never slice-time `Evidence`.
    pub synopsis: String,
    /// Optional agent-authored topic slugs (kebab-case).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    /// Parent lead id within the same source. Absent on a top-level
    /// imported evidence scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Source-local focus that produced this lead. Absent when the
    /// lead is an unfocused import or survey row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
}

impl Lead {
    /// A top-level lead with no topics, parent, or focus.
    #[must_use]
    pub fn new(
        lead: impl Into<String>, source: impl Into<String>, synopsis: impl Into<String>,
    ) -> Self {
        Self {
            lead: lead.into(),
            source: source.into(),
            synopsis: synopsis.into(),
            topics: Vec::new(),
            parent: None,
            focus: None,
        }
    }
}

/// Deterministically re-check a survey's lead set.
///
/// Every `lead` id, topic slug, and parent id must be a kebab slug
/// and the `synopsis` non-empty. This is the `survey` validate-before-
/// visible gate — the orchestrator only merges a clean set into
/// `leads.md`.
///
/// # Errors
///
/// Returns [`Error::Validation`] keyed on `leads-lead-schema`
/// (exit code 2) carrying one line per violation.
pub fn validate_leads(leads: &[Lead]) -> Result<(), Error> {
    let mut findings = Vec::new();
    for lead in leads {
        if !crate::evidence::is_kebab(&lead.lead) {
            findings.push(format!("lead `{}` is not a kebab slug", lead.lead));
        }
        if lead.synopsis.trim().is_empty() {
            findings.push(format!("lead `{}` has an empty synopsis", lead.lead));
        }
        for topic in &lead.topics {
            if !crate::evidence::is_kebab(topic) {
                findings.push(format!("lead `{}` topic `{topic}` is not a kebab slug", lead.lead));
            }
        }
        if let Some(parent) = &lead.parent
            && !crate::evidence::is_kebab(parent)
        {
            findings.push(format!("lead `{}` parent `{parent}` is not a kebab slug", lead.lead));
        }
    }
    if findings.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation {
            code: "leads-lead-schema".into(),
            detail: findings.join("; "),
        })
    }
}
