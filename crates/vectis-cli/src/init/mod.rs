//! `vectis init` -- scaffold a new Crux project.
//!
//! Chunk 2 wires in the prerequisite check so a missing toolchain is reported
//! before any work begins; real scaffolding orchestration lands in chunks 5-8.

use crate::{
    CommandOutcome, InitArgs,
    error::VectisError,
    prerequisites::{self, AssemblyKind},
    versions::Versions,
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

    // Resolve version pins so a bad `--version-file` is reported up-front
    // (chunk 4 smoke test). Real consumption of the resolved struct lands
    // in chunks 5/6 when the templates start needing it.
    let project_dir = args
        .dir
        .clone()
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)?;
    let _versions = Versions::resolve(&project_dir, args.version_file.as_deref())?;

    Ok(CommandOutcome::Stub { command: "init" })
}
