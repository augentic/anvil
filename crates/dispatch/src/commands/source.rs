//! `specify source {resolve, survey, extract}` — source adapter
//! operations.
//!
//! `resolve` shares the run-side dispatch with the target axis on the
//! unified `commands::resolve_adapter` helper (it is byte-identical to
//! the target-axis path apart from the `@version` peel). `survey` and
//! `extract` are guest-owned collapsed orchestrations
//! (`workflow_lib::orchestrate`) — only their clap surface lives
//! here so the grammar stays whole.

pub mod cli;
