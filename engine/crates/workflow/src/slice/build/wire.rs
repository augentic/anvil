//! Closed-shape target build request/report wire DTOs and the
//! success-blocking gate.
//!
//! Both envelopes are schema-validated (`validate_build_request_json` /
//! `validate_build_report_json`) before the verb deserialises here. See
//! DECISIONS.md §"Target build envelope (D6, D9 target side, D7 proof)".

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specify_diagnostics::{
    Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity, blocking,
};
use specify_error::{Error, Result};

use crate::platform::Platform;

/// Wire version pinned by both build schemas (`version` `const: 1`).
pub const BUILD_VERSION: u32 = 1;

/// The per-slice build request handed to a target adapter.
///
/// Round-trips `schemas/target/build-request.schema.json`. `project_dir`
/// (the working tree) and [`BuildInputs::root`] (the slice tree) are
/// distinct by design; all [`BuildArtifacts`] paths resolve against
/// `root`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BuildRequest {
    /// Wire version; always [`BUILD_VERSION`] per the schema `const`.
    pub version: u32,
    /// Slice being built (kebab-case).
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
    /// Singular rendered `proposal.md`.
    pub proposal: String,
    /// Singular rendered `design.md`.
    pub design: String,
    /// Singular rendered `tasks.md`.
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
/// finding via [`enforce_report_no_blocking_on_success`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BuildOutput {
    /// Platform this output was produced for.
    pub platform: Platform,
    /// Relative path (from `project-dir`) to the produced artifact.
    pub path: String,
}

/// The per-slice "has UI surface" signal authored by the build brief.
///
/// Carries the count of screen-bearing requirements the slice
/// introduces or modifies, derived from the brief's own `spec.md`
/// judgement (never from `## Platforms`). `screens == 0` means "no UI
/// surface". The finalize phase compares this declared intent against
/// the produced `composition.yaml` via
/// [`evaluate_ui_surface_coherence`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct UiSurface {
    /// Count of screen-bearing requirements this slice introduces or
    /// modifies. `0` means no UI surface.
    pub screens: u32,
}

/// The per-slice build report a target adapter returns.
///
/// Round-trips `schemas/target/build-report.schema.json`. `findings`
/// elements are [`Diagnostic`]s governed by `diagnostic.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BuildReport {
    /// Wire version; always [`BUILD_VERSION`] per the schema `const`.
    pub version: u32,
    /// Slice that was built; must match the request.
    pub slice: String,
    /// Adapter that produced the report (e.g. `omnia@1.0.0`).
    pub target: String,
    /// `success` or `failure`.
    pub status: BuildStatus,
    /// Diagnostic findings; defaults to `[]`.
    #[serde(default)]
    pub findings: Vec<Diagnostic>,
    /// Per-platform build outputs; defaults to `[]` for backward
    /// compatibility. When non-empty the finalize gate verifies every
    /// path exists on disk.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<BuildOutput>,
    /// Optional per-slice UI-surface signal (A4). Absent on reports that
    /// predate the field, in which case [`evaluate_ui_surface_coherence`]
    /// returns no warnings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_surface: Option<UiSurface>,
}

/// Reject a [`BuildStatus::Success`] report carrying any blocking
/// finding.
///
/// A finding blocks per the [`blocking`] predicate (an open `critical`
/// / `important` violation). On [`BuildStatus::Failure`] blocking
/// findings are allowed, so the gate is a no-op.
///
/// # Errors
///
/// Returns [`Error::Validation`] keyed on
/// `target-build-success-with-blocking-finding` (exit code 2) when a
/// `success` report carries a blocking finding.
pub fn enforce_report_no_blocking_on_success(report: &BuildReport) -> Result<()> {
    if report.status == BuildStatus::Success && report.findings.iter().any(blocking) {
        return Err(Error::validation_failed(
            "target-build-success-with-blocking-finding",
            "a success build report carries no blocking finding",
            format!("slice `{}` reported success with a blocking finding", report.slice),
        ));
    }
    Ok(())
}

