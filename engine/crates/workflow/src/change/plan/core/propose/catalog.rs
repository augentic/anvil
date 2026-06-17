//! Lead-catalog assembly: the `(source, lead)` identity oracle and the
//! pure `kind: request` envelope builder.
//!
//! [`build_request`] / [`build_catalog`] are filesystem-free so they
//! unit-test without a temp project; the project topology they embed is
//! resolved separately by [`super::topology::resolve_topology`].

use std::collections::{BTreeMap, BTreeSet};

use specify_error::{Error, Result};
use specify_model::discovery::Discovery;

use super::PROPOSAL_VERSION;
use super::wire::{LeadCatalogEntry, ProjectRef, ProposalKind, ProposalRequest};

/// Set of `(source, lead)` identities surveyed in `discovery.md`.
///
/// The membership oracle the response-validation kernel
/// (`Plan::propose_from`) checks every agent-supplied `{ source, lead }`
/// against to reject orphan bindings and to prove every surveyed lead is
/// covered by at least one slice. Identities are deduplicated — a
/// well-formed `discovery.md` carries a unique `(source, lead)` per
/// lead, so [`LeadCatalog::len`] equals the surveyed lead count.
///
/// Keyed `source -> {lead}` (never an empty lead set, since
/// [`build_catalog`] only inserts when a lead is present) so a
/// `(source, lead)` membership probe borrows `&str` through
/// `String: Borrow<str>` and never allocates an owned key. Iterating
/// `source` then `lead` yields the same lexicographic `(source, lead)`
/// order a flat `BTreeSet<(String, String)>` would.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LeadCatalog {
    identities: BTreeMap<String, BTreeSet<String>>,
}

impl LeadCatalog {
    /// `true` when the `(source, lead)` identity was surveyed.
    #[must_use]
    pub fn contains(&self, source: &str, lead: &str) -> bool {
        self.identities.get(source).is_some_and(|leads| leads.contains(lead))
    }

    /// Number of distinct surveyed identities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.identities.values().map(BTreeSet::len).sum()
    }

    /// `true` when no lead was surveyed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.identities.is_empty()
    }

    /// Surveyed `(source, lead)` identities in lexicographic order.
    pub(super) fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.identities.iter().flat_map(|(source, leads)| {
            leads.iter().map(move |lead| (source.as_str(), lead.as_str()))
        })
    }
}

/// Build the `(source, lead)` identity set from a surveyed
/// `discovery.md`.
///
/// Shared with the response-validation kernel: `propose --from`
/// re-reads `discovery.md`, calls this to rebuild the catalog, then
/// checks every response `(source, lead)` against it. Duplicate
/// identities collapse into one set entry (see [`LeadCatalog`]).
#[must_use]
pub fn build_catalog(discovery: &Discovery) -> LeadCatalog {
    let mut identities: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for lead in discovery.leads() {
        identities.entry(lead.source.clone()).or_default().insert(lead.lead.clone());
    }
    LeadCatalog { identities }
}

/// Assemble the `kind: request` envelope from a surveyed `discovery.md`
/// and an already-resolved project topology.
///
/// `leads[]` is one [`LeadCatalogEntry`] per `discovery.leads()` row,
/// carrying `source`, `lead`, and `synopsis`.
/// `projects` (produced by [`super::topology::resolve_topology`]) is
/// embedded verbatim.
///
/// # Errors
///
/// Returns [`Error::Validation`] (`plan-reconcile-empty-catalog`, exit
/// 2) when `discovery.md` carries no leads — `propose --dry-run` has
/// nothing to reconcile.
pub fn build_request(discovery: &Discovery, projects: &[ProjectRef]) -> Result<ProposalRequest> {
    let leads: Vec<LeadCatalogEntry> = discovery
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
            "propose --dry-run requires at least one surveyed lead",
            "discovery.md carries no leads under `## Lead inventory`",
        ));
    }

    Ok(ProposalRequest {
        version: PROPOSAL_VERSION,
        kind: ProposalKind::Request,
        projects: projects.to_vec(),
        leads,
    })
}
