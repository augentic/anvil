//! The Emery change loop: the plan author orchestration, the drained
//! execute loop, and the `emery plan *` operations. The `plan.yaml`
//! state machine lives in [`project::plan`].

pub(crate) mod judgment;
pub mod orchestrate;
pub mod plan;
pub mod source;

// The intentional external surface: the plan state machine, its domain
// enums and DTO types, and the propose/topology entry points the
// native host and crate-level tests drive.
pub use project::plan::{
    AdvanceBody, AdvanceReason, AuthorityOverride, Disagreement, DisagreementValue, Divergence,
    Entry, EntryPatch, GapRow, GapsBody, LoopStep, NextActionKind, Patch, Plan, SharedLeadRollup,
    SliceSourceBinding, SourceBinding, Status, StatusBody, StatusCounts, StopBody, StopReason,
};
