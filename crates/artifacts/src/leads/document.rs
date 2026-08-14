//! In-memory model of `<change>/leads.md`.
//!
//! Catalog-only inventory plus `### <source>:<lead>` blocks; no
//! prefix or suffix. Digest covers parsed fields, not Markdown.

use std::collections::BTreeMap;
use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::{Error, Result};
use serde::Serialize;

use self::parse::{Parser, is_inventory_heading};
use super::lead::Lead;
use crate::atomic;

mod parse;

/// Wire version stamped into the canonical digest payload.
const DIGEST_VERSION: u32 = 1;

/// In-memory model of one `leads.md` catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leads {
    /// Parsed lead inventory in document order.
    leads: Vec<Lead>,
}

impl Leads {
    /// An empty catalog.
    #[must_use]
    pub const fn empty() -> Self {
        Self { leads: Vec::new() }
    }

    /// Build a catalog from an already-validated lead set.
    #[must_use]
    pub const fn from_leads(leads: Vec<Lead>) -> Self {
        Self { leads }
    }

    /// Parse `text` as a catalog-only lead document.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Diag`] (`leads-parse-failed`) on a structural
    /// defect — prefix/suffix prose, duplicate `lead:` bullets,
    /// unsupported `aliases:` bullets, missing required bullets.
    pub fn parse(text: &str) -> Result<Self> {
        Parser::new(text).run()
    }

    /// Parse a survey lead set that may omit the inventory heading.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Diag`] (`leads-parse-failed`) when the
    /// normalized lead set has the same structural defects rejected by
    /// [`Self::parse`].
    pub fn parse_lead_set(text: &str) -> Result<Self> {
        if text.lines().any(is_inventory_heading) {
            Self::parse(text)
        } else {
            let mut normalized = String::with_capacity("## Lead inventory\n\n".len() + text.len());
            normalized.push_str("## Lead inventory\n\n");
            normalized.push_str(text);
            Self::parse(&normalized)
        }
    }

