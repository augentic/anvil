//! Shared scaffolding for target-adapter operation templates.
//!
//! Phase-leg answer shape, helpers over [`judgment`], prompt renderers,
//! and report-coherence checks. Leg sequencing stays in each target core.

use std::path::Path;

use omnia_guest::Model;
use serde::Deserialize;

use crate::answers::{
    PHASE_REPORT_ANSWER_SCHEMA, PhaseReportAnswer, REPORT_ANSWER_SCHEMA, ReportAnswer,
};
use crate::judgment;
use crate::seam::{
    Context, Error, Finding, Input, Payload, PhaseFinding, PhaseReport, Report, Status, Workspace,
};

/// Answer schema for one internal phase leg (not part of the WIT contract).
pub const PHASE_ANSWER_SCHEMA: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "applicable": {
      "description": "Whether this leg had work to do. `false` means the leg wrote nothing (e.g. a phase or format sub-flow with no owned surface in this slice).",
      "type": "boolean"
    },
    "summary": {
      "description": "One-paragraph account of what was generated, reviewed, repaired, or why the leg was skipped.",
      "minLength": 1,
      "type": "string"
    },
    "written": {
      "default": [],
      "description": "Workspace-relative paths of files this leg created or modified.",
      "items": { "type": "string" },
      "type": "array"
    }
  },
  "required": ["applicable", "summary"]
}"#;

/// One internal phase leg's schema-gated answer.
#[derive(Debug, Deserialize)]
pub struct PhaseAnswer {
    /// Whether the leg had work to do.
    pub applicable: bool,
    /// One-paragraph account of the leg's outcome.
    pub summary: String,
    /// Workspace-relative paths the leg created or modified.
    #[serde(default)]
    pub written: Vec<String>,
}

/// Issue one internal phase leg through the shared judgment helper.
///
/// # Errors
///
/// As [`judgment`].
pub async fn phase<P: Model>(
    model: &P, ctx: &Context<'_>, system: String, user: String, name: &str,
) -> Result<PhaseAnswer, Error> {
    judgment(model, ctx, system, user, name, PHASE_ANSWER_SCHEMA).await
}

/// Issue one report leg and project onto the seam-facing report.
///
/// # Errors
///
/// As [`judgment`].
pub async fn report<P: Model>(
    model: &P, ctx: &Context<'_>, system: String, user: String,
) -> Result<Report, Error> {
    judgment::<P, ReportAnswer>(model, ctx, system, user, "report", REPORT_ANSWER_SCHEMA)
        .await
        .map(ReportAnswer::into_report)
}

/// Issue one RFC-90 phase leg and project onto the phase report.
///
/// Serves `build` / `repair` / `verify` / `review`. The caller
/// attaches a continuation afterwards when it has session state to
/// preserve.
///
/// # Errors
///
/// As [`judgment`].
pub async fn phase_report<P: Model>(
    model: &P, ctx: &Context<'_>, system: String, user: String, name: &str,
) -> Result<PhaseReport, Error> {
    judgment::<P, PhaseReportAnswer>(model, ctx, system, user, name, PHASE_REPORT_ANSWER_SCHEMA)
        .await
        .map(PhaseReportAnswer::into_report)
}

/// Render the typed repair brief for a repair prompt.
///
/// One numbered block per finding with rule id, severity, location,
/// impact, and remediation — the deterministic engine projection,
/// never a transcript reconstruction.
#[must_use]
pub fn render_findings(findings: &[PhaseFinding]) -> String {
    use std::fmt::Write as _;

    if findings.is_empty() {
        return "(no findings were supplied)".to_string();
    }
    findings
        .iter()
        .enumerate()
        .map(|(index, finding)| {
            let rule = finding.rule_id.as_deref().unwrap_or("(no rule)");
            let location = finding.location.as_ref().map_or_else(String::new, |location| {
                let mut anchor = format!("\n   at: {}", location.path);
                if let Some(line) = location.line {
                    let _ = write!(anchor, ":{line}");
                }
                anchor
            });
            format!(
                "{}. [{:?}] {rule} — {}{location}\n   impact: {}\n   remediation: {}",
                index + 1,
                finding.severity,
                finding.title,
                finding.impact,
                finding.remediation,
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Join prompt bodies with `---` separators.
#[must_use]
pub fn assemble_system(bodies: &[&str]) -> String {
    bodies.join("\n\n---\n\n")
}

/// Render typed inputs as labeled path or body sections.
///
/// Path-form inputs arrive project-relative on the seam and are
/// rendered against the workspace's agent-visible artifact root
/// ([`Workspace::artifact_path`]), so the agent — whose working
/// directory is the lent private workspace — can read each file
/// before writing code. Body-form inputs (RFC-55 non-lent
/// deployments) inline the artifact text under the label with no
/// path.
#[must_use]
pub fn render_inputs(inputs: &[Input], workspace: &Workspace) -> String {
    if inputs.is_empty() {
        return "(no slice artifacts were provided)".to_string();
    }
    let sections = inputs
        .iter()
        .map(|input| match input.payload() {
            Payload::Path(path) => {
                format!("### input: {} → {}", input.label(), workspace.artifact_path(path))
            }
            Payload::Body(body) => format!("### input: {}\n\n{body}", input.label()),
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "Slice artifact inputs. Read each path (read-only artifact roots \
         outside your workspace) before writing code; a body is inlined \
         under its label only when the deployment does not lend a workspace.\n\n{sections}"
    )
}

/// Render one phase leg outcome for the report prompt.
#[must_use]
pub fn render_outcome(name: &str, answer: &PhaseAnswer) -> String {
    format!(
        "- {name}: applicable={}, wrote {:?} — {}",
        answer.applicable, answer.written, answer.summary
    )
}

/// Declared outputs a `success` report claims that are missing from the
/// workspace.
///
/// `failure` reports are already parked; their output claims are not re-checked.
#[must_use]
pub fn missing_outputs(report: &Report, workspace_root: &Path) -> Vec<String> {
    if report.status == Status::Failure {
        return Vec::new();
    }
    report
        .outputs
        .iter()
        .filter(|output| !workspace_root.join(&output.path).exists())
        .map(|output| {
            format!("- declared output `{}` does not exist in the workspace", output.path)
        })
        .collect()
}

/// Append residual findings and force `failure` when any remain (or when
/// a `success` answer already carries blocking findings).
#[must_use]
pub fn enforce(mut report: Report, residual: Vec<Finding>) -> Report {
    if !residual.is_empty() {
        report.status = Status::Failure;
        report.findings.extend(residual);
    }
    if report.status == Status::Success
        && report.findings.iter().any(|finding| finding.severity.blocking())
    {
        report.status = Status::Failure;
    }
    report
}
