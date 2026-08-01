//! SDK-seam to engine-seam DTO conversion — the one native copy of
//! the mapping the wasm guest shim applies at the WIT boundary.
//!
//! [`crate::catalog`] projects adapter metadata through it at
//! registration; [`crate::provider::Provider`] maps every operation's
//! values through it at dispatch. Fixture and adapter crates stay on
//! the SDK DTOs and never repeat this mapping.

use adapter::seam as aseam;
use artifacts::evidence::AuthorityClass;
use diagnostics::{Diagnostic, Severity};
use project::adapter::metadata::Metadata;
use project::adapter::{BuildInputDeclaration, PlatformsCapability};
use project::seam::wire::{BuildOutput, BuildReport, BuildStatus, UiSurface, build_finding};
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

/// Narrow a workflow working tree to the SDK tree.
#[must_use]
pub fn narrow_tree(tree: seam::WorkingTree) -> aseam::WorkingTree {
    aseam::WorkingTree {
        base: tree.base,
        subpath: tree.subpath,
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
    }
}
