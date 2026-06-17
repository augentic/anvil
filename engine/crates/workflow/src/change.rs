//! Specify change orchestration: plan-driven multi-slice changes, the
//! operator-facing `change.md` brief, and the `plan.yaml` state machine.

mod plan;

pub use plan::core::{
    Divergence, Entry, EntryPatch, LeadCatalog, LeadCatalogEntry, Lifecycle, NextActionKind,
    NextBody, NextReason, Patch, Plan, ProjectMissingPlatforms, ProjectRef, ProposalKind,
    ProposalRequest, ProposalResponse, ProposeOutcome, ResponseMember, ResponseSlice,
    SliceAuthorityOverride, SliceSourceBinding, SourceBinding, Status, StatusBody, StatusCounts,
    StopBody, StopReason, TargetRef, TargetRefParseError, apply_greenfield_seed, build_catalog,
    build_request, drained_line, emit_authority_override_seed_events, entry_mut,
    mutate_authority_overrides, orphan_authority_override_keys, plan_finding,
    plan_finding_structured, plan_next_body, plan_status_body, reject_duplicate_source_keys,
    reject_orphan_overrides, resolve_target, resolve_topology, unknown_slice_err,
};
pub use plan::doctor::{
    BOOTSTRAP_APP_ICON_MISSING, CYCLE, CloneSignature, ORPHAN_SOURCE, STALE_CLONE, StaleReason,
    bootstrap_app_icon_findings, detect, doctor as plan_doctor,
};
