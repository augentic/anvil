//! In-memory representation of one `## Lead inventory` block.
//!
//! One raw, unmerged lead as surfaced by one source: the `source` that
//! produced it, the kebab-case `lead` (unique only within that
//! `source`), and the content-bearing per-source `synopsis`. Identity
//! is the `(source, lead)` pair; cross-source unification is deferred
//! to plan time, where `/emery:plan`'s `propose` sub-step reads these
//! leads but never edits `discovery.md`.

use error::Error;
use serde::{Deserialize, Serialize};

/// One raw, unmerged block under `## Lead inventory` in `discovery.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Lead {
    /// Stable kebab-case identifier, unique only within this lead's
    /// `source`. Re-survey of that source replaces the block by
    /// `(source, lead)`.
    pub lead: String,
    /// Source binding key that surfaced this lead. Matches a
    /// top-level `plan.yaml.sources.<key>` binding; a `survey`
    /// attributes every lead it produces to its own source key.
    pub source: String,
    /// Content-bearing per-source synopsis of the lead as this source
    /// surfaced it. SHOULD name the operation/surface and its salient
    /// constraint so a same-slug lead from another source can be
    /// matched or distinguished on content; MAY span more than one
    /// line. Plan-time headline material only — never slice-time
    /// `Evidence`.
    pub synopsis: String,
    /// Optional agent-authored per-lead topic slugs (kebab-case).
    /// Survey populates them as additional context; the CLI computes no
    /// grouping from them — they are agent context and the join key for
    /// the propose-time decision-contradiction warning. Absent (the
    /// default) means unclassified and never blocks reconciliation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
}

/// Deterministically re-check a survey's lead set.
///
/// Every `lead` id and topic slug must be a kebab slug and the
/// `synopsis` non-empty. This is the `survey` validate-before-visible
/// gate — the orchestrator only merges a clean set into
/// `discovery.md`.
///
/// # Errors
///
/// Returns [`Error::Validation`] keyed on `discovery-lead-schema`
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
    }
    if findings.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation {
            code: "discovery-lead-schema".into(),
            detail: findings.join("; "),
        })
    }
}
