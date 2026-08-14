//! Type definitions for `plan.yaml`, split by concern behind this
//! stable facade.
//!
//! Validation findings are emitted by the sibling `validate` / `doctor` modules.

mod binding;
mod patch;
mod reconciliation;
mod source;
mod state;
mod target;

pub use binding::{DefinitionIdentity, ProfileRef, ReviewIdentity, TargetBinding};
pub use patch::{EntryPatch, Patch};
pub use reconciliation::{AuthorityOverride, Disagreement, DisagreementValue, Divergence};
pub use source::{SliceSourceBinding, SourceBinding};
pub use state::{Entry, Plan, Status};
pub use target::TargetRef;
