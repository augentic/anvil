//! `vectis init` -- scaffold a new Crux project.
//!
//! Chunk 2 wires in the prerequisite check so a missing toolchain is reported
//! before any work begins; real scaffolding orchestration lands in chunks 5-8.

use crate::{
    CommandOutcome, InitArgs,
    error::VectisError,
    prerequisites::{self, AssemblyKind},
};

pub fn run(args: &InitArgs) -> Result<CommandOutcome, VectisError> {
    let mut assemblies = vec![AssemblyKind::Core];
    if let Some(shells) = &args.shells {
        for shell in shells.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match shell {
                "ios" => assemblies.push(AssemblyKind::Ios),
                "android" => assemblies.push(AssemblyKind::Android),
                other => {
                    return Err(VectisError::InvalidProject {
                        message: format!(
                            "unknown shell platform: {other:?} (expected one of: ios, android)"
                        ),
                    });
                }
            }
        }
    }

    prerequisites::check(&assemblies)?;

    Ok(CommandOutcome::Stub { command: "init" })
}
