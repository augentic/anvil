//! Judgment-answer schemas, deserializers, and source-axis validation tails.

use serde::Deserialize;

use crate::seam::{
    BuildOutput, ClaimKind, Error, Evidence, Finding, Lead, PhaseFinding, PhaseOutcome,
    PhaseReport, PhaseSource, PhaseWrite, Report, Severity, Status, UiSurface,
};

/// Answer schema gating `survey` replies.
pub const LEADS_ANSWER_SCHEMA: &str = include_str!("../schemas/answers/leads.schema.json");

/// Answer schema gating `extract` replies.
pub const EVIDENCE_ANSWER_SCHEMA: &str = include_str!("../schemas/answers/evidence.schema.json");

/// Answer schema gating `build` / `merge` replies.
pub const REPORT_ANSWER_SCHEMA: &str = include_str!("../schemas/answers/report.schema.json");

/// Answer schema gating the RFC-90 phase replies (`build` / `repair`
/// / `verify` / `review`).
pub const PHASE_REPORT_ANSWER: &str = include_str!("../schemas/answers/phase-report.schema.json");

/// `survey` answer envelope (`{ "leads": [...] }`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct LeadsAnswer {
    /// Leads in source order.
    pub leads: Vec<Lead>,
}

/// # Errors
///
/// When the answer does not parse into `{ "leads": [...] }`.
pub fn parse_leads(answer: &str) -> Result<Vec<Lead>, serde_json::Error> {
    serde_json::from_str::<LeadsAnswer>(answer).map(|envelope| envelope.leads)
}

/// # Errors
///
/// When the answer does not parse into Evidence.
pub fn parse_evidence(answer: &str) -> Result<Evidence, serde_json::Error> {
    serde_json::from_str(answer)
}

// Schemas leave these as plain strings; we enforce the grammars in-guest.
const KEBAB_PATTERN: &str = "^[a-z0-9]+(-[a-z0-9]+)*$";
const DOTTED_KEBAB_PATTERN: &str = "^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$";

