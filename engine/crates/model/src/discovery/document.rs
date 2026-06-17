//! In-memory model of `<project_dir>/discovery.md` — the
//! `## Lead inventory` section plus the surrounding operator
//! prose.
//!
//! Each block is a raw, unmerged lead identified by the `(source, lead)`
//! pair. The `## Lead inventory` section uses the block grammar
//!
//! ```markdown
//! ### <source>:<lead>
//!
//! - lead: <lead>
//! - source: <source>
//! - synopsis: <reconciliation-grade headline>
//! ```
//!
//! [`Discovery::load`] parses the file faithfully (preserving prose
//! before, between, and after lead blocks) and exposes the
//! `## Lead inventory` section as structured [`Lead`]
//! rows. [`Discovery::write_atomic`] re-renders the file with any
//! mutations propagated; prose around lead blocks round-trips
//! unchanged.
//!
//! The on-disk format intentionally mirrors what source adapters
//! write at `survey` time, so the same parser feeds re-survey
//! and operator edits.

use std::collections::BTreeMap;
use std::path::Path;

use specify_error::{Error, Result};

use self::parse::{Parser, is_inventory_heading};
use super::lead::Lead;
use crate::atomic;

mod parse;

/// In-memory model of one `discovery.md` file.
///
/// Stores every lead block under the canonical `## Lead
/// inventory` heading plus the file's surrounding prose. Mutations
/// flow through the [`Lead`] accessors ([`Discovery::lead_mut`]);
/// [`Discovery::write_atomic`] persists the result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    /// Raw prose preceding `## Lead inventory`, with the trailing
    /// newline preserved. Empty when the file starts with the heading.
    prefix: String,
    /// Parsed lead inventory in document order.
    leads: Vec<Lead>,
    /// Raw prose following the last lead block (or following the
    /// `## Lead inventory` heading when no leads were
    /// declared). Empty when nothing trails the inventory.
    suffix: String,
    /// `true` when the input file contained a `## Lead inventory`
    /// heading. A discovery.md without the heading round-trips as
    /// pure prose (the `leads` vector is empty and the heading
    /// is appended on write when leads have been added).
    has_inventory_heading: bool,
}

impl Discovery {
    /// Parse `text` as the in-memory discovery document.
    ///
    /// The parser preserves all prose outside the `## Lead
    /// inventory` section verbatim. Inside the section, every
    /// `### <id>` block plus its bullet list is collected as a
    /// [`Lead`]; bullets are parsed line-by-line.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Diag`] (`discovery-parse-failed`) on a
    /// structural defect — duplicate `lead:` bullets, retired
    /// `aliases:` bullets, missing required bullets.
    pub fn parse(text: &str) -> Result<Self> {
        Parser::new(text).run()
    }

    /// Parse a survey `leads.md` artifact.
    ///
    /// Survey briefs ask agents to write only `### <lead>` blocks; the
    /// CLI owns the surrounding `## Lead inventory` frame. This accepts
    /// both framed and unframed lead sets, then delegates to the strict
    /// discovery parser.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Diag`] (`discovery-parse-failed`) when the
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

    /// Load and parse `discovery.md` at `path`. Returns
    /// [`Error::ArtifactNotFound`] when the file is absent.
    ///
    /// # Errors
    ///
    /// - [`Error::ArtifactNotFound`] when the file does not exist.
    /// - [`Error::Filesystem`] on read failure.
    /// - [`Error::Diag`] on parse failure.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(Error::ArtifactNotFound {
                kind: "discovery.md",
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

    /// Re-render the document and atomically persist it at `path`.
    /// The atomic envelope is shared with every other `.specify/`
    /// writer ([`atomic::bytes_write`]).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the temp-file write / rename
    /// fails. Schema validation is the caller's responsibility.
    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        let body = self.render();
        atomic::bytes_write(path, body.as_bytes())
    }

    /// Borrow the parsed lead inventory in document order.
    #[must_use]
    pub fn leads(&self) -> &[Lead] {
        &self.leads
    }

    /// Consume the document and return its lead inventory in document
    /// order.
    #[must_use]
    pub fn into_leads(self) -> Vec<Lead> {
        self.leads
    }

    /// Locate a lead by its canonical `lead` id for mutation.
    #[must_use]
    pub fn lead_mut(&mut self, id: &str) -> Option<&mut Lead> {
        self.leads.iter_mut().find(|c| c.lead == id)
    }

    /// Resolve a `--sources <key>=<value>` token to its lead by exact
    /// match on the canonical `lead` id.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError::Unknown`] when no lead matches `token`.
    pub fn resolve_lead(&self, token: &str) -> std::result::Result<&Lead, ResolveError> {
        self.leads.iter().find(|c| c.lead == token).ok_or_else(|| ResolveError::Unknown {
            token: token.to_string(),
        })
    }

    /// Merge a re-survey of `source` into the inventory and
    /// atomically persist the result at `path`.
    ///
    /// `leads` is the lead set the source's `survey` produced (already
    /// validated against `schemas/discovery/lead.schema.json` by the
    /// caller). The surveying source owns attribution: every incoming
    /// lead's `source` is force-set to `source`. Each incoming lead
    /// replaces the prior block sharing its `(source, lead)` pair **in
    /// place**. Incoming leads with no prior block are appended in survey
    /// order. Prior blocks from *other* source keys, and prior blocks of
    /// this source whose `lead` is absent from the incoming set, are left
    /// untouched — re-survey replaces by `(source, lead)`, it does not
    /// prune and never collapses across sources.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the atomic re-render fails.
    pub fn merge_survey(&mut self, source: &str, leads: Vec<Lead>, path: &Path) -> Result<()> {
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
        self.write_atomic(path)
    }

    /// Render the document back to its on-disk shape.
    fn render(&self) -> String {
        let mut out = String::with_capacity(self.prefix.len() + self.suffix.len() + 128);
        out.push_str(&self.prefix);
        if self.has_inventory_heading || !self.leads.is_empty() {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("## Lead inventory\n\n");
            for (idx, lead) in self.leads.iter().enumerate() {
                if idx > 0 {
                    out.push('\n');
                }
                render_lead(&mut out, lead);
            }
        }
        if !self.suffix.is_empty() {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&self.suffix);
        }
        out
    }
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
}

/// Outcome of [`Discovery::resolve_lead`] when the supplied token does
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
                write!(f, "no lead in discovery.md has an id matching `{token}`")
            }
        }
    }
}

impl std::error::Error for ResolveError {}

#[cfg(test)]
mod tests;
