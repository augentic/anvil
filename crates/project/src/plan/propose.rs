//! Lead-reconciliation envelope DTOs and the plan-time `propose`
//! domain core.
//!
//! The guest `plan author` orchestration wraps agent-led lead
//! reconciliation in this projection kernel. The wire contract is a
//! single envelope discriminated by a closed `kind: request | response`,
//! owned by the serde DTOs in `wire` (the judgment-answer schema is
//! generated from them by [`crate::answers::proposal`]). The pieces
//! split across focused submodules, re-exported here so the public
//! path stays `…::core::propose::<item>`:
//!
//! - `wire` — the serde DTOs for both envelope kinds.
//! - `catalog` — the `(source, lead)` identity oracle (`LeadCatalog`)
//!   plus the pure [`build_request`] / `build_catalog` assembly.
//! - `topology` — [`resolve_topology`], the only filesystem access:
//!   it reads the workspace topology cache or resolves the regular
//!   project's target adapter to its canonical `name[@vN]` ref.
//! - `kernel` — the `Plan::propose_from` projection kernel and its
//!   semantic invariants.

mod catalog;
mod kernel;
mod topology;
mod wire;

pub use catalog::build_request;
pub use kernel::resolve_target;
pub use topology::{apply_greenfield_seed, resolve_topology};
pub use wire::{GateProse, ProjectRef, ProposalRequest, ProposalResponse};

/// Wire version stamped on both envelope kinds.
const PROPOSAL_VERSION: u32 = 1;
