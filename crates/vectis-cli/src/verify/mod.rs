//! `vectis verify` -- run the per-assembly compilation pipelines.
//!
//! Stubbed in chunk 1; real orchestration lands in chunk 9.

use crate::{CommandOutcome, error::VectisError};

pub fn run() -> Result<CommandOutcome, VectisError> {
    Ok(CommandOutcome::Stub { command: "verify" })
}
