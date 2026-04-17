//! `vectis add-shell` -- add iOS or Android to an existing project.
//!
//! Chunk 2 wires in the prerequisite check (core plus the requested
//! platform); the real `app.rs` parser and shell scaffolding land in chunk 10.

use crate::{
    AddShellArgs, CommandOutcome,
    error::VectisError,
    prerequisites::{self, AssemblyKind},
};

pub fn run(args: &AddShellArgs) -> Result<CommandOutcome, VectisError> {
    let shell = match args.platform.as_str() {
        "ios" => AssemblyKind::Ios,
        "android" => AssemblyKind::Android,
        other => {
            return Err(VectisError::InvalidProject {
                message: format!(
                    "unknown shell platform: {other:?} (expected one of: ios, android)"
                ),
            });
        }
    };

    prerequisites::check(&[AssemblyKind::Core, shell])?;

    Ok(CommandOutcome::Stub { command: "add-shell" })
}
