//! Lead-reconciliation envelope DTOs and the plan-time `propose`
//! domain core.
//!
//! `specify plan propose` wraps agent-led lead reconciliation in a
//! CLI-owned projection kernel. The wire contract is a single
//! envelope discriminated by a closed `kind: request | response`,
//! validated against `schemas/discovery/proposal.schema.json`
//! ([`crate::schema::validate_proposal_json`]). The pieces split across
//! focused submodules, re-exported here so the public path stays
//! `…::core::propose::<item>`:
//!
//! - [`wire`] — the serde DTOs for both envelope kinds.
//! - [`catalog`] — the `(source, lead)` identity oracle ([`LeadCatalog`])
//!   plus the pure [`build_request`] / [`build_catalog`] assembly.
//! - [`topology`] — [`resolve_topology`], the only filesystem access:
//!   it reads the workspace topology cache or resolves the regular
//!   project's target adapter to its canonical `name@vN` ref.
//! - [`kernel`] — the `Plan::propose_from` projection kernel and its
//!   semantic invariants.

mod catalog;
mod kernel;
mod platforms;
mod topology;
mod wire;

pub use catalog::{LeadCatalog, build_catalog, build_request};
pub use kernel::{ProposeOutcome, resolve_target};
pub use platforms::ProjectMissingPlatforms;
pub use topology::{apply_greenfield_seed, resolve_topology};
pub use wire::{
    LeadCatalogEntry, ProjectRef, ProposalKind, ProposalRequest, ProposalResponse, ResponseMember,
    ResponseSlice,
};

/// Wire version pinned by `schemas/discovery/proposal.schema.json`
/// (`const: 1` on both envelope kinds).
const PROPOSAL_VERSION: u32 = 1;

#[cfg(test)]
mod tests;
