//! The declarative platforms capability from a target's metadata
//! answer and its validation gate: [`PlatformsCapability::check`] and
//! the typed [`PlatformsViolation`] outcomes.

use error::Error;
use serde::{Deserialize, Serialize};

use crate::Platform;

/// Declarative platforms capability from a target's metadata answer.
///
/// When a target declares `platforms`, the CLI enforces platform
/// requirements at `emery init` time and scaffolds defaults for
/// greenfield workspace members: `required` demands `--platforms`,
/// `allowed` is the closed accepted set, `default` seeds greenfield sync.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PlatformsCapability {
    /// Whether projects using this target must declare platforms.
    pub required: bool,
    /// Platforms this target accepts.
    pub allowed: Vec<Platform>,
    /// Default platform set for greenfield scaffolding.
    pub default: Vec<Platform>,
}

/// Typed outcome of [`PlatformsCapability::check`].
///
/// Each caller surface owns a diagnostic-code family
/// (`project-platforms-*` at init, `topology-cache-project-platforms-*`
/// at topology resolution); the shared `PlatformsViolation::into_error`
/// converter keeps both mappings — and the rules — in one place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformsViolation {
    /// The capability demands a platform set but none was declared.
    /// Carries the capability's display-formatted `default` set for the
    /// caller's hint text.
    RequiredButMissing {
        /// Display-formatted `default` platform tokens.
        defaults: Vec<String>,
    },
    /// A non-empty platform set omits the mandatory `core` member.
    MissingCore,
    /// A declared platform is outside the capability's `allowed` set.
    /// Carries the display-formatted allowed set for the hint text.
    NotAllowed {
        platform: Platform,
        /// Display-formatted `allowed` platform tokens.
        allowed: Vec<String>,
    },
}

/// Which caller surface a [`PlatformsViolation`] is being reported
/// from. Selects the diagnostic-code family and message wording in
/// `PlatformsViolation::into_error`.
#[derive(Debug, Clone, Copy)]
pub enum PlatformsSurface<'a> {
    /// `emery init --platforms` validation — the
    /// `project-platforms-*` family.
    Init {
        /// Target adapter name for the message text.
        target: &'a str,
    },
    /// Workspace topology backstop validation — the
    /// `topology-cache-project-platforms-*` family.
    TopologySlot {
        /// Workspace slot (registry) name for the message text.
        registry: &'a str,
        /// Target adapter name for the message text.
        target: &'a str,
    },
}

impl PlatformsViolation {
    /// Convert this violation into the engine [`Error`] for the given
    /// caller surface, preserving each surface's locked diagnostic
    /// codes and message wording.
    pub(crate) fn into_error(self, surface: PlatformsSurface<'_>) -> Error {
        match (self, surface) {
            (Self::RequiredButMissing { defaults }, PlatformsSurface::Init { target }) => {
                Error::validation_failed(
                    "project-platforms-required",
                    format!("target '{target}' requires --platforms"),
                    format!(
                        "target '{target}' requires --platforms; default set is [{}]",
                        defaults.join(", "),
                    ),
                )
            }
            (
                Self::RequiredButMissing { defaults },
                PlatformsSurface::TopologySlot { registry, target },
            ) => Error::validation_failed(
                "topology-cache-project-platforms-missing",
                format!("workspace slot `{registry}` declares platforms"),
                format!(
                    "workspace slot `{registry}` target '{target}' requires platforms but \
                     project.yaml declares none; default set is [{}]",
                    defaults.join(", "),
                ),
            ),
            (Self::MissingCore, PlatformsSurface::Init { .. }) => Error::validation_failed(
                "project-platforms-must-include-core",
                "platform set must include `core`",
                "the --platforms set must include `core`; every project that declares platforms \
                 requires the shared Rust core crate",
            ),
            (Self::MissingCore, PlatformsSurface::TopologySlot { registry, .. }) => {
                Error::validation_failed(
                    "topology-cache-project-platforms-must-include-core",
                    format!("workspace slot `{registry}` platform set includes `core`"),
                    format!(
                        "workspace slot `{registry}` platform set must include `core`; every \
                         project that declares platforms requires the shared Rust core crate",
                    ),
                )
            }
            (Self::NotAllowed { platform, allowed }, PlatformsSurface::Init { target }) => {
                Error::validation_failed(
                    "project-platforms-not-allowed",
                    format!("platform `{platform}` is not in the target's allowed set"),
                    format!(
                        "platform `{platform}` is not allowed by target '{target}'; allowed: [{}]",
                        allowed.join(", "),
                    ),
                )
            }
            (
                Self::NotAllowed { platform, allowed },
                PlatformsSurface::TopologySlot { registry, target },
            ) => Error::validation_failed(
                "topology-cache-project-platforms-not-allowed",
                format!("workspace slot `{registry}` platform `{platform}` is allowed"),
                format!(
                    "workspace slot `{registry}` platform `{platform}` is not allowed by target \
                     '{target}'; allowed: [{}]",
                    allowed.join(", "),
                ),
            ),
        }
    }
}

impl PlatformsCapability {
    /// Validate a declared platform set against this capability: a
    /// required capability refuses an empty set; a non-empty set must
    /// include [`Platform::Core`] and stay inside `allowed`. An empty
    /// set on a non-required capability passes (platforms are opt-in).
    ///
    /// # Errors
    ///
    /// Returns the first [`PlatformsViolation`] in rule order.
    pub(crate) fn check(&self, platforms: &[Platform]) -> Result<(), PlatformsViolation> {
        if platforms.is_empty() {
            if self.required {
                return Err(PlatformsViolation::RequiredButMissing {
                    defaults: self.default.iter().map(ToString::to_string).collect(),
                });
            }
            return Ok(());
        }
        if !platforms.contains(&Platform::Core) {
            return Err(PlatformsViolation::MissingCore);
        }
        for p in platforms {
            if !self.allowed.contains(p) {
                return Err(PlatformsViolation::NotAllowed {
                    platform: *p,
                    allowed: self.allowed.iter().map(ToString::to_string).collect(),
                });
            }
        }
        Ok(())
    }
}
