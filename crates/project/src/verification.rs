//! Closed host-verification vocabulary, profile reports, and D3 codes.
//!
//! Types and attempt-local persist only: no verifier or phase-machine wiring.

use std::fmt;
use std::str::FromStr;

use error::Error;
use serde::{Deserialize, Serialize};
use strum::VariantArray;

mod report;

pub use report::{
    Context, Edit, Handle, ProfileReport, RawOutput, SuggestionGroup, ToolPin, regression,
    unchanged_failure_set,
};

/// Finding-granularity sibling of `target-phase-source-tool`.
pub const FINDING_SOURCE_TOOL: &str = "target-phase-finding-source-tool";

/// Closed semantic verification-profile name.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    strum::VariantArray,
    schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ProfileName {
    /// Formatting conformance without source mutation.
    Fmt,
    /// Compile or platform-build viability.
    Build,
    /// Language or platform static analysis.
    Clippy,
    /// Target test execution.
    Test,
    /// Documentation build and documentation tests.
    Doc,
    /// Dependency trust policy.
    Vet,
    /// Dependency and licence policy.
    Deny,
    /// The target's complete required local gate.
    Ci,
}

/// Closed verification-context kind carried on the wire.
///
/// Phase A bind accepts only [`Self::SliceAttempt`]; the domain
/// variants exist so the field is closed before Phase B opens them.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    strum::VariantArray,
    schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum VerificationContextKind {
    /// One serial slice-attempt verification lineage.
    SliceAttempt,
    /// Frontier-domain verification context. Not accepted at bind.
    FrontierDomain,
    /// Complete-domain verification context. Not accepted at bind.
    CompleteDomain,
}

/// Persisted oracle-assurance class on a profile report.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    strum::VariantArray,
    schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum OracleAssurance {
    /// Checks over model-writable candidate inputs only.
    Candidate,
    /// The executed policy bound at least one protected input or oracle.
    Protected,
    /// The executed policy bound both candidate and protected inputs.
    Mixed,
}

/// Projected execution-assurance class. Never stored on a profile report.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    strum::VariantArray,
    schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ExecutionAssurance {
    /// Model judgment produced the result, including interpreted commands.
    ModelAssisted,
    /// Every required host attestation resolved directly.
    HostAttested,
    /// Host attestations plus deterministic in-component findings.
    Hybrid,
}

/// Closed sandbox-feature set a host may enforce and attest.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    strum::VariantArray,
    schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum SandboxFeature {
    /// Candidate workspace is the process working tree.
    WorkdirBind,
    /// Child environment is an explicit allowlist.
    EnvAllowlist,
    /// No inherited credentials enter the child environment.
    NoInheritedCredentials,
    /// Network egress is denied for the check.
    EgressDeny,
    /// Wall time, stdio bytes, and process count are bounded.
    ResourceLimits,
    /// Cancellation reaps the complete process tree.
    ProcessTreeReap,
    /// Writes are limited to declared ephemeral roots.
    EphemeralWriteRoots,
    /// Bound protected inputs are mounted read-only.
    ProtectedInputReadonly,
}

/// Closed D3 fail-closed discriminants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::VariantArray)]
pub enum Discriminant {
    /// Required profile has no deployment policy for this target/platform.
    ProfileUnavailable,
    /// A required sandbox feature cannot be enforced, or setup failed.
    SandboxDenied,
    /// Approved tool absent, or pinned toolchain identity drifted.
    ToolMissing,
    /// Policy names a parser the host does not have.
    ParserMissing,
    /// Wall time, CPU, memory, process count, or stdio bound hit.
    LimitExhausted,
    /// Host cancelled the process tree.
    Cancelled,
    /// Target/platform/profile tuple is not in the registry.
    PlatformUnsupported,
    /// Resolved record fails context, candidate, policy, or digest binding.
    AttestationMismatch,
    /// Two handles cover the same required profile/platform.
    AttestationDuplicate,
    /// Host could not persist the normalized report before return.
    AttestationPersistFailed,
    /// `ci` combined with any other profile name in one requirement set.
    ProfilesIncoherent,
}

impl VerificationContextKind {
    /// Whether Phase A bind accepts this kind.
    #[must_use]
    pub const fn accepted(self) -> bool {
        matches!(self, Self::SliceAttempt)
    }
}

impl Discriminant {
    /// Stable kebab discriminant surfaced as the JSON `error` field.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ProfileUnavailable => "verification-profile-unavailable",
            Self::SandboxDenied => "verification-sandbox-denied",
            Self::ToolMissing => "verification-tool-missing",
            Self::ParserMissing => "verification-parser-missing",
            Self::LimitExhausted => "verification-limit-exhausted",
            Self::Cancelled => "verification-cancelled",
            Self::PlatformUnsupported => "verification-platform-unsupported",
            Self::AttestationMismatch => "verification-attestation-mismatch",
            Self::AttestationDuplicate => "verification-attestation-duplicate",
            Self::AttestationPersistFailed => "verification-attestation-persist-failed",
            Self::ProfilesIncoherent => "verification-profiles-incoherent",
        }
    }

    /// Whether this discriminant routes through [`Error::Validation`] (exit 2).
    #[must_use]
    pub const fn validation(self) -> bool {
        !matches!(self, Self::Cancelled | Self::AttestationPersistFailed)
    }

    /// Fail-closed error for this discriminant.
    ///
    /// Exit-2 rows are [`Error::Validation`]; exit-1 rows are [`Error::Diag`].
    #[must_use]
    pub fn error(self, detail: impl Into<String>) -> Error {
        let detail = detail.into();
        if self.validation() {
            Error::validation_failed(self.code(), "", detail)
        } else {
            Error::Diag {
                code: self.code(),
                detail,
            }
        }
    }
}

impl fmt::Display for Discriminant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for Discriminant {
    type Err = strum::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for &variant in Self::VARIANTS {
            if variant.code() == s {
                return Ok(variant);
            }
        }
        Err(strum::ParseError::VariantNotFound)
    }
}

/// Returns `true` when `ci` appears with any other profile name.
///
/// One requirement set may name `ci` alone or any non-`ci` combination;
/// mixing `ci` with another name is incoherent.
#[must_use]
pub fn ci_exclusive(profiles: &[ProfileName]) -> bool {
    profiles.contains(&ProfileName::Ci) && profiles.iter().any(|name| *name != ProfileName::Ci)
}

/// Adapter-authored `source: tool` finding. Uses [`Error::Diag`] like
/// the rest of the `target-phase-*` family.
#[must_use]
pub fn finding_source_tool(detail: impl Into<String>) -> Error {
    Error::Diag {
        code: FINDING_SOURCE_TOOL,
        detail: detail.into(),
    }
}