    /// Load and parse `leads.md` at `path`.
    ///
    /// # Errors
    ///
    /// - [`Error::ArtifactNotFound`] when the file does not exist.
    /// - [`Error::Filesystem`] on read failure.
    /// - [`Error::Diag`] on parse failure.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(Error::ArtifactNotFound {
                kind: "leads.md",
                path: path.to_path_buf(),
            });
        }
        let raw = std::fs::read_to_string(path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&raw)
    }

    /// Re-render the catalog and atomically persist it at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the temp-file write / rename fails.
    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        atomic::bytes_write(path, self.render().as_bytes())
    }

    /// Borrow the parsed lead inventory in document order.
    #[must_use]
    pub fn leads(&self) -> &[Lead] {
        &self.leads
    }

    /// Consume the document and return its lead inventory.
    #[must_use]
    pub fn into_leads(self) -> Vec<Lead> {
        self.leads
    }

    /// Locate a lead by its canonical `lead` id for mutation.
    #[must_use]
    pub fn lead_mut(&mut self, id: &str) -> Option<&mut Lead> {
        self.leads.iter_mut().find(|lead| lead.lead == id)
    }

    /// Resolve a `--sources <key>=<value>` token to its lead by exact
    /// match on the canonical `lead` id.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError::Unknown`] when no lead matches `token`.
    pub fn resolve_lead(&self, token: &str) -> std::result::Result<&Lead, ResolveError> {
        self.leads.iter().find(|lead| lead.lead == token).ok_or_else(|| ResolveError::Unknown {
            token: token.to_string(),
        })
    }

    /// Canonical digest hex of the parsed catalog.
    ///
    /// Covers every source key and, within that source, every lead id,
    /// synopsis, topic, parent, and focus. Independent of Markdown
    /// formatting: two documents that parse to the same leads share a
    /// digest.
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn digest_hex(&self) -> Result<String> {
        let mut leads = self.leads.clone();
        leads.sort_by(|left, right| {
            left.source.cmp(&right.source).then_with(|| left.lead.cmp(&right.lead))
        });
        let payload = CanonicalCatalog {
            version: DIGEST_VERSION,
            leads,
        };
        Ok(sha256_hex(atomic::serialise_yaml(&payload)?.as_bytes()))
    }

    /// Merge a re-survey of `source` into the in-memory inventory.
    ///
    /// Incoming leads replace the prior block sharing their
    /// `(source, lead)` pair in place; new leads append in survey
    /// order. Other sources — and this source's blocks absent from
    /// the incoming set — stay. Re-survey never prunes and never
    /// collapses across sources.
    pub fn merge_leads(&mut self, source: &str, leads: Vec<Lead>) {
        let mut slots: Vec<Option<Lead>> = leads
            .into_iter()
            .map(|mut lead| {
                lead.source = source.to_string();
                Some(lead)
            })
            .collect();
        let mut slot_by_lead: BTreeMap<String, usize> = BTreeMap::new();
        for (idx, slot) in slots.iter().enumerate() {
            if let Some(lead) = slot {
                slot_by_lead.entry(lead.lead.clone()).or_insert(idx);
            }
        }

        let mut merged: Vec<Lead> = Vec::with_capacity(self.leads.len() + slots.len());
        for prior in &self.leads {
            let replacement = if prior.source == source {
                slot_by_lead.get(&prior.lead).and_then(|&idx| slots[idx].take())
            } else {
                None
            };
            match replacement {
                Some(next) => merged.push(next),
                None => merged.push(prior.clone()),
            }
        }
        for slot in &mut slots {
            if let Some(lead) = slot.take() {
                merged.push(lead);
            }
        }

        self.leads = merged;
    }

    /// Merge a re-survey of `source` into the inventory and
    /// atomically persist the result at `path`.
    ///
    /// The current view is rewritten; a previously retained revision
    /// is never touched.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the atomic re-render fails.
    pub fn merge_survey(&mut self, source: &str, leads: Vec<Lead>, path: &Path) -> Result<()> {
        self.merge_leads(source, leads);
        self.write_atomic(path)
    }

    /// Render the catalog to its on-disk shape.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::from("## Lead inventory\n");
        if !self.leads.is_empty() {
            out.push('\n');
        }
        for (idx, lead) in self.leads.iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            render_lead(&mut out, lead);
        }
        out
    }
}

/// Canonical digest payload — parsed fields only, closed sort.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct CanonicalCatalog {
    version: u32,
    leads: Vec<Lead>,
}

/// Render a single `### <source>:<lead>` block onto `out`.
fn render_lead(out: &mut String, lead: &Lead) {
    out.push_str("### ");
    out.push_str(&lead.source);
    out.push(':');
    out.push_str(&lead.lead);
    out.push_str("\n\n");
    out.push_str("- lead: ");
    out.push_str(&lead.lead);
    out.push('\n');
    out.push_str("- source: ");
    out.push_str(&lead.source);
    out.push('\n');
    out.push_str("- synopsis: ");
    out.push_str(&lead.synopsis);
    out.push('\n');
    if !lead.topics.is_empty() {
        out.push_str("- topics: [");
        out.push_str(&lead.topics.join(", "));
        out.push_str("]\n");
    }
    if let Some(parent) = &lead.parent {
        out.push_str("- parent: ");
        out.push_str(parent);
        out.push('\n');
    }
    if let Some(focus) = &lead.focus {
        out.push_str("- focus: ");
        out.push_str(focus);
        out.push('\n');
    }
}

/// Outcome of [`Leads::resolve_lead`] when the supplied token does
/// not match any lead's canonical id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// No lead has a `lead` id matching `token`.
    Unknown {
        /// Operator-supplied value that failed to resolve.
        token: String,
    },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown { token } => {
                write!(f, "no lead in leads.md has an id matching `{token}`")
            }
        }
    }
}

impl std::error::Error for ResolveError {}