/// Reject a [`BuildStatus::Success`] report whose `outputs[]` paths
/// do not all exist under `project_dir`.
///
/// Each declared path must resolve to a non-empty file **or
/// directory** (targets like vectis declare per-platform tree paths
/// such as `shared/`). Empty `outputs` is accepted (backward
/// compatibility — the field is optional). On [`BuildStatus::Failure`]
/// the gate is a no-op (a failed build need not have produced
/// outputs).
///
/// # Errors
///
/// Returns [`Error::Validation`] keyed on
/// `target-build-output-missing` (exit code 2) when a success report
/// declares an output path that is absent, empty (zero-length file or
/// entry-less directory), or escapes the project directory.
pub fn enforce_report_outputs_exist(report: &BuildReport, project_dir: &Path) -> Result<()> {
    if report.status != BuildStatus::Success || report.outputs.is_empty() {
        return Ok(());
    }
    for output in &report.outputs {
        let path = Path::new(&output.path);
        if path.is_absolute() || path.components().any(|c| c == std::path::Component::ParentDir) {
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
            // Tree outputs (e.g. vectis `shared/`, `iOS/`, `Android/`)
            // are declared as directory paths; non-empty means at least
            // one directory entry.
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

/// `true` when the directory contains at least one entry.
fn dir_has_entries(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some())
}

/// Compare the report's authored `ui_surface` against the produced
/// `composition.yaml` and return any non-blocking coherence warnings
/// (A4).
///
/// This is a pure *self-consistency* check: both the UI-surface
/// judgement and the composition output come from the agent, so the
/// host never re-derives screen identification — it only catches the
/// agent contradicting itself. The returned [`Diagnostic`]s are
/// `deterministic` / `violation` / `suggestion` (non-blocking per
/// [`blocking`]); they are surfaced at finalize and never alter the
/// verb's exit code.
///
/// - `ui_surface.screens == 0` but `composition_path` declares a UI
///   surface ⇒ `composition-unexpected-for-non-ui-slice`.
/// - `ui_surface.screens > 0` but `composition_path` is empty/absent ⇒
///   `composition-empty-for-ui-slice`.
///
/// When `report.ui_surface` is `None` (a report predating the field),
/// no warnings are emitted (back-compat).
#[must_use]
pub fn evaluate_ui_surface_coherence(
    report: &BuildReport, composition_path: &Path,
) -> Vec<Diagnostic> {
    let Some(ui_surface) = report.ui_surface else {
        return Vec::new();
    };

    let has_surface = composition_declares_surface(composition_path);
    let mut warnings = Vec::new();

    if ui_surface.screens == 0 && has_surface {
        warnings.push(ui_surface_warning(
            "composition-unexpected-for-non-ui-slice",
            "A slice reporting no UI surface (`ui-surface.screens: 0`) produced a non-empty \
             composition.",
            format!(
                "slice `{}` reported `ui-surface.screens: 0` but produced a non-empty \
                 composition.yaml; the UI-surface judgement contradicts the composition output",
                report.slice
            ),
        ));
    }

    if ui_surface.screens > 0 && !has_surface {
        warnings.push(ui_surface_warning(
            "composition-empty-for-ui-slice",
            "A slice reporting a UI surface (`ui-surface.screens > 0`) produced an absent or \
             empty composition.",
            format!(
                "slice `{}` reported `ui-surface.screens: {}` but produced an absent or empty \
                 composition.yaml; the UI-surface judgement contradicts the composition output",
                report.slice, ui_surface.screens
            ),
        ));
    }

    warnings
}

/// Build a single non-blocking A4 coherence warning.
fn ui_surface_warning(rule_id: &'static str, title: &'static str, detail: String) -> Diagnostic {
    Diagnostic::finding(
        rule_id,
        title,
        detail,
        Severity::Suggestion,
        DiagnosticKind::Violation,
        DiagnosticSource::Deterministic,
        Artifact::Composition,
        None,
    )
}

/// Whether the composition at `path` declares any UI surface (A4's
/// "non-empty" definition).
///
/// Non-empty: a `screens:` map with ≥1 entry, or a `delta:` envelope
/// with any `added` / `modified` / `removed` entry. Empty: an absent
/// file, a `screens: {}` map, or an all-empty `delta:`. A malformed or
/// unreadable file is treated as empty — the coherence check is
/// advisory and never aborts.
fn composition_declares_surface(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(doc) = serde_saphyr::from_str::<Value>(&text) else {
        return false;
    };

    if doc.get("screens").and_then(Value::as_object).is_some_and(|s| !s.is_empty()) {
        return true;
    }

    doc.get("delta").and_then(Value::as_object).is_some_and(|delta| {
        ["added", "modified", "removed"]
            .iter()
            .any(|key| delta.get(*key).and_then(Value::as_object).is_some_and(|m| !m.is_empty()))
    })
}

#[cfg(test)]
mod tests;
