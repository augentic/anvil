//! The `plan.yaml` data model and its state machine.
//!
//! On-disk representation of `plan.yaml` and the in-memory [`Plan`]
//! state machine that wraps it, plus the four `plan validate` health
//! diagnostics (`doctor`). [`Plan::transition`] is the only path that
//! mutates `Entry::status`.

pub mod amend;
pub mod archive;
pub mod authority_override;
pub mod create;
pub mod doctor;
mod execution;
pub mod io;
pub mod model;
pub mod next;
pub mod propose;
pub mod remove;
pub mod scaffold;
pub mod status;
pub mod transitions;
pub mod validate;

pub use authority_override::{entry_mut, unknown_slice_err};
pub use doctor::{author_gate, claim_gate, detect, full_report};
pub use model::{
    AuthorityOverride, Disagreement, DisagreementValue, Divergence, Entry, EntryPatch, Lifecycle,
    Patch, Plan, SliceSourceBinding, SourceBinding, Status,
};
pub use next::{NextBody, NextReason, claim_next};
pub use propose::{
    GateProse, ProjectRef, ProposalRequest, ProposalResponse, apply_greenfield_seed, build_request,
    resolve_target, resolve_topology,
};
pub use scaffold::scaffold;
pub use status::{
    LoopStep, NextActionKind, StatusBody, StatusCounts, StopBody, StopReason, drained_line,
    plan_status_body,
};
pub use validate::{finding, orphan_authority_override_keys, reject_duplicate_source_keys};
