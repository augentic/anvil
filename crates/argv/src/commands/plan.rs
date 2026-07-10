//! `specify plan *` grammar.
//!
//! The concrete command `*Args` structs live in [`cli`]. Custom field
//! grammars ([`SourceAssign`], [`BindingArg`], [`KindAssign`]) remain
//! transport-neutral, explicit `TryFrom<Args>` conversions live in the
//! typed command router, and operation-level desugaring lives in
//! `workflow::change::plan::handlers`.
//!
//! [`SourceAssign`]: workflow::change::plan::handlers::SourceAssign
//! [`BindingArg`]: workflow::change::plan::handlers::BindingArg
//! [`KindAssign`]: workflow::change::plan::handlers::KindAssign

pub mod cli;
