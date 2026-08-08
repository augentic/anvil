//! The `plan.yaml` data model and its operations.
//!
//! Per-entry ladder labels project from the fact union — `plan.yaml`
//! carries no stored status field.

pub mod advance;
pub mod amend;
pub mod archive;
pub mod authority_override;
pub mod create;
pub mod doctor;
mod execution;
pub mod gaps;
pub mod io;
pub mod model;
pub mod pins;
pub mod propose;
pub mod remove;
pub mod scaffold;
pub mod scope;
pub mod status;
pub mod undo;
pub mod validate;

pub use advance::{AdvanceBody, AdvanceReason, advance_next};
pub use authority_override::{entry_mut, unknown_slice_err};
pub use doctor::{advance_gate, author_gate, detect, full_report};
pub use execution::{collect_events, project_ladders};
pub use gaps::{GapRow, GapsBody, SharedLeadRollup, plan_gaps_body};
pub use model::{
    AuthorityOverride, Disagreement, DisagreementValue, Divergence, Entry, EntryPatch, Patch, Plan,
    SliceSourceBinding, SourceBinding, Status,
};
pub use pins::{close as close_source_pins, dir_cid, empty_cid, file_cid, source_cid, value_cid};
pub use propose::{
    GateProse, ProjectRef, ProposalRequest, ProposalResponse, apply_greenfield_seed, build_request,
    resolve_target, resolve_topology,
};
pub use scaffold::scaffold;
pub use scope::in_scope;
pub use status::{
    LoopStep, NextActionKind, StatusBody, StatusCounts, StopBody, StopReason, drained_line,
    plan_status_body,
};
pub use undo::{UndoStep, undo_entry};
pub use validate::{finding, orphan_authority_override_keys, reject_duplicate_source_keys};
