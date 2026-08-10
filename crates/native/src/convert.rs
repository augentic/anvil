//! SDK-seam to engine-seam DTO conversion — the one native copy of
//! the mapping the wasm guest shim applies at the WIT boundary.
//! Fixture and adapter crates never repeat this mapping.

use adapter::seam as aseam;
use artifacts::evidence::AuthorityClass;
use diagnostics::{
    Artifact, Confidence, Diagnostic, DiagnosticKind, DiagnosticSource, FindingEvidence,
    FindingLocation, Severity,
};
use project::adapter::metadata::Metadata;
use project::adapter::{
    BuildInputDeclaration, PlatformsCapability, WritableArtifactDeclaration, WritableArtifactKind,
};
use project::seam::wire::{
    BuildOutput, BuildReport, BuildStatus, PhaseOutcome, PhaseReport, PhaseRoot, PhaseSource,
    PhaseWrite, RepairOrigin, UiSurface, build_finding,
};
use project::seam::{self, BuildContext, Evidence, Input, Lead};

/// Widen an SDK operation error to the engine seam error.
#[must_use]
pub fn error(error: aseam::Error) -> seam::Error {
    match error {
        aseam::Error::InvalidRequest(detail) => seam::Error::InvalidRequest(detail),
        aseam::Error::Io(detail) => seam::Error::Io(detail),
        aseam::Error::Internal(detail) => seam::Error::Internal(detail),
    }
}

/// Widen an SDK lead to the engine lead.
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

/// Widen SDK evidence to the engine evidence document.
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

/// Narrow a workflow workspace to the SDK workspace.
#[must_use]
pub fn narrow_workspace(workspace: seam::Workspace) -> aseam::Workspace {
    aseam::Workspace {
        id: workspace.id,
        root: workspace.root,
        artifacts: workspace.artifacts,
        artifact_stage: workspace.artifact_stage.map(|stage| aseam::ArtifactStage {
            id: stage.id,
            root: stage.root,
        }),
    }
}

