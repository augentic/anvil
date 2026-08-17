//! Build-time platform gate (A14).
//!
//! The project's declared platform set must satisfy the bound target's
//! capability at every build assembly, not just at `emery init` —
//! otherwise a project scaffolded before the target declared its
//! capability (or a hand-edited `project.yaml`) reaches the target
//! with no validated set and the target is left to guess.

use error::Error;
use project::adapter::{PlatformsSurface, TargetAdapter};
use project::config::{Layout, ProjectConfig};

/// Enforce the target's platforms capability over the project's
/// declared set.
///
/// A target without a capability passes; a detached change home has no
/// `project.yaml` to validate, so the target's in-guest validation
/// over the materialized tree is the gate there.
///
/// # Errors
///
/// The `target-build-platforms-*` family (exit 2) on the first
/// violation; config-load failures surface unchanged.
pub fn enforce(layout: Layout<'_>, adapter: &TargetAdapter) -> Result<(), Error> {
    let Some(capability) = &adapter.platforms else {
        return Ok(());
    };
    if layout.is_detached() {
        return Ok(());
    }
    let config = ProjectConfig::load(layout.project_dir())?;
    capability.enforce(
        &config.platforms,
        PlatformsSurface::Build {
            target: &adapter.name,
        },
    )
}
