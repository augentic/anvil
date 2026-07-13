//! Type definitions for `plan.yaml`, split by concern behind this
//! stable facade.
//!
//! The facade re-exports `state` (`Plan` / `Entry` / `Status` /
//! `Lifecycle`), `source` (the source bindings), `reconciliation`
//! (divergence and authority overrides), `target` (the resolved
//! target reference), and `patch` (the amend builders). Validation
//! findings are emitted on the neutral
//! [`schema::diagnostics::Diagnostic`] currency by the sibling
//! `validate` / `doctor` modules; behaviour lives in the sibling
//! submodules.

mod patch;
mod reconciliation;
mod source;
mod state;
mod target;

pub use patch::{EntryPatch, Patch};
pub use reconciliation::{AuthorityOverride, Disagreement, DisagreementValue, Divergence};
pub use source::{SliceSourceBinding, SourceBinding};
pub use state::{Entry, Lifecycle, Plan, Status};
pub use target::TargetRef;
