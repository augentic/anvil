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
// Handler/orchestrator plumbing: reachable inside the crate only. The
// `authority_override` module rides along so call sites read
// `authority_override::mutate` / `::reject_orphans` / `::emit_seed_events`.
pub(crate) use plan::core::{
    authority_override, build_request, claim_next, drained_line, entry_mut,
    orphan_authority_override_keys, plan_finding, plan_status_body, reject_duplicate_source_keys,
    scaffold, unknown_slice_err,
};
pub(crate) use plan::doctor::{author_gate, claim_gate, detect, full_report as plan_full_report};
