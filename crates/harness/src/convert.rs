//! SDK-seam to workflow-seam DTO conversion — the one native copy of
//! the mapping the wasm guest shim applies at the WIT boundary.
//!
//! [`crate::catalog`] projects adapter metadata through it at
//! registration; [`crate::provider::Provider`] maps every operation's
//! values through it at dispatch. Fixture and adapter crates stay on
//! the SDK DTOs and never repeat this mapping.

use adapter::seam as aseam;
use artifacts::evidence::AuthorityClass;
use diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use project::adapter::metadata::Metadata;
use project::adapter::{BuildInputDeclaration, PlatformsCapability};
use project::seam::wire::{BUILD_VERSION, BuildOutput, BuildReport, BuildStatus, UiSurface};
use project::seam::{self, Evidence, Input, Lead};

/// Widen an SDK operation error to the workflow seam error.
#[must_use]
pub fn error(error: aseam::Error) -> seam::Error {
    match error {
        aseam::Error::InvalidRequest(detail) => seam::Error::InvalidRequest(detail),
        aseam::Error::Io(detail) => seam::Error::Io(detail),
        aseam::Error::Internal(detail) => seam::Error::Internal(detail),
    }
}

/// Widen an SDK lead to the workflow lead.
#[must_use]
pub fn lead(lead: aseam::Lead) -> Lead {
    Lead {
        lead: lead.lead,
        synopsis: lead.synopsis,
        topics: lead.topics,
    }
}

/// Narrow a workflow lead to the SDK lead.
#[must_use]
pub fn narrow_lead(lead: Lead) -> aseam::Lead {
    aseam::Lead {
        lead: lead.lead,
        synopsis: lead.synopsis,
        topics: lead.topics,
    }
}

/// Widen SDK evidence to the workflow evidence document.
#[must_use]
pub fn evidence(evidence: aseam::Evidence) -> Evidence {
    Evidence {
        authority: authority(evidence.authority),
        claims: evidence.claims.into_iter().map(claim).collect(),
    }
}

const fn authority(authority: aseam::Authority) -> AuthorityClass {
    match authority {
        aseam::Authority::Intent => AuthorityClass::Intent,
        aseam::Authority::Documentation => AuthorityClass::Documentation,
        aseam::Authority::Behaviour => AuthorityClass::Behaviour,
    }
}

// Open per-kind claim fields do not cross the compact seam record.
fn claim(claim: aseam::Claim) -> artifacts::evidence::Claim {
    let mut typed = artifacts::evidence::Claim::new(claim_kind(claim.kind));
    typed.id = claim.id;
    typed.path = claim.path;
    typed.synopsis = claim.synopsis;
    typed.set_backing(claim.backing.map(|backing| match backing {
        aseam::Backing::Payload(payload) => artifacts::evidence::Backing::Payload(payload),
        aseam::Backing::Path(path) => artifacts::evidence::Backing::Path(path),
    }));
    typed
}

const fn claim_kind(kind: aseam::ClaimKind) -> artifacts::evidence::ClaimKind {
    use artifacts::evidence::ClaimKind;
    match kind {
        aseam::ClaimKind::Intent => ClaimKind::Intent,
        aseam::ClaimKind::Requirement => ClaimKind::Requirement,
        aseam::ClaimKind::Criterion => ClaimKind::Criterion,
        aseam::ClaimKind::Decision => ClaimKind::Decision,
        aseam::ClaimKind::Section => ClaimKind::Section,
        aseam::ClaimKind::Diagram => ClaimKind::Diagram,
        aseam::ClaimKind::Contract => ClaimKind::Contract,
        aseam::ClaimKind::Example => ClaimKind::Example,
        aseam::ClaimKind::Excerpt => ClaimKind::Excerpt,
        aseam::ClaimKind::Type => ClaimKind::Type,
        aseam::ClaimKind::Call => ClaimKind::Call,
        aseam::ClaimKind::Region => ClaimKind::Region,
        aseam::ClaimKind::Container => ClaimKind::Container,
        aseam::ClaimKind::Leaf => ClaimKind::Leaf,
    }
}

