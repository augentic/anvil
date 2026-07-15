//! The `adapter`-world WIT export bindings the examples guest shims over.
//!
//! Mirrors the shared bindings crate real adapters use
//! (`crates/adapter` in `augentic/specify-adapters`): one
//! `wit_bindgen::generate!` with `pub_export_macro`, flat re-exports,
//! and the [`From`] impls between the generated records and the
//! engine's own seam DTOs ([`project::seam`], [`artifacts::evidence`]).
//! A shim implements the axis `Guest` traits for its own type and wires
//! them in with `testkit::wit::export!(Adapter with_types_in
//! testkit::wit)`, keeping the guest itself a thin invoke-and-map shim
//! over the [`crate::adapter`] fixture core.

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

use artifacts::evidence;
pub use generated::*;
use project::platform;
use project::seam::{self, wire};

use self::generated::exports::specify::adapter::{source, target};

impl From<seam::Error> for source::Error {
    fn from(error: seam::Error) -> Self {
        match error {
            seam::Error::InvalidRequest(detail) => Self::InvalidRequest(detail),
            seam::Error::Io(detail) => Self::Io(detail),
            seam::Error::Internal(detail) => Self::Internal(detail),
        }
    }
}

impl From<source::Lead> for seam::Lead {
    fn from(lead: source::Lead) -> Self {
        Self {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        }
    }
}

impl From<seam::Lead> for source::Lead {
    fn from(lead: seam::Lead) -> Self {
        Self {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        }
    }
}

impl From<seam::Evidence> for source::Evidence {
    fn from(evidence: seam::Evidence) -> Self {
        Self {
            authority: evidence.authority.into(),
            claims: evidence.claims.into_iter().map(source::Claim::from).collect(),
        }
    }
}

impl From<evidence::AuthorityClass> for source::Authority {
    fn from(authority: evidence::AuthorityClass) -> Self {
        match authority {
            evidence::AuthorityClass::Intent => Self::Intent,
            evidence::AuthorityClass::Documentation => Self::Documentation,
            evidence::AuthorityClass::Behaviour => Self::Behaviour,
        }
    }
}

impl From<evidence::Claim> for source::Claim {
    fn from(claim: evidence::Claim) -> Self {
        let backing = claim.backing().map(source::Backing::from);
        Self {
            kind: claim.kind.into(),
            id: claim.id,
            path: claim.path,
            synopsis: claim.synopsis,
            backing,
        }
    }
}

impl From<evidence::ClaimKind> for source::ClaimKind {
    fn from(kind: evidence::ClaimKind) -> Self {
        match kind {
            evidence::ClaimKind::Intent => Self::Intent,
            evidence::ClaimKind::Requirement => Self::Requirement,
            evidence::ClaimKind::Criterion => Self::Criterion,
            evidence::ClaimKind::Decision => Self::Decision,
            evidence::ClaimKind::Section => Self::Section,
            evidence::ClaimKind::Diagram => Self::Diagram,
            evidence::ClaimKind::Contract => Self::Contract,
            evidence::ClaimKind::Example => Self::Example,
            evidence::ClaimKind::Excerpt => Self::Excerpt,
            evidence::ClaimKind::Type => Self::Type,
            evidence::ClaimKind::Call => Self::Call,
            evidence::ClaimKind::Region => Self::Region,
            evidence::ClaimKind::Container => Self::Container,
            evidence::ClaimKind::Leaf => Self::Leaf,
        }
    }
}

impl From<evidence::Backing> for source::Backing {
    fn from(backing: evidence::Backing) -> Self {
        match backing {
            evidence::Backing::Payload(payload) => Self::Payload(payload),
            evidence::Backing::Path(path) => Self::Path(path),
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

impl From<target::Input> for seam::Input {
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

impl From<target::MergePhase> for seam::MergePhase {
    fn from(phase: target::MergePhase) -> Self {
        match phase {
            target::MergePhase::Preflight => Self::Preflight,
            target::MergePhase::Postflight => Self::Postflight,
        }
    }
}

// Narrow the fixture's stamped `BuildReport` to the WIT report: the
// envelope keys (`version`, `slice`, `target`) stay caller-owned on the
// seam, and the fixture never emits findings or a UI surface.
impl From<wire::BuildReport> for target::Report {
    fn from(report: wire::BuildReport) -> Self {
        Self {
            status: report.status.into(),
            findings: Vec::new(),
            outputs: report.outputs.into_iter().map(target::BuildOutput::from).collect(),
            ui_surface: None,
        }
    }
}

impl From<wire::BuildStatus> for target::Status {
    fn from(status: wire::BuildStatus) -> Self {
        match status {
            wire::BuildStatus::Success => Self::Success,
            wire::BuildStatus::Failure => Self::Failure,
        }
    }
}

impl From<wire::BuildOutput> for target::BuildOutput {
    fn from(output: wire::BuildOutput) -> Self {
        Self {
            platform: output.platform.into(),
            path: output.path,
        }
    }
}
