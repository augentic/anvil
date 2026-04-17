//! `vectis verify` -- run the per-assembly compilation pipelines.
//!
//! Chunk 2 wires in the prerequisite check (auto-detecting which assemblies
//! exist on disk so we only require the toolchains that are actually needed);
//! real verification orchestration lands in chunk 9.

use crate::{
    CommandOutcome, VerifyArgs,
    error::VectisError,
    prerequisites::{self, AssemblyKind},
};

pub fn run(args: &VerifyArgs) -> Result<CommandOutcome, VectisError> {
    let dir = args
        .dir
        .clone()
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)?;

    let mut assemblies = vec![AssemblyKind::Core];
    if dir.join("iOS").is_dir() {
        assemblies.push(AssemblyKind::Ios);
    }
    if dir.join("Android").is_dir() {
        assemblies.push(AssemblyKind::Android);
    }

    prerequisites::check(&assemblies)?;

    Ok(CommandOutcome::Stub { command: "verify" })
}
