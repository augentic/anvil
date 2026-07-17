//! Shared scaffolding for target-adapter operation templates.
//!
//! Phase-leg answer shape, helpers over [`judgment`], prompt renderers,
//! and report-coherence checks. Leg sequencing stays in each target core.

use std::path::Path;

use omnia_guest::Model;
use serde::Deserialize;

use crate::answers::{REPORT_ANSWER_SCHEMA, ReportAnswer};
use crate::judgment;
use crate::seam::{Context, Error, Finding, Input, Report, Status};

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

/// Join prompt bodies with `---` separators.
#[must_use]
pub fn assemble_system(bodies: &[&str]) -> String {
    bodies.join("\n\n---\n\n")
}

/// Render typed inputs as labeled prompt sections.
#[must_use]
pub fn render_inputs(inputs: &[Input]) -> String {
    if inputs.is_empty() {
        return "(no slice artifacts were provided)".to_string();
    }
    inputs
        .iter()
        .map(|input| format!("### input: {}\n\n{}", input.label(), input.body()))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Render one phase leg outcome for the report prompt.
#[must_use]
pub fn render_outcome(name: &str, answer: &PhaseAnswer) -> String {
    format!(
        "- {name}: applicable={}, wrote {:?} — {}",
        answer.applicable, answer.written, answer.summary
    )
}

/// Declared outputs a `success` report claims that are missing from the tree.
///
/// `failure` reports are already parked; their output claims are not re-checked.
#[must_use]
pub fn missing_outputs(report: &Report, tree_root: &Path) -> Vec<String> {
    if report.status == Status::Failure {
        return Vec::new();
    }
    report
        .outputs
        .iter()
        .filter(|output| !tree_root.join(&output.path).exists())
        .map(|output| {
            format!("- declared output `{}` does not exist in the working tree", output.path)
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
