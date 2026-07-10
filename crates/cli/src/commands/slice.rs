//! `specify slice *` grammar; only the clap surface is carried here.
//!
//! The handlers live in `workflow::slice::handlers` (deterministic
//! commands) and `workflow::orchestrate::handlers` (`refine`,
//! `build`, `merge run`).

pub mod cli;
