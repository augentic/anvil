//! RFC §6.1 bootstrap trigger context — pure composition over shell detect.

#[cfg(test)]
#[path = "bootstrap/tests.rs"]
mod tests;

use std::path::Path;

use specify_error::{Error, Result};

use super::detect::vectis_missing_platforms;
use crate::Platform;
use crate::config::ProjectConfig;

/// UI platforms that can trigger the §6.1 `app-icon` gate.
const UI_PLATFORMS: &[Platform] = &[Platform::Ios, Platform::Android];

/// RFC §6.1 bootstrap trigger context for a Vectis-bound project.
///
/// [`Self::triggers`] is `true` when vectis shell-detect reports at least
/// one declared UI platform (`ios` / `android`) absent on disk.
/// `core`-only absence does not trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapContext {
    /// Whether RFC §6.1 applies.
    pub triggers: bool,
    /// Declared-but-absent UI platforms (`ios` / `android` only).
    ///
    /// Empty when [`Self::triggers`] is `false`. `core` is never included.
    pub missing_ui: Vec<Platform>,
}

/// Compute RFC §6.1 bootstrap context from in-process shell detect.
///
/// Loads `project.yaml.platforms` and reuses [`vectis_missing_platforms`];
/// does not read `plan.yaml` slice names.
///
/// # Errors
///
/// Propagates [`ProjectConfig::load`], adapter resolution, and detect failures.
pub fn bootstrap_context(project_dir: &Path) -> Result<BootstrapContext, Error> {
    let config = ProjectConfig::load(project_dir)?;
    if config.platforms.is_empty() {
        return Ok(empty_bootstrap_context());
    }
    let missing = vectis_missing_platforms(project_dir, &config.platforms)?;
    Ok(bootstrap_context_from_missing(&missing))
}

/// Derive §6.1 bootstrap context from a declared-but-absent platform list.
///
/// Filters to UI platforms only; used by [`bootstrap_context`] and exposed
/// for pure composition in plan-validate callers.
#[must_use]
pub fn bootstrap_context_from_missing(missing: &[Platform]) -> BootstrapContext {
    let missing_ui: Vec<Platform> =
        missing.iter().copied().filter(|p| UI_PLATFORMS.contains(p)).collect();
    BootstrapContext {
        triggers: !missing_ui.is_empty(),
        missing_ui,
    }
}

const fn empty_bootstrap_context() -> BootstrapContext {
    BootstrapContext {
        triggers: false,
        missing_ui: Vec::new(),
    }
}
