//! Lead-reconciliation envelope DTOs and the plan-time `propose` core.
//!
//! One wire envelope discriminated by the closed `kind: request | response`.

mod catalog;
mod kernel;
mod topology;
mod wire;

pub use catalog::build_request;
pub use kernel::resolve_target;
pub use topology::resolve_topology;
pub use wire::{GateProse, ProjectRef, ProposalRequest, ProposalResponse};

/// Wire version stamped on both envelope kinds.
pub(crate) const PROPOSAL_VERSION: u32 = 1;
