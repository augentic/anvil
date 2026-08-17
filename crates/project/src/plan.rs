//! The `plan.yaml` data model and its operations.
//!
//! Per-entry ladder labels project from the fact union — `plan.yaml`
//! carries no stored status field.

pub mod advance;
pub mod amend;
pub mod archive;
pub mod authority_override;
pub mod correction;
pub mod create;
pub mod decomposition;
pub mod discovery;
pub mod doctor;
pub mod epoch;
mod execution;
pub mod gaps;
pub mod io;
pub mod leads;
pub mod model;
pub mod pins;
pub mod projection;
pub mod proposal;
pub mod propose;
pub mod publication;
pub mod remove;
pub mod scaffold;
pub mod schedule;
pub mod scope;
pub mod status;
pub mod validate;
pub mod verify;

pub use advance::{AdvanceBody, AdvanceReason, advance_next};
pub use authority_override::{entry_mut, unknown_slice_err};
pub use correction::{Correction, active as active_corrections};
pub use decomposition::{
    BoundaryReview, Child, Decomposition, FocusParent, PARTITION_VERSION, PartitionKind,
    PartitionResponse, ReviewVerdict, VERSION as DECOMPOSITION_VERSION, closed_slices,
    retain as retain_decomposition,
};
pub use discovery::{Discovery, VERSION as DISCOVERY_VERSION};
pub use doctor::{advance_gate, author_gate, detect, full_report};
pub use execution::{collect_events, ensure_authored, project_ladders};
pub use gaps::{
    DebtCounts, Deferral, Disposition, GapRow, GapsBody, SharedLeadRollup, plan_gaps_body,
};
pub use leads::retain as retain_leads;
pub use model::{
    AuthorityOverride, DefinitionIdentity, Disagreement, DisagreementValue, Divergence, Entry,
    EntryPatch, Patch, Plan, ProfileRef, ReviewIdentity, SliceSourceBinding, SourceBinding, Status,
    TargetBinding,
};
pub use pins::{close as close_source_pins, dir_cid, empty_cid, file_cid, source_cid, value_cid};
pub use projection::{Projections, contributing_leads};
pub use proposal::{
    Applied, Boundary as BoundaryProposal, Envelope, Frontiers, Ownership, Proposal, Repair,
    Revision, VERSION as PROPOSAL_VERSION, apply as apply_proposal, author_overlap,
};
pub use propose::{
    GateProse, ProjectRef, ProposalRequest, ProposalResponse, build_request, resolve_target,
    resolve_topology,
};
pub use scaffold::scaffold;
pub use schedule::{WorkItem, layers, ready_set};
pub use scope::in_scope;
pub use status::{
    InProgressBody, LoopStep, NextActionKind, PublicationMemberBody, PublicationMemberState,
    StatusBody, StatusCounts, StopBody, StopReason, drained_line, plan_status_body,
};
pub use validate::{finding, orphan_authority_override, reject_duplicate_source};
pub use verify::{closed_plan, resolve_from};
