//! On-disk representation of `plan.yaml` and the in-memory [`Plan`]
//! state machine that wraps it. [`Plan::transition`] is the only path
//! that mutates `Entry::status`.

pub mod amend;
pub mod archive;
pub mod authority_override;
pub mod create;
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

pub use authority_override::{
    emit_seed_events as emit_authority_override_seed_events, entry_mut, mutate_authority_overrides,
    reject_orphan_overrides, unknown_slice_err,
};
pub use model::{
    Disagreement, DisagreementValue, Divergence, Entry, EntryPatch, Lifecycle, Patch, Plan,
    SliceAuthorityOverride, SliceSourceBinding, SourceBinding, Status, TargetRef,
    TargetRefParseError,
};
pub use next::{NextBody, NextReason, claim_next};
pub use propose::{
    GateProse, LeadCatalog, LeadCatalogEntry, ProjectRef, ProposalKind, ProposalRequest,
    ProposalResponse, ProposeOutcome, ResponseMember, ResponseSlice, apply_greenfield_seed,
    build_request, resolve_target, resolve_topology,
};
pub use scaffold::scaffold;
pub use status::{
    LoopStep, NextActionKind, StatusBody, StatusCounts, StopBody, StopReason, drained_line,
    plan_status_body,
};
pub use validate::{orphan_authority_override_keys, plan_finding, reject_duplicate_source_keys};