/// Narrow a workflow merge phase to the SDK phase.
#[must_use]
pub const fn narrow_phase(phase: seam::MergePhase) -> aseam::MergePhase {
    match phase {
        seam::MergePhase::Preflight => aseam::MergePhase::Preflight,
        seam::MergePhase::Postflight => aseam::MergePhase::Postflight,
    }
}

/// Narrow a workflow input to the SDK input.
#[must_use]
pub fn narrow_input(input: Input) -> aseam::Input {
    match input {
        Input::Proposal(body) => aseam::Input::Proposal(body),
        Input::Design(body) => aseam::Input::Design(body),
        Input::Tasks(body) => aseam::Input::Tasks(body),
        Input::Spec(body) => aseam::Input::Spec(body),
        Input::Other(body) => aseam::Input::Other(body),
    }
}

/// Widen a seam report to the stamped `BuildReport` envelope — the
/// same stamping the engine's guest shim applies to a WIT report.
#[must_use]
pub fn widen_report(id: &str, slice: String, report: aseam::Report) -> BuildReport {
    BuildReport {
        version: BUILD_VERSION,
        slice,
        target: id.strip_prefix("target:").unwrap_or(id).to_string(),
        status: match report.status {
            aseam::Status::Success => BuildStatus::Success,
            aseam::Status::Failure => BuildStatus::Failure,
        },
        findings: report.findings.into_iter().map(finding).collect(),
        outputs: report
            .outputs
            .into_iter()
            .map(|output| BuildOutput {
                platform: platform(output.platform),
                path: output.path,
            })
            .collect(),
        ui_surface: report.ui_surface.map(|surface| UiSurface {
            screens: surface.screens,
        }),
    }
}

// The folded `detail` prose serves as title, impact, and remediation.
fn finding(finding: aseam::Finding) -> Diagnostic {
    let mut diagnostic = Diagnostic::finding(
        finding.rule_id.clone().unwrap_or_else(|| "target-build-finding".to_string()),
        finding.detail.clone(),
        finding.detail,
        severity(finding.severity),
        DiagnosticKind::Violation,
        DiagnosticSource::ModelAssisted,
        Artifact::Code,
        None,
    );
    diagnostic.rule_id = finding.rule_id;
    diagnostic.fingerprint = diagnostics::fingerprint(&diagnostic);
    diagnostic
}

const fn severity(severity: aseam::Severity) -> Severity {
    match severity {
        aseam::Severity::Critical => Severity::Critical,
        aseam::Severity::Important => Severity::Important,
        aseam::Severity::Suggestion => Severity::Suggestion,
        aseam::Severity::Optional => Severity::Optional,
    }
}

/// Widen an SDK platform to the workflow platform enum.
#[must_use]
pub const fn platform(platform: aseam::Platform) -> project::platform::Platform {
    use project::platform::Platform;
    match platform {
        aseam::Platform::Core => Platform::Core,
        aseam::Platform::Ios => Platform::Ios,
        aseam::Platform::Android => Platform::Android,
        aseam::Platform::Web => Platform::Web,
        aseam::Platform::Desktop => Platform::Desktop,
    }
}

/// Project SDK source metadata onto the workflow resolver metadata.
#[must_use]
pub fn source_metadata(record: aseam::SourceMetadata) -> Metadata {
    Metadata {
        specify_floor: record.specify_floor,
        inputs: Vec::new(),
        platforms: None,
    }
}

/// Project SDK target metadata onto the workflow resolver metadata.
#[must_use]
pub fn target_metadata(record: aseam::TargetMetadata) -> Metadata {
    Metadata {
        specify_floor: record.specify_floor,
        inputs: record
            .inputs
            .into_iter()
            .map(|input| BuildInputDeclaration {
                path: input.path,
                required: input.required,
            })
            .collect(),
        platforms: record.platforms.map(|capability| PlatformsCapability {
            required: capability.required,
            allowed: capability.allowed.into_iter().map(platform).collect(),
            default: capability.default.into_iter().map(platform).collect(),
        }),
    }
}
