//! Closed-shape target build request/report wire DTOs and the
//! success-blocking gate.
//!
//! `deny_unknown_fields` closes both envelope shapes.

use std::path::{Path, PathBuf};

use diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity, is_blocking};
use error::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::platform::Platform;

/// Wire version stamped on both build envelopes.
pub const BUILD_VERSION: u32 = 1;

/// The per-slice build request handed to a target adapter.
///
/// `project_dir`
/// (the working tree) and [`BuildInputs::root`] (the slice tree) are
/// distinct by design; all [`BuildArtifacts`] paths resolve against
/// `root`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BuildRequest {
    /// Wire version — always [`BUILD_VERSION`].
    pub version: u32,
    /// Slice name the build serves.
    pub slice: String,
    /// Working tree the target builds into and validates against.
    pub project_dir: PathBuf,
    /// Slice tree plus the resolved artifact paths.
    pub inputs: BuildInputs,
    /// Requirements excluded from this build's obligations (RFC-86a
    /// D4): one entry per deferred gap row on the slice, assembled
    /// from the disposition projection at request time. Empty when
    /// nothing is deferred.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deferred: Vec<DeferredRequirement>,
}

/// One requirement excluded from a build's obligations (RFC-86a D4):
/// the slice-local id and title for the target's prompt rendering,
/// plus the canonical body digest binding the exclusion to exact
/// content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DeferredRequirement {
    /// Slice-local requirement id (`REQ-NNN`) — advisory presentation.
    pub id: String,
    /// Requirement title.
    pub title: String,
    /// Canonical requirement-body digest (`sha256:<hex>`) — the
    /// deferral match key.
    pub requirement_digest: String,
}

/// The slice tree root plus the rendered artifacts the target consumes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BuildInputs {
    /// Slice tree that every [`BuildArtifacts`] path resolves against.
    pub root: PathBuf,
    /// The rendered artifact paths, relative to [`BuildInputs::root`].
    pub artifacts: BuildArtifacts,
}

/// The rendered artifact paths under [`BuildInputs::artifacts`], each
/// relative to [`BuildInputs::root`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BuildArtifacts {
    /// Path to `proposal.md`.
    pub proposal: String,
    /// Path to `design.md`.
    pub design: String,
    /// Path to `tasks.md`.
    pub tasks: String,
    /// One or more per-domain `spec.md` files (`specs/<domain>/spec.md`).
    pub specs: Vec<String>,
    /// Target-specific inputs declared by the bound adapter's manifest.
    /// Empty when the adapter declares none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional: Vec<String>,
}

/// Closed build outcome enum.
///
/// Partial success is [`BuildStatus::Success`] carrying non-blocking
/// findings only — the CLI rejects a `success` report with any blocking
/// finding via the report's internal blocking-findings gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum BuildStatus {
    /// Build succeeded; only non-blocking findings (or none) allowed.
    Success,
    /// Build failed; blocking findings allowed.
    Failure,
}

/// A single per-platform build output declared in a [`BuildReport`].
///
/// Each entry names the platform and a path (relative to `project-dir`)
/// where the target adapter produced an artifact. The CLI finalize gate
/// verifies every declared path exists and is non-empty
/// (`target-build-output-missing`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BuildOutput {
    /// Target platform.
    pub platform: Platform,
    /// Relative path (from `project-dir`) to the produced artifact.
    pub path: String,
}

/// The per-slice "has UI surface" signal authored by the build brief.
///
/// Carries the count of screen-bearing requirements the slice
/// introduces or modifies, derived from the brief's own `spec.md`
/// judgement (never from `## Platforms`). `screens == 0` means "no UI
/// surface". The UI-surface coherence check that consumed this signal
/// lives in-guest (the vectis core's report gate); the engine only
/// round-trips the field on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct UiSurface {
    /// Count of screen-bearing requirements this slice introduces or
    /// modifies. `0` means no UI surface.
    pub screens: u32,
}

/// The per-slice build report a target adapter returns.
///
/// `findings` elements are typed [`Diagnostic`]s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BuildReport {
    /// Report schema version.
    pub version: u32,
    /// Slice that was built; must match the request.
    pub slice: String,
    /// Adapter that produced the report (e.g. `omnia@1.0.0`).
    pub target: String,
    /// Adapter-reported outcome.
    pub status: BuildStatus,
    /// Diagnostic findings; defaults to `[]`.
    #[serde(default)]
    pub findings: Vec<Diagnostic>,
    /// Per-platform build outputs; defaults to `[]` for backward
    /// compatibility. When non-empty the finalize gate verifies every
    /// path exists on disk.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<BuildOutput>,
    /// Optional per-slice UI-surface signal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_surface: Option<UiSurface>,
    /// Slice-local requirement ids (`REQ-NNN`) the adapter claims to
    /// have implemented; defaults to `[]`. The finalize gate rejects
    /// any intersection with the request's `deferred[]` exclusion set
    /// (RFC-86a D4).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub covered: Vec<String>,
}

impl BuildReport {
    /// Stamp the caller-owned envelope fields onto adapter-reported
    /// content: the wire version plus the target name derived from
    /// the routed id (`target:` prefix stripped). The one stamping
    /// every host (native provider, engine guest shim) applies at the
    /// seam.
    #[must_use]
    pub fn stamped(
        id: &str, slice: String, status: BuildStatus, findings: Vec<Diagnostic>,
        outputs: Vec<BuildOutput>, ui_surface: Option<UiSurface>, covered: Vec<String>,
    ) -> Self {
        Self {
            version: BUILD_VERSION,
            slice,
            target: id.strip_prefix("target:").unwrap_or(id).to_string(),
            status,
            findings,
            outputs,
            ui_surface,
            covered,
        }
    }

