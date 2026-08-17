//! Build-time platform gate (A14).
//!
//! The declared platform set must satisfy the bound target's capability
//! at every build assembly — the target never guesses an unvalidated set.

use error::Error;
use project::adapter::{PlatformsSurface, TargetAdapter};
use project::config::{Layout, ProjectConfig};

/// Enforce the target's platforms capability over the project's
/// declared set.
///
/// A target without a capability passes. A detached change home has no
/// `project.yaml` to validate here, so the gate defers to the target's
/// in-guest handling of the materialized tree — a platform-requiring
/// target (vectis) fails closed there when the tree declares no
/// platform set, it does not guess one.
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
