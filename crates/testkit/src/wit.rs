//! The `adapter`-world WIT export bindings the examples guest shims over.
//!
//! Mirrors the adapter SDK's per-axis bindings (`crates/adapter`): one
//! `wit_bindgen::generate!` with `pub_export_macro`, flat re-exports,
//! and the [`From`] impls between the generated records and the SDK's
//! seam DTOs ([`adapter::seam`]). The combined `adapter` world stays
//! Specify-owned — the SDK's `source!` / `target!` macros export one
//! axis each — so the fixture guest implements the axis `Guest` traits
//! itself, wires them in with `testkit::wit::export!(Adapter
//! with_types_in testkit::wit)`, and dispatches every operation through
//! the canonical [`crate::fixture`] trait implementors.

mod generated {
    #![allow(
        missing_docs,
        unsafe_code,
        clippy::pedantic,
        clippy::nursery,
        reason = "wit-bindgen generated bindings are not hand-maintained; the generated code cannot carry this workspace's lint posture"
    )]

    wit_bindgen::generate!({
        world: "adapter",
        path: "../../wit",
        generate_all,
        pub_export_macro: true,
    });
}

use adapter::seam as aseam;
pub use generated::*;
use project::platform;

use self::generated::exports::specify::adapter::{source, target};

impl From<aseam::Error> for source::Error {
    fn from(error: aseam::Error) -> Self {
        match error {
            aseam::Error::InvalidRequest(detail) => Self::InvalidRequest(detail),
            aseam::Error::Io(detail) => Self::Io(detail),
            aseam::Error::Internal(detail) => Self::Internal(detail),
        }
    }
}

// The engine-seam error, for the one operation (id-keyed guidance)
// the guest still routes through the fixture core directly.
impl From<project::seam::Error> for source::Error {
    fn from(error: project::seam::Error) -> Self {
        match error {
            project::seam::Error::InvalidRequest(detail) => Self::InvalidRequest(detail),
            project::seam::Error::Io(detail) => Self::Io(detail),
            project::seam::Error::Internal(detail) => Self::Internal(detail),
        }
    }
}

impl From<source::Lead> for aseam::Lead {
    fn from(lead: source::Lead) -> Self {
        Self {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        }
    }
}

impl From<aseam::Lead> for source::Lead {
    fn from(lead: aseam::Lead) -> Self {
        Self {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        }
    }
}

impl From<aseam::Evidence> for source::Evidence {
    fn from(evidence: aseam::Evidence) -> Self {
        Self {
            authority: evidence.authority.into(),
            claims: evidence.claims.into_iter().map(source::Claim::from).collect(),
        }
    }
}

impl From<aseam::Authority> for source::Authority {
    fn from(authority: aseam::Authority) -> Self {
        match authority {
            aseam::Authority::Intent => Self::Intent,
            aseam::Authority::Documentation => Self::Documentation,
            aseam::Authority::Behaviour => Self::Behaviour,
        }
    }
}

impl From<aseam::Claim> for source::Claim {
    fn from(claim: aseam::Claim) -> Self {
        Self {
            kind: claim.kind.into(),
            id: claim.id,
            path: claim.path,
            synopsis: claim.synopsis,
            backing: claim.backing.map(source::Backing::from),
        }
    }
}

impl From<aseam::ClaimKind> for source::ClaimKind {
    fn from(kind: aseam::ClaimKind) -> Self {
        match kind {
            aseam::ClaimKind::Intent => Self::Intent,
            aseam::ClaimKind::Requirement => Self::Requirement,
            aseam::ClaimKind::Criterion => Self::Criterion,
            aseam::ClaimKind::Decision => Self::Decision,
            aseam::ClaimKind::Section => Self::Section,
            aseam::ClaimKind::Diagram => Self::Diagram,
            aseam::ClaimKind::Contract => Self::Contract,
            aseam::ClaimKind::Example => Self::Example,
            aseam::ClaimKind::Excerpt => Self::Excerpt,
            aseam::ClaimKind::Type => Self::Type,
            aseam::ClaimKind::Call => Self::Call,
            aseam::ClaimKind::Region => Self::Region,
            aseam::ClaimKind::Container => Self::Container,
            aseam::ClaimKind::Leaf => Self::Leaf,
        }
    }
}

