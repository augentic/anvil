//! The Emery change loop: plan-driven multi-slice changes — the
//! author orchestration (survey fan-out, reconciliation judgment,
//! Gate 1 prose), the drained execute loop, and the `emery plan *`
//! operations. The `plan.yaml` state machine lives in
//! [`project::plan`]; the per-slice refine / build / merge loop this
//! crate drives lives in `slice`. See
//! `docs/standards/architecture.md` for the rationale.

pub(crate) mod judgment;
pub mod orchestrate;
pub mod plan;
pub mod source;

// The intentional external surface: the plan state machine, its domain
// enums and DTO types, and the propose/topology entry points the
// native host and crate-level tests drive.
pub use project::plan::{
    AuthorityOverride, Disagreement, DisagreementValue, Divergence, Entry, EntryPatch, Lifecycle,
    LoopStep, NextActionKind, NextBody, NextReason, Patch, Plan, SliceSourceBinding, SourceBinding,
    Status, StatusBody, StatusCounts, StopBody, StopReason,
};
