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

/// Adapter-selected outcome of one build phase (RFC-90 D2), mirroring
/// the WIT `phase-outcome` enum.
///
/// There is no adapter-selected `success | failure`: blocking
/// findings and dispatch errors determine failure.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum PhaseOutcome {
    /// The operation ran and produced its result.
    Completed,
    /// The operation has no target-specific work for this dispatch.
    /// Must carry no blocking findings and no writes.
    NotApplicable,
}

/// Required report-level assurance claim (RFC-90 D2), mirroring the
/// WIT `phase-source` enum.
///
/// `Tool` is reserved on the wire but rejected by the RFC-90 engine
/// gate until a trusted host-tool execution seam exists (RFC-95).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum PhaseSource {
    /// No model or external tool contributed to the phase result.
    Deterministic,
    /// Model judgment produced the result, including an agent invoking
    /// and interpreting native commands.
    ModelAssisted,
    /// More than one assurance source contributed.
    Hybrid,
    /// Trusted host-tool output. Reserved; rejected in RFC-90.
    Tool,
}

/// Which engine gate supplied the findings a `repair` dispatch
/// carries (RFC-90 D2), mirroring the WIT `repair-origin` enum.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum RepairOrigin {
    /// Findings from the latest verification report.
    Verification,
    /// Findings from the latest standards-review report.
    Review,
}

/// Which writable root a phase write landed under, mirroring the WIT
/// `phase-root` enum.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum PhaseRoot {
    /// The writable product workspace (`workspace.root`).
    Workspace,
    /// The writable artifact stage (`workspace.artifact-stage.root`).
    Artifacts,
}

/// One audit-evidence write reported by a phase, mirroring the WIT
/// `phase-write` record.
///
/// Paths are relative to the named root; absolute paths and `..` are
/// invalid (an engine gate). RFC-87 capture and the staged-artifact
/// diff remain the authoritative write records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PhaseWrite {
    /// Root the path is relative to.
    pub root: PhaseRoot,
    /// Root-relative '/'-separated path.
    pub path: String,
}

/// The typed result of exactly one build-phase operation (RFC-90 D2),
/// mirroring the WIT `phase-report` record.
///
/// `findings` elements are the full typed [`Diagnostic`] shape — the
/// WIT `phase-finding` is its isomorphic projection, so nothing folds
/// at the seam. `outputs` / `ui_surface` are meaningful only on
/// `build` reports (an engine gate). The continuation rides the seam
/// but never the persisted YAML: the engine persists it separately,
/// scoped to the attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PhaseReport {
    /// Adapter-selected outcome.
    pub outcome: PhaseOutcome,
    /// Required report-level assurance claim; must cover every
    /// finding source in the report.
    pub source: PhaseSource,
    /// Typed findings; defaults to `[]`.
    #[serde(default)]
    pub findings: Vec<Diagnostic>,
    /// Candidate per-platform outputs (`build` only); defaults to `[]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<BuildOutput>,
    /// Candidate UI-surface signal (`build` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_surface: Option<UiSurface>,
    /// Audit-evidence writes performed by the phase; defaults to `[]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub written: Vec<PhaseWrite>,
    /// Adapter-opaque continuation payload: `None` preserves the
    /// current value, `Some(vec![])` clears it, non-empty replaces
    /// it. Seam-only — never serialized with the report.
    #[serde(skip)]
    pub next_continuation: Option<Vec<u8>>,
}

impl PhaseReport {
    /// Whether any finding blocks per [`is_blocking`].
    #[must_use]
    pub fn has_blocking(&self) -> bool {
        self.findings.iter().any(is_blocking)
    }
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
        outputs: Vec<BuildOutput>, ui_surface: Option<UiSurface>,
    ) -> Self {
        Self {
            version: BUILD_VERSION,
            slice,
            target: id.strip_prefix("target:").unwrap_or(id).to_string(),
            status,
            findings,
            outputs,
            ui_surface,
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