fn is_kebab(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|seg| {
            !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

fn is_dotted_kebab(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(is_kebab)
}

fn enforce(operation: &str, findings: &[String]) -> Result<(), Error> {
    if findings.is_empty() {
        return Ok(());
    }
    Err(Error::Internal(format!(
        "{operation} answer failed deterministic validation:\n{}",
        findings.join("\n")
    )))
}

/// Deterministic post-host-gate check: kebab lead/topic ids, non-empty synopsis.
///
/// # Errors
///
/// [`Error::Internal`] with one findings-style line per violation.
pub fn validate_leads(leads: &[Lead]) -> Result<(), Error> {
    let mut findings = Vec::new();
    for lead in leads {
        if !is_kebab(&lead.lead) {
            findings.push(format!("- lead `{}`: id does not match `{KEBAB_PATTERN}`", lead.lead));
        }
        if lead.synopsis.trim().is_empty() {
            findings.push(format!("- lead `{}`: synopsis is empty", lead.lead));
        }
        for topic in &lead.topics {
            if !is_kebab(topic) {
                findings.push(format!(
                    "- lead `{}`: topic `{topic}` does not match `{KEBAB_PATTERN}`",
                    lead.lead
                ));
            }
        }
    }
    enforce("survey", &findings)
}

/// Typed parse + [`validate_leads`] — the [`crate::repaired`] tail.
///
/// # Errors
///
/// [`Error::Internal`] on parse or validation failure.
pub fn leads_tail(answer: &str) -> Result<Vec<Lead>, Error> {
    let leads = parse_leads(answer)
        .map_err(|err| Error::Internal(format!("leads answer did not deserialize: {err}")))?;
    validate_leads(&leads)?;
    Ok(leads)
}

/// Deterministic post-host-gate check: dotted-kebab claim ids where required.
///
/// # Errors
///
/// [`Error::Internal`] with one findings-style line per violation.
pub fn validate_evidence(evidence: &Evidence) -> Result<(), Error> {
    let mut findings = Vec::new();
    for (index, claim) in evidence.claims.iter().enumerate() {
        match &claim.id {
            Some(id) if !is_dotted_kebab(id) => {
                findings.push(format!(
                    "- claim {index}: id `{id}` does not match `{DOTTED_KEBAB_PATTERN}`"
                ));
            }
            None if matches!(
                claim.kind,
                ClaimKind::Requirement | ClaimKind::Criterion | ClaimKind::Example
            ) =>
            {
                findings.push(format!("- claim {index}: `{:?}` claims require an id", claim.kind));
            }
            _ => {}
        }
    }
    enforce("extract", &findings)
}

/// Typed parse + [`validate_evidence`] — the [`crate::repaired`] tail.
///
/// # Errors
///
/// [`Error::Internal`] on parse or validation failure.
pub fn evidence_tail(answer: &str) -> Result<Evidence, Error> {
    let evidence = parse_evidence(answer)
        .map_err(|err| Error::Internal(format!("evidence answer did not deserialize: {err}")))?;
    validate_evidence(&evidence)?;
    Ok(evidence)
}

/// Slice of `diagnostic.schema.json` the seam projects into [`Finding`].
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Diagnostic {
    /// Codex citation, if any.
    #[serde(default)]
    pub rule_id: Option<String>,
    /// Short finding title.
    pub title: String,
    /// Review severity.
    pub severity: Severity,
    /// Operator-facing risk.
    pub impact: String,
    /// Action to clear the finding.
    pub remediation: String,
}

impl Diagnostic {
    /// Collapse `title` / `impact` / `remediation` into [`Finding::detail`].
    #[must_use]
    pub fn into_finding(self) -> Finding {
        Finding {
            rule_id: self.rule_id,
            severity: self.severity,
            detail: format!("{} — {}; remediation: {}", self.title, self.impact, self.remediation),
        }
    }
}

/// One RFC-90 phase answer body (`build` / `repair` / `verify` /
/// `review`): the phase report minus the adapter-attached
/// `next-continuation` — opaque session bytes never come from the
/// model answer.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct PhaseReportAnswer {
    /// Adapter-selected phase outcome.
    pub outcome: PhaseOutcome,
    /// Required report-level assurance claim.
    pub source: PhaseSource,
    /// Full structured findings.
    #[serde(default)]
    pub findings: Vec<PhaseFinding>,
    /// Candidate per-platform build outputs (`build` only).
    #[serde(default)]
    pub outputs: Vec<BuildOutput>,
    /// Optional UI-surface signal (`build` only).
    #[serde(default)]
    pub ui_surface: Option<UiSurface>,
    /// Slice-local requirement ids the phase claims to have
    /// implemented (`build` only); must never name a deferred
    /// requirement (RFC-86a D4).
    #[serde(default)]
    pub covered: Vec<String>,
    /// Audit-evidence writes performed by the phase.
    #[serde(default)]
    pub written: Vec<PhaseWrite>,
}

impl PhaseReportAnswer {
    /// # Errors
    ///
    /// When the answer does not parse into the phase-report shape.
    pub fn parse(answer: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(answer)
    }

    /// Project onto the seam-facing [`PhaseReport`], carrying no
    /// continuation change (the adapter attaches one separately when
    /// it has session state to preserve).
    #[must_use]
    pub fn into_report(self) -> PhaseReport {
        PhaseReport {
            outcome: self.outcome,
            source: self.source,
            findings: self.findings,
            outputs: self.outputs,
            ui_surface: self.ui_surface,
            covered: self.covered,
            written: self.written,
            next_continuation: None,
        }
    }
}

/// `merge` answer body.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct ReportAnswer {
    /// Operation outcome.
    pub status: Status,
    /// Structured diagnostics.
    #[serde(default)]
    pub findings: Vec<Diagnostic>,
    /// Per-platform build outputs.
    #[serde(default)]
    pub outputs: Vec<BuildOutput>,
    /// Optional UI-surface signal.
    #[serde(default)]
    pub ui_surface: Option<UiSurface>,
}

impl ReportAnswer {
    /// # Errors
    ///
    /// When the answer does not parse into the report shape.
    pub fn parse(answer: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(answer)
    }

    /// Project onto the compact seam-facing [`Report`].
    #[must_use]
    pub fn into_report(self) -> Report {
        Report {
            status: self.status,
            findings: self.findings.into_iter().map(Diagnostic::into_finding).collect(),
            outputs: self.outputs,
            ui_surface: self.ui_surface,
        }
    }
}
