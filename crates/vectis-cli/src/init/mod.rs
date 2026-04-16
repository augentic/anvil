//! `vectis init` -- scaffold a new Crux project.
//!
//! Stubbed in chunk 1; real orchestration lands in chunks 5-8.

use crate::{CommandOutcome, error::VectisError};

pub fn run() -> Result<CommandOutcome, VectisError> {
    Ok(CommandOutcome::Stub { command: "init" })
}
