//! Host profile-report DTO, attestation handle, and attempt-local store.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use diagnostics::digest::sha256_hex;
use diagnostics::{Diagnostic, Severity, is_blocking};
use error::{Error, Result};
use serde::{Deserialize, Serialize};

use super::{Discriminant, OracleAssurance, ProfileName, SandboxFeature, VerificationContextKind};
use crate::platform::Platform;
use crate::snapshot::SnapshotId;

/// Opaque attestation handle: `sha256:` of the persisted report bytes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Handle(SnapshotId);

/// Slice-attempt identity carried on a profile report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Context {
    /// Closed context kind. Phase A bind accepts only `slice-attempt`.
    pub kind: VerificationContextKind,
    /// Change (plan) identity.
    pub change: String,
    /// Slice name.
    pub slice: String,
    /// 1-based attempt ordinal.
    pub attempt: u32,
}

/// One pinned toolchain binary captured at preflight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ToolPin {
    /// PATH binary name (`rustc`, `cargo`, …).
    pub name: String,
    /// Tool-reported version string.
    pub version: String,
    /// Digest of the executable bytes.
    pub digest: SnapshotId,
}

/// One whole-file replacement in a mechanical-suggestion group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Edit {
    /// Candidate-relative `/`-separated path.
    pub path: String,
    /// Digest of the file bytes the edit replaces.
    pub preimage_digest: SnapshotId,
    /// Digest of the replacement bytes.
    pub result_digest: SnapshotId,
}

/// Atomic set of path-bounded whole-file replacements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SuggestionGroup {
    /// Edits that must apply together or not at all.
    pub edits: Vec<Edit>,
}

/// Bounded raw-output fallback when structured parse is absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RawOutput {
    /// Digest of the secret-filtered raw bytes.
    pub digest: SnapshotId,
    /// Short secret-filtered tail. No durations, pids, or temp roots.
    pub tail: String,
}

/// Host-normalized result of one profile against one candidate.
///
/// Execution assurance is projected later and is not a field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProfileReport {
    /// Profile that produced this record.
    pub profile: ProfileName,
    /// Platform slot this run filled.
    pub platform: Platform,
    /// Verification context bound for the run.
    pub context: Context,
    /// Logical candidate snapshot id.
    pub candidate: SnapshotId,
    /// Digest of the executed profile policy.
    pub policy_digest: SnapshotId,
    /// Producer-stamped report identity. Distinct from [`Handle`].
    pub report_digest: SnapshotId,
    /// Oracle-assurance class derived from the executed policy.
    pub oracle_assurance: OracleAssurance,
    /// Digests of protected inputs the executed policy bound.
    #[serde(default)]
    pub protected_inputs: Vec<SnapshotId>,
    /// Digests of oracles the executed policy bound.
    #[serde(default)]
    pub oracles: Vec<SnapshotId>,
    /// Sandbox features actually enforced on this run.
    #[serde(default)]
    pub enforced_sandbox: Vec<SandboxFeature>,
    /// Toolchain pins captured for the run.
    #[serde(default)]
    pub toolchain_identity: Vec<ToolPin>,
    /// Normalized findings in the RFC-90 [`Diagnostic`] shape.
    #[serde(default)]
    pub findings: Vec<Diagnostic>,
    /// Optional mechanical-suggestion group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion_group: Option<SuggestionGroup>,
    /// Raw fallback when structured parse is absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<RawOutput>,
}

impl Handle {
    /// Wrap the SHA-256 of `bytes` as a handle.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(SnapshotId::from_digest(&sha256_hex(bytes)))
    }

    /// Parse the canonical `sha256:<64 lowercase hex>` wire form.
    ///
    /// # Errors
    ///
    /// `snapshot-id-malformed` when the scheme or digest shape is wrong.
    pub fn parse(value: &str) -> Result<Self> {
        Ok(Self(SnapshotId::parse(value)?))
    }

    /// The canonical wire form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The bare lowercase-hex digest without the scheme prefix.
    #[must_use]
    pub fn digest(&self) -> &str {
        self.0.digest()
    }

    /// Attempt-local path: `<attempt_dir>/attestations/<digest>`.
    ///
    /// Filename is the bare hex — `:` in the wire handle is not a legal
    /// Windows path component.
    #[must_use]
    pub fn path(&self, attempt_dir: &Path) -> PathBuf {
        attempt_dir.join("attestations").join(self.digest())
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ProfileReport {
    /// Canonical YAML bytes (trailing newline).
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn canonical_yaml(&self) -> Result<String> {
        artifacts::atomic::serialise_yaml(self)
    }

    /// Handle over [`Self::canonical_yaml`].
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn handle(&self) -> Result<Handle> {
        Ok(Handle::from_bytes(self.canonical_yaml()?.as_bytes()))
    }

    /// Persist at `<attempt_dir>/attestations/<digest>`.
    ///
    /// # Errors
    ///
    /// `verification-attestation-persist-failed` on serialize or write failure.
    pub fn persist(&self, attempt_dir: &Path) -> Result<Handle> {
        let yaml = self.canonical_yaml().map_err(persist_failed)?;
        let handle = Handle::from_bytes(yaml.as_bytes());
        artifacts::atomic::bytes_write(&handle.path(attempt_dir), yaml.as_bytes())
            .map_err(persist_failed)?;
        Ok(handle)
    }

    /// Load the report stored under `handle`.
    ///
    /// # Errors
    ///
    /// Filesystem / YAML failures; `verification-attestation-mismatch`
    /// when the persisted bytes do not hash to `handle`.
    pub fn load(attempt_dir: &Path, handle: &Handle) -> Result<Self> {
        let path = handle.path(attempt_dir);
        let text = crate::fs::read_text(&path)?;
        let actual = Handle::from_bytes(text.as_bytes());
        if actual != *handle {
            return Err(Discriminant::AttestationMismatch
                .error(format!("attestation `{handle}` does not match persisted bytes")));
        }
        Ok(serde_saphyr::from_str(&text)?)
    }
}

/// Whether two reports share the same blocking-fingerprint set.
#[must_use]
pub fn unchanged_failure_set(left: &ProfileReport, right: &ProfileReport) -> bool {
    fingerprints(left) == fingerprints(right)
}

/// Whether `candidate` is lexicographically worse than `best` on
/// `(critical, important, suggestion, optional)` counts.
#[must_use]
pub fn regression(candidate: &ProfileReport, best: &ProfileReport) -> bool {
    counts(candidate) > counts(best)
}

fn fingerprints(report: &ProfileReport) -> BTreeSet<&str> {
    report
        .findings
        .iter()
        .filter(|finding| is_blocking(finding))
        .map(|finding| finding.fingerprint.as_str())
        .collect()
}

fn counts(report: &ProfileReport) -> [usize; 4] {
    let mut totals = [0; 4];
    for finding in &report.findings {
        let index = match finding.severity {
            Severity::Critical => 0,
            Severity::Important => 1,
            Severity::Suggestion => 2,
            Severity::Optional => 3,
        };
        totals[index] += 1;
    }
    totals
}

fn persist_failed(err: impl fmt::Display) -> Error {
    Discriminant::AttestationPersistFailed.error(err.to_string())
}
