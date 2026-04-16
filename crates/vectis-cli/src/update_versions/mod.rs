//! `vectis update-versions` -- query registries and compute coherent pins.
//!
//! Stubbed in chunk 1; real orchestration lands in chunk 11.

use crate::{CommandOutcome, error::VectisError};

pub fn run() -> Result<CommandOutcome, VectisError> {
    Ok(CommandOutcome::Stub { command: "update-versions" })
}
