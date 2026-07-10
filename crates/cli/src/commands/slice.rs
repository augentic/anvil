//! `specify slice *` grammar. The handlers live in `workflow::slice::verbs`
//! (deterministic verbs) and `workflow::orchestrate::verbs` (`refine`, `build`,
//! `merge run`); only the clap surface is carried here.

pub mod cli;
