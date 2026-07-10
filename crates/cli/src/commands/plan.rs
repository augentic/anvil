//! `specify plan *` grammar.
//!
//! The mirror `*Args` structs live in [`cli`]; the custom field
//! grammars ([`SourceAssign`], [`BindingArg`], [`KindAssign`]) and the
//! `--source` / `--intent` desugaring live in
//! `workflow::change::plan::handlers` so both transports share them.
//!
//! [`SourceAssign`]: workflow::change::plan::handlers::SourceAssign
//! [`BindingArg`]: workflow::change::plan::handlers::BindingArg
//! [`KindAssign`]: workflow::change::plan::handlers::KindAssign

pub mod cli;