/// Narrow a workflow repair origin to the SDK origin.
#[must_use]
pub const fn narrow_origin(origin: RepairOrigin) -> aseam::RepairOrigin {
    match origin {
        RepairOrigin::Verification => aseam::RepairOrigin::Verification,
        RepairOrigin::Review => aseam::RepairOrigin::Review,
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
    let payload = |body: seam::Payload| match body {
        seam::Payload::Path(path) => aseam::Payload::Path(path),
        seam::Payload::Body(text) => aseam::Payload::Body(text),
    };
    match input {
        Input::Proposal(body) => aseam::Input::Proposal(payload(body)),
        Input::Design(body) => aseam::Input::Design(payload(body)),
        Input::Tasks(body) => aseam::Input::Tasks(payload(body)),
        Input::Spec(body) => aseam::Input::Spec(payload(body)),
        Input::Other(body) => aseam::Input::Other(payload(body)),
    }
}

/// Narrow a workflow build context to the SDK context.
#[must_use]
pub fn narrow_context(context: BuildContext) -> aseam::BuildContext {
    aseam::BuildContext {
        sources: context.sources,
    }
}

/// Widen a seam report to the stamped `BuildReport` envelope — the
/// same stamping the engine's guest shim applies to a WIT report.
#[must_use]
pub fn widen_report(id: &str, slice: String, report: aseam::Report) -> BuildReport {
    BuildReport::stamped(
        id,
        slice,
        match report.status {
            aseam::Status::Success => BuildStatus::Success,
            aseam::Status::Failure => BuildStatus::Failure,
        },
        report.findings.into_iter().map(finding).collect(),
        report
            .outputs
            .into_iter()
            .map(|output| BuildOutput {
                platform: platform(output.platform),
                path: output.path,
            })
            .collect(),
        report.ui_surface.map(|surface| UiSurface {
            screens: surface.screens,
        }),
    )
}

fn finding(finding: aseam::Finding) -> Diagnostic {
    build_finding(finding.rule_id, finding.detail, severity(finding.severity))
}

const fn severity(severity: aseam::Severity) -> Severity {
    match severity {
        aseam::Severity::Critical => Severity::Critical,
        aseam::Severity::Important => Severity::Important,
        aseam::Severity::Suggestion => Severity::Suggestion,
        aseam::Severity::Optional => Severity::Optional,
    }
}

const fn narrow_severity(severity: Severity) -> aseam::Severity {
    match severity {
        Severity::Critical => aseam::Severity::Critical,
        Severity::Important => aseam::Severity::Important,
        Severity::Suggestion => aseam::Severity::Suggestion,
        Severity::Optional => aseam::Severity::Optional,
    }
}

/// Widen an SDK phase report to the engine wire shape — the
/// isomorphic RFC-90 D2 projection: nothing folds at this seam.
#[must_use]
pub fn phase_report(report: aseam::PhaseReport) -> PhaseReport {
    PhaseReport {
        outcome: match report.outcome {
            aseam::PhaseOutcome::Completed => PhaseOutcome::Completed,
            aseam::PhaseOutcome::NotApplicable => PhaseOutcome::NotApplicable,
        },
        source: match report.source {
            aseam::PhaseSource::Deterministic => PhaseSource::Deterministic,
            aseam::PhaseSource::ModelAssisted => PhaseSource::ModelAssisted,
            aseam::PhaseSource::Hybrid => PhaseSource::Hybrid,
            aseam::PhaseSource::Tool => PhaseSource::Tool,
        },
        findings: report.findings.into_iter().map(phase_finding).collect(),
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
        written: report
            .written
            .into_iter()
            .map(|write| PhaseWrite {
                root: match write.root {
                    aseam::PhaseRoot::Workspace => PhaseRoot::Workspace,
                    aseam::PhaseRoot::Artifacts => PhaseRoot::Artifacts,
                },
                path: write.path,
            })
            .collect(),
        next_continuation: report.next_continuation,
    }
}

/// Widen an SDK phase finding to the full [`Diagnostic`] shape. The
/// engine stamps `target_adapter` / `slice` / change identity and
/// recomputes the fingerprint; this projection keeps every field
/// as-given.
fn phase_finding(finding: aseam::PhaseFinding) -> Diagnostic {
    Diagnostic {
        id: finding.id,
        rule_id: finding.rule_id,
        related_rule_ids: (!finding.related_rule_ids.is_empty())
            .then_some(finding.related_rule_ids),
        title: finding.title,
        severity: severity(finding.severity),
        source: diagnostic_source(finding.source),
        kind: match finding.kind {
            aseam::FindingKind::Violation => DiagnosticKind::Violation,
            aseam::FindingKind::Review => DiagnosticKind::Review,
        },
        target_adapter: None,
        source_adapter: None,
        slice: None,
        change: None,
        artifact: artifact(finding.artifact),
        location: finding.location.map(location),
        evidence: evidence_union(finding.evidence),
        impact: finding.impact,
        remediation: finding.remediation,
        confidence: finding.confidence.map(|confidence| match confidence {
            aseam::FindingConfidence::High => Confidence::High,
            aseam::FindingConfidence::Medium => Confidence::Medium,
            aseam::FindingConfidence::Low => Confidence::Low,
        }),
        fingerprint: finding.fingerprint,
    }
}

/// Narrow a [`Diagnostic`] to the SDK phase finding for a repair
/// brief. The engine-stamped identity fields (`target_adapter` /
/// `source_adapter` / `slice` / `change`) do not exist on the SDK
/// record and are dropped.
#[must_use]
pub fn narrow_finding(diagnostic: Diagnostic) -> aseam::PhaseFinding {
    aseam::PhaseFinding {
        id: diagnostic.id,
        rule_id: diagnostic.rule_id,
        related_rule_ids: diagnostic.related_rule_ids.unwrap_or_default(),
        title: diagnostic.title,
        severity: narrow_severity(diagnostic.severity),
        source: narrow_diagnostic_source(diagnostic.source),
        kind: match diagnostic.kind {
            DiagnosticKind::Violation => aseam::FindingKind::Violation,
            DiagnosticKind::Review => aseam::FindingKind::Review,
        },
        artifact: narrow_artifact(diagnostic.artifact),
        location: diagnostic.location.map(narrow_location),
        evidence: narrow_evidence(diagnostic.evidence),
        impact: diagnostic.impact,
        remediation: diagnostic.remediation,
        confidence: diagnostic.confidence.map(|confidence| match confidence {
            Confidence::High => aseam::FindingConfidence::High,
            Confidence::Medium => aseam::FindingConfidence::Medium,
            Confidence::Low => aseam::FindingConfidence::Low,
        }),
        fingerprint: diagnostic.fingerprint,
    }
}

const fn diagnostic_source(source: aseam::DiagnosticSource) -> DiagnosticSource {
    match source {
        aseam::DiagnosticSource::Deterministic => DiagnosticSource::Deterministic,
        aseam::DiagnosticSource::ModelAssisted => DiagnosticSource::ModelAssisted,
        aseam::DiagnosticSource::Hybrid => DiagnosticSource::Hybrid,
        aseam::DiagnosticSource::Human => DiagnosticSource::Human,
        aseam::DiagnosticSource::Tool => DiagnosticSource::Tool,
    }
}

const fn narrow_diagnostic_source(source: DiagnosticSource) -> aseam::DiagnosticSource {
    match source {
        DiagnosticSource::Deterministic => aseam::DiagnosticSource::Deterministic,
        DiagnosticSource::ModelAssisted => aseam::DiagnosticSource::ModelAssisted,
        DiagnosticSource::Hybrid => aseam::DiagnosticSource::Hybrid,
        DiagnosticSource::Human => aseam::DiagnosticSource::Human,
        DiagnosticSource::Tool => aseam::DiagnosticSource::Tool,
    }
}

const fn artifact(artifact: aseam::FindingArtifact) -> Artifact {
    match artifact {
        aseam::FindingArtifact::Code => Artifact::Code,
        aseam::FindingArtifact::Tests => Artifact::Tests,
        aseam::FindingArtifact::Contracts => Artifact::Contracts,
        aseam::FindingArtifact::Specs => Artifact::Specs,
        aseam::FindingArtifact::Design => Artifact::Design,
        aseam::FindingArtifact::Decisions => Artifact::Decisions,
        aseam::FindingArtifact::Tasks => Artifact::Tasks,
        aseam::FindingArtifact::Assets => Artifact::Assets,
        aseam::FindingArtifact::Tokens => Artifact::Tokens,
        aseam::FindingArtifact::Composition => Artifact::Composition,
        aseam::FindingArtifact::Plan => Artifact::Plan,
        aseam::FindingArtifact::Unknown => Artifact::Unknown,
    }
}

const fn narrow_artifact(artifact: Artifact) -> aseam::FindingArtifact {
    match artifact {
        Artifact::Code => aseam::FindingArtifact::Code,
        Artifact::Tests => aseam::FindingArtifact::Tests,
        Artifact::Contracts => aseam::FindingArtifact::Contracts,
        Artifact::Specs => aseam::FindingArtifact::Specs,
        Artifact::Design => aseam::FindingArtifact::Design,
        Artifact::Decisions => aseam::FindingArtifact::Decisions,
        Artifact::Tasks => aseam::FindingArtifact::Tasks,
        Artifact::Assets => aseam::FindingArtifact::Assets,
        Artifact::Tokens => aseam::FindingArtifact::Tokens,
        Artifact::Composition => aseam::FindingArtifact::Composition,
        Artifact::Plan => aseam::FindingArtifact::Plan,
        Artifact::Unknown => aseam::FindingArtifact::Unknown,
    }
}

fn location(location: aseam::PhaseLocation) -> FindingLocation {
    FindingLocation {
        path: location.path,
        line: location.line,
        column: location.column,
        end_line: location.end_line,
        end_column: location.end_column,
    }
}

fn narrow_location(location: FindingLocation) -> aseam::PhaseLocation {
    aseam::PhaseLocation {
        path: location.path,
        line: location.line,
        column: location.column,
        end_line: location.end_line,
        end_column: location.end_column,
    }
}

// The SDK evidence union already carries a `serde_json::Value`
// payload, so both directions map structurally.
fn evidence_union(evidence: aseam::FindingEvidence) -> FindingEvidence {
    match evidence {
        aseam::FindingEvidence::Snippet { value } => FindingEvidence::Snippet { value },
        aseam::FindingEvidence::Digest {
            sha256,
            summary,
            locations,
        } => FindingEvidence::Digest {
            sha256,
            summary,
            locations: locations.map(|locations| locations.into_iter().map(location).collect()),
        },
        aseam::FindingEvidence::Structured {
            summary,
            data,
            locations,
        } => FindingEvidence::Structured {
            summary,
            data,
            locations: locations.map(|locations| locations.into_iter().map(location).collect()),
        },
    }
}

fn narrow_evidence(evidence: FindingEvidence) -> aseam::FindingEvidence {
    match evidence {
        FindingEvidence::Snippet { value } => aseam::FindingEvidence::Snippet { value },
        FindingEvidence::Digest {
            sha256,
            summary,
            locations,
        } => aseam::FindingEvidence::Digest {
            sha256,
            summary,
            locations: locations
                .map(|locations| locations.into_iter().map(narrow_location).collect()),
        },
        FindingEvidence::Structured {
            summary,
            data,
            locations,
        } => aseam::FindingEvidence::Structured {
            summary,
            data,
            locations: locations
                .map(|locations| locations.into_iter().map(narrow_location).collect()),
        },
    }
}

/// Widen an SDK platform to the engine platform enum.
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

/// Project SDK source metadata onto the engine resolver metadata.
#[must_use]
pub fn source_metadata(record: aseam::SourceMetadata) -> Metadata {
    Metadata {
        emery_floor: record.emery_floor,
        inputs: Vec::new(),
        platforms: None,
        writable_artifacts: Vec::new(),
    }
}

/// Project SDK target metadata onto the engine resolver metadata.
#[must_use]
pub fn target_metadata(record: aseam::TargetMetadata) -> Metadata {
    Metadata {
        emery_floor: record.emery_floor,
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
        writable_artifacts: record
            .writable_artifacts
            .into_iter()
            .map(|artifact| WritableArtifactDeclaration {
                path: artifact.path,
                kind: match artifact.kind {
                    aseam::WritableArtifactKind::File => WritableArtifactKind::File,
                    aseam::WritableArtifactKind::Tree => WritableArtifactKind::Tree,
                },
            })
            .collect(),
    }
}
