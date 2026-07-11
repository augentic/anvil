//! Specify change orchestration: plan-driven multi-slice changes, the
//! operator-facing `change.md` brief, and the `plan.yaml` state machine.

pub mod plan;

// The intentional external surface: the plan state machine, its domain
// enums and DTO types, and the propose/topology entry points the
// harness and crate-level tests drive.
pub use plan::core::{
    Disagreement, DisagreementValue, Divergence, Entry, EntryPatch, GateProse, LeadCatalog,
    LeadCatalogEntry, Lifecycle, LoopStep, NextActionKind, NextBody, NextReason, Patch, Plan,
    ProjectRef, ProposalKind, ProposalRequest, ProposalResponse, ProposeOutcome, ResponseMember,
    ResponseSlice, SliceAuthorityOverride, SliceSourceBinding, SourceBinding, Status, StatusBody,
    StatusCounts, StopBody, StopReason, TargetRef, TargetRefParseError, apply_greenfield_seed,
    resolve_target, resolve_topology,
};
// Handler/orchestrator plumbing: reachable inside the crate only.
pub(crate) use plan::core::{
    build_request, claim_next, drained_line, emit_authority_override_seed_events, entry_mut,
    mutate_authority_overrides, orphan_authority_override_keys, plan_finding, plan_status_body,
    reject_duplicate_source_keys, reject_orphan_overrides, unknown_slice_err,
};
pub(crate) use plan::doctor::{detect, doctor as plan_doctor};
