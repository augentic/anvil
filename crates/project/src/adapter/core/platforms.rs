//! The declarative platforms capability from a target's metadata
//! answer and its validation gate: [`PlatformsCapability::check`] and
//! the typed [`PlatformsViolation`] outcomes.

use error::Error;
use serde::{Deserialize, Serialize};

use crate::Platform;

/// Declarative platforms capability from a target's metadata answer.
///
/// When a target declares `platforms`, the CLI enforces platform
/// requirements at `emery init` time: `required` demands
/// `--platforms`, `allowed` is the closed accepted set, `default`
/// names the suggested greenfield set.
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
/// (`project-platforms-*` at init); the shared
/// `PlatformsViolation::into_error` converter keeps the mapping — and
/// the rules — in one place.
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

/// Which caller surface a `PlatformsViolation` is being reported
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
    /// Build-assembly validation (A14) — the
    /// `target-build-platforms-*` family. Platforms are re-validated
    /// at every build, not just at init: a project scaffolded before
    /// the target declared its capability (or a hand-edited set) must
    /// refuse typed instead of letting the target guess a shell set.
    Build {
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
            (Self::MissingCore, PlatformsSurface::Init { .. }) => Error::validation_failed(
                "project-platforms-must-include-core",
                "platform set must include `core`",
                "the --platforms set must include `core`; every project that declares platforms \
                 requires the shared Rust core crate",
            ),
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
            (Self::RequiredButMissing { defaults }, PlatformsSurface::Build { target }) => {
                Error::validation_failed(
                    "target-build-platforms-required",
                    format!("target '{target}' requires a declared platform set"),
                    format!(
                        "target '{target}' requires a declared platform set but project.yaml \
                         carries none; run `emery init --upgrade --platforms <csv>` (default set \
                         is [{}])",
                        defaults.join(", "),
                    ),
                )
            }
            (Self::MissingCore, PlatformsSurface::Build { .. }) => Error::validation_failed(
                "target-build-platforms-must-include-core",
                "the project platform set must include `core`",
                "the project.yaml platform set omits the mandatory `core` member; run `emery \
                 init --upgrade --platforms <csv>` with a set that includes `core`",
            ),
            (Self::NotAllowed { platform, allowed }, PlatformsSurface::Build { target }) => {
                Error::validation_failed(
                    "target-build-platforms-not-allowed",
                    format!("platform `{platform}` is not in the target's allowed set"),
                    format!(
                        "the project.yaml platform set names `{platform}`, which target \
                         '{target}' does not allow; allowed: [{}]",
                        allowed.join(", "),
                    ),
                )
            }
        }
    }
}

impl PlatformsCapability {
    /// Validate `platforms` and convert the first violation onto
    /// `surface`'s diagnostic family — the one public gate every
    /// caller surface (init, build assembly) runs.
    ///
    /// # Errors
    ///
    /// The first violation in rule order, typed for `surface`.
    pub fn enforce(
        &self, platforms: &[Platform], surface: PlatformsSurface<'_>,
    ) -> Result<(), Error> {
        self.check(platforms).map_err(|violation| violation.into_error(surface))
    }

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
