//! `vectis update-versions` -- query registries and compute coherent pins.
//!
//! Chunk 2 wires in the prerequisite check: by default only core toolchains
//! are required; `--verify` upgrades to all assemblies because the verify
//! subprocedure scaffolds and builds full projects. Real registry queries land
//! in chunk 11.

use crate::{
    CommandOutcome, UpdateVersionsArgs,
    error::VectisError,
    prerequisites::{self, AssemblyKind},
};

pub fn run(args: &UpdateVersionsArgs) -> Result<CommandOutcome, VectisError> {
    let assemblies: Vec<AssemblyKind> = if args.verify {
        vec![AssemblyKind::Core, AssemblyKind::Ios, AssemblyKind::Android]
    } else {
        vec![AssemblyKind::Core]
    };

    prerequisites::check(&assemblies)?;

    Ok(CommandOutcome::Stub {
        command: "update-versions",
    })
}
