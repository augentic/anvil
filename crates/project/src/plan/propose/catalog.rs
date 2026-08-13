//! Lead-catalog assembly: the `(source, lead)` identity oracle and the
//! pure `kind: request` envelope builder.
//!
//! [`build_request`] / [`build_catalog`] are filesystem-free.

use std::collections::{BTreeMap, BTreeSet};

use artifacts::leads::Leads;
use error::{Error, Result};

use super::PROPOSAL_VERSION;
use super::wire::{LeadCatalogEntry, ProjectRef, ProposalKind, ProposalRequest};

/// Set of `(source, lead)` identities surveyed in `leads.md`.
///
/// The membership oracle `Plan::propose_from` checks every
/// agent-supplied `{ source, lead }` against, rejecting orphan bindings
/// and proving every surveyed lead is covered by at least one slice.
/// Keyed `source -> {lead}` (never an empty lead set) so a membership
/// probe borrows `&str` without allocating; iteration order matches a
/// flat lexicographic `(source, lead)` set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LeadCatalog {
    identities: BTreeMap<String, BTreeSet<String>>,
}

impl LeadCatalog {
    /// `true` when the `(source, lead)` identity was surveyed.
    #[must_use]
    pub(crate) fn contains(&self, source: &str, lead: &str) -> bool {
        self.identities.get(source).is_some_and(|leads| leads.contains(lead))
    }

    /// Surveyed `(source, lead)` identities in lexicographic order.
    pub(super) fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.identities.iter().flat_map(|(source, leads)| {
            leads.iter().map(move |lead| (source.as_str(), lead.as_str()))
        })
    }
}

/// Build the `(source, lead)` identity set from a surveyed
/// `leads.md`.
///
/// Shared with the response-validation kernel: the reconciliation tail
/// re-reads `leads.md`, calls this to rebuild the catalog, then
/// checks every response `(source, lead)` against it. Duplicate
/// identities collapse into one set entry (see [`LeadCatalog`]).
#[must_use]
pub fn build_catalog(leads: &Leads) -> LeadCatalog {
    let mut identities: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for lead in leads.leads() {
        identities.entry(lead.source.clone()).or_default().insert(lead.lead.clone());
    }
    LeadCatalog { identities }
}

/// Assemble the `kind: request` envelope from a surveyed `leads.md`
/// and an already-resolved project topology.
///
/// `leads[]` is one `LeadCatalogEntry` per catalog row, carrying
/// `source`, `lead`, and `synopsis`.
/// `projects` (produced by [`super::topology::resolve_topology`]) is
/// embedded verbatim.
///
/// # Errors
///
/// Returns [`Error::Validation`] (`plan-reconcile-empty-catalog`, exit
/// 2) when `leads.md` carries no leads — the reconciliation
/// request has nothing to group.
pub fn build_request(catalog: &Leads, projects: &[ProjectRef]) -> Result<ProposalRequest> {
    let leads: Vec<LeadCatalogEntry> = catalog
        .leads()
        .iter()
        .map(|lead| LeadCatalogEntry {
            source: lead.source.clone(),
            lead: lead.lead.clone(),
            synopsis: lead.synopsis.clone(),
            topics: lead.topics.clone(),
        })
        .collect();

    if leads.is_empty() {
        return Err(Error::validation_failed(
            "plan-reconcile-empty-catalog",
            "lead reconciliation requires at least one surveyed lead",
            "leads.md carries no leads under `## Lead inventory`",
        ));
    }

    Ok(ProposalRequest {
        version: PROPOSAL_VERSION,
        kind: ProposalKind::Request,
        projects: projects.to_vec(),
        leads,
    })
}
