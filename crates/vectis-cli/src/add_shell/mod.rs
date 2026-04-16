//! `vectis add-shell` -- add iOS or Android to an existing project.
//!
//! Stubbed in chunk 1; real orchestration (incl. the `app.rs` parser) lands in
//! chunk 10.

use crate::{CommandOutcome, error::VectisError};

pub fn run() -> Result<CommandOutcome, VectisError> {
    Ok(CommandOutcome::Stub { command: "add-shell" })
}