impl From<aseam::Backing> for source::Backing {
    fn from(backing: aseam::Backing) -> Self {
        match backing {
            aseam::Backing::Payload(payload) => Self::Payload(payload),
            aseam::Backing::Path(path) => Self::Path(path),
        }
    }
}

impl From<project::adapter::PlatformsCapability> for target::PlatformsCapability {
    fn from(capability: project::adapter::PlatformsCapability) -> Self {
        Self {
            required: capability.required,
            allowed: capability.allowed.into_iter().map(target::Platform::from).collect(),
            default: capability.default.into_iter().map(target::Platform::from).collect(),
        }
    }
}

impl From<platform::Platform> for target::Platform {
    fn from(platform: platform::Platform) -> Self {
        match platform {
            platform::Platform::Core => Self::Core,
            platform::Platform::Ios => Self::Ios,
            platform::Platform::Android => Self::Android,
            platform::Platform::Web => Self::Web,
            platform::Platform::Desktop => Self::Desktop,
        }
    }
}

impl From<target::Input> for aseam::Input {
    fn from(input: target::Input) -> Self {
        match input {
            target::Input::Proposal(body) => Self::Proposal(body),
            target::Input::Design(body) => Self::Design(body),
            target::Input::Tasks(body) => Self::Tasks(body),
            target::Input::Spec(body) => Self::Spec(body),
            target::Input::Other(body) => Self::Other(body),
        }
    }
}

impl From<target::MergePhase> for aseam::MergePhase {
    fn from(phase: target::MergePhase) -> Self {
        match phase {
            target::MergePhase::Preflight => Self::Preflight,
            target::MergePhase::Postflight => Self::Postflight,
        }
    }
}

impl From<target::WorkingTree> for aseam::WorkingTree {
    fn from(tree: target::WorkingTree) -> Self {
        Self {
            base: tree.base,
            subpath: tree.subpath,
        }
    }
}

impl From<aseam::Report> for target::Report {
    fn from(report: aseam::Report) -> Self {
        Self {
            status: report.status.into(),
            findings: report.findings.into_iter().map(target::Finding::from).collect(),
            outputs: report.outputs.into_iter().map(target::BuildOutput::from).collect(),
            ui_surface: report.ui_surface.map(|surface| target::UiSurface {
                screens: surface.screens,
            }),
        }
    }
}

impl From<aseam::Status> for target::Status {
    fn from(status: aseam::Status) -> Self {
        match status {
            aseam::Status::Success => Self::Success,
            aseam::Status::Failure => Self::Failure,
        }
    }
}

impl From<aseam::Finding> for target::Finding {
    fn from(finding: aseam::Finding) -> Self {
        Self {
            rule_id: finding.rule_id,
            severity: finding.severity.into(),
            detail: finding.detail,
        }
    }
}

impl From<aseam::Severity> for target::Severity {
    fn from(severity: aseam::Severity) -> Self {
        match severity {
            aseam::Severity::Critical => Self::Critical,
            aseam::Severity::Important => Self::Important,
            aseam::Severity::Suggestion => Self::Suggestion,
            aseam::Severity::Optional => Self::Optional,
        }
    }
}

impl From<aseam::BuildOutput> for target::BuildOutput {
    fn from(output: aseam::BuildOutput) -> Self {
        Self {
            platform: output.platform.into(),
            path: output.path,
        }
    }
}

impl From<aseam::Platform> for target::Platform {
    fn from(platform: aseam::Platform) -> Self {
        match platform {
            aseam::Platform::Core => Self::Core,
            aseam::Platform::Ios => Self::Ios,
            aseam::Platform::Android => Self::Android,
            aseam::Platform::Web => Self::Web,
            aseam::Platform::Desktop => Self::Desktop,
        }
    }
}