    /// Reject a [`BuildStatus::Success`] report carrying any blocking
    /// finding.
    ///
    /// A finding blocks per the [`is_blocking`] predicate (an open
    /// `critical` / `important` violation). On [`BuildStatus::Failure`]
    /// blocking findings are allowed, so the gate is a no-op.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] keyed on
    /// `target-build-success-with-blocking-finding` (exit code 2) when
    /// a `success` report carries a blocking finding.
    pub fn enforce_no_blocking(&self) -> Result<()> {
        if self.status == BuildStatus::Success && self.findings.iter().any(is_blocking) {
            return Err(Error::validation_failed(
                "target-build-success-with-blocking-finding",
                "a success build report carries no blocking finding",
                format!("slice `{}` reported success with a blocking finding", self.slice),
            ));
        }
        Ok(())
    }

    /// Reject a report claiming coverage of a requirement the request
    /// excluded from build scope (RFC-86a D4).
    ///
    /// A deferred requirement is out of the build's obligations — no
    /// implementation, scaffolding, or placeholders — so a report
    /// whose `covered[]` intersects the request's `deferred[]` ids is
    /// a contract violation regardless of status.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] keyed on
    /// `target-build-deferred-covered` (exit code 2) naming every
    /// claimed deferred requirement.
    pub fn enforce_deferred_not_covered(&self, deferred: &[DeferredRequirement]) -> Result<()> {
        let claimed: Vec<&str> = deferred
            .iter()
            .filter(|req| self.covered.contains(&req.id))
            .map(|req| req.id.as_str())
            .collect();
        if claimed.is_empty() {
            return Ok(());
        }
        Err(Error::validation_failed(
            "target-build-deferred-covered",
            "a build report never claims coverage of a deferred requirement",
            format!(
                "slice `{}` report claims coverage of deferred requirement(s) {} — deferred \
                 requirements are excluded from build scope and conserved as debt",
                self.slice,
                claimed.join(", ")
            ),
        ))
    }

    /// Reject a [`BuildStatus::Success`] report whose `outputs[]`
    /// paths do not all exist under `project_dir`.
    ///
    /// Each declared path must resolve to a non-empty file **or
    /// directory** (targets like vectis declare per-platform tree
    /// paths such as `shared/`). Empty `outputs` is accepted (backward
    /// compatibility — the field is optional). On
    /// [`BuildStatus::Failure`] the gate is a no-op (a failed build
    /// need not have produced outputs).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] keyed on
    /// `target-build-output-missing` (exit code 2) when a success
    /// report declares an output path that is absent, empty
    /// (zero-length file or entry-less directory), or escapes the
    /// project directory.
    pub fn enforce_outputs_exist(&self, project_dir: &Path) -> Result<()> {
        if self.status != BuildStatus::Success || self.outputs.is_empty() {
            return Ok(());
        }
        for output in &self.outputs {
            let path = Path::new(&output.path);
            if path.is_absolute() || path.components().any(|c| c == std::path::Component::ParentDir)
            {
                return Err(Error::validation_failed(
                    "target-build-output-missing",
                    "every build output path is a relative path within the project",
                    format!(
                        "output for platform `{}` at `{}` is absolute or contains `..`",
                        output.platform, output.path
                    ),
                ));
            }
            let full = project_dir.join(path);
            match std::fs::metadata(&full) {
                Ok(meta) if meta.is_file() && meta.len() > 0 => {}
                // Tree outputs (e.g. vectis `shared/`, `iOS/`,
                // `Android/`) are declared as directory paths;
                // non-empty means at least one directory entry.
                Ok(meta) if meta.is_dir() && dir_has_entries(&full) => {}
                Ok(meta) if !meta.is_file() && !meta.is_dir() => {
                    return Err(Error::validation_failed(
                        "target-build-output-missing",
                        "every build output path is a regular file or directory",
                        format!(
                            "output for platform `{}` at `{}` exists but is neither a regular file nor a directory",
                            output.platform, output.path
                        ),
                    ));
                }
                Ok(_) => {
                    return Err(Error::validation_failed(
                        "target-build-output-missing",
                        "every build output path exists and is non-empty",
                        format!(
                            "output for platform `{}` at `{}` exists but is empty",
                            output.platform, output.path
                        ),
                    ));
                }
                Err(_) => {
                    return Err(Error::validation_failed(
                        "target-build-output-missing",
                        "every build output path exists and is non-empty",
                        format!(
                            "output for platform `{}` at `{}` does not exist under {}",
                            output.platform,
                            output.path,
                            project_dir.display()
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// `true` when the directory contains at least one entry.
fn dir_has_entries(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some())
}

/// One model-assisted build finding, as both hosts stamp it at the
/// seam.
///
/// An absent adapter rule id falls back to `target-build-finding` for
/// the title/fingerprint inputs, the as-authored id is preserved on
/// the diagnostic, and the fingerprint is recomputed over the final
/// shape. The folded `detail` prose serves as title, impact, and
/// remediation.
#[must_use]
pub fn build_finding(rule_id: Option<String>, detail: String, severity: Severity) -> Diagnostic {
    let mut diagnostic = Diagnostic::finding(
        rule_id.clone().unwrap_or_else(|| "target-build-finding".to_string()),
        detail.clone(),
        detail,
        severity,
        DiagnosticKind::Violation,
        DiagnosticSource::ModelAssisted,
        Artifact::Code,
        None,
    );
    diagnostic.rule_id = rule_id;
    diagnostic.fingerprint = diagnostics::fingerprint(&diagnostic);
    diagnostic
}
