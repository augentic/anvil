//! Canonical operations-trait implementors over the fixture core.
//!
//! Each unit type binds one catalog identity onto the shared
//! [`crate::adapter`] core: behaviour still keys off the routed
//! `ctx.adapter_id`, so one core serves every fixture name. The impls
//! narrow the core's engine-seam values (`project::seam`,
//! `artifacts::evidence`) to the SDK's [`adapter::seam`] records —
//! the same mapping the WASM guest applies at the WIT boundary.

use adapter::registry::Doc;
use adapter::seam::{self as aseam, Context, SourceMetadata, TargetMetadata};
use adapter::{Source, Target};
use artifacts::evidence;
use omnia_guest::Model;
use project::seam;
use project::seam::wire;

use crate::adapter as core;

/// The fixture's single embedded reference document.
pub const DOCS: &[Doc] = &[Doc {
    path: "reference.md",
    body: "# Adapter Reference\n\nThe harness adapter serves both axes from one component: \
           deterministic survey/extract data on the source interface and guidance/build/merge \
           on the target interface.\n",
}];

/// The static guidance brief the trait surface serves — the id-keyed
/// [`core::guidance`] stays available for the id-routed provider path.
const GUIDANCE: &str = "Fixture guidance (target:fixture): keep specs behavioural, one domain \
                        per spec; builds write one markdown artifact per slice under \
                        `fixture-build/`.";

macro_rules! fixture_source {
    ($ty:ident, $name:literal) => {
        impl Source for $ty {
            const NAME: &'static str = $name;

            fn metadata() -> SourceMetadata {
                SourceMetadata { specify_floor: None }
            }

            fn docs() -> &'static [Doc] {
                DOCS
            }

            async fn survey<P: Model>(
                _model: &P, ctx: &Context<'_>,
            ) -> Result<Vec<aseam::Lead>, aseam::Error> {
                let leads = core::survey(ctx.adapter_id).map_err(error_out)?;
                Ok(leads.into_iter().map(lead_out).collect())
            }

            async fn extract<P: Model>(
                _model: &P, ctx: &Context<'_>, lead: &aseam::Lead,
            ) -> Result<aseam::Evidence, aseam::Error> {
                let lead = lead_in(lead);
                core::extract(ctx.adapter_id, &lead).map(evidence_out).map_err(error_out)
            }
        }
    };
}

macro_rules! fixture_target {
    ($ty:ident, $name:literal) => {
        impl Target for $ty {
            const NAME: &'static str = $name;

            fn metadata() -> TargetMetadata {
                TargetMetadata {
                    specify_floor: None,
                    inputs: Vec::new(),
                    platforms: None,
                }
            }

            fn docs() -> &'static [Doc] {
                DOCS
            }

            fn guidance() -> &'static str {
                GUIDANCE
            }

            async fn build<P: Model>(
                _model: &P, ctx: &Context<'_>, slice: &str, inputs: &[aseam::Input],
                tree: &aseam::WorkingTree,
            ) -> Result<aseam::Report, aseam::Error> {
                let root = ctx.tree_root(tree);
                let inputs: Vec<seam::Input> = inputs.iter().map(input_in).collect();
                core::build(&root, ctx.adapter_id, slice, &inputs)
                    .map(report_out)
                    .map_err(error_out)
            }

            async fn merge<P: Model>(
                _model: &P, ctx: &Context<'_>, slice: &str, phase: aseam::MergePhase,
                tree: &aseam::WorkingTree,
            ) -> Result<aseam::Report, aseam::Error> {
                let root = ctx.tree_root(tree);
                core::merge(&root, ctx.adapter_id, slice, phase_in(phase))
                    .map(report_out)
                    .map_err(error_out)
            }
        }
    };
}

/// The default fixture identity — both axes, like the WASM guest.
#[derive(Clone, Copy, Debug)]
pub struct Fixture;

fixture_source!(Fixture, "fixture");
fixture_target!(Fixture, "fixture");

/// The documentation half of the adversarial source pair.
#[derive(Clone, Copy, Debug)]
pub struct FixtureDocs;

fixture_source!(FixtureDocs, "fixture-docs");

/// The behaviour (code) half of the adversarial source pair.
#[derive(Clone, Copy, Debug)]
pub struct FixtureCode;

fixture_source!(FixtureCode, "fixture-code");

fn error_out(error: seam::Error) -> aseam::Error {
    match error {
        seam::Error::InvalidRequest(detail) => aseam::Error::InvalidRequest(detail),
        seam::Error::Io(detail) => aseam::Error::Io(detail),
        seam::Error::Internal(detail) => aseam::Error::Internal(detail),
    }
}

fn lead_out(lead: seam::Lead) -> aseam::Lead {
    aseam::Lead {
        lead: lead.lead,
        synopsis: lead.synopsis,
        topics: lead.topics,
    }
}

fn lead_in(lead: &aseam::Lead) -> seam::Lead {
    seam::Lead {
        lead: lead.lead.clone(),
        synopsis: lead.synopsis.clone(),
        topics: lead.topics.clone(),
    }
}

fn evidence_out(evidence: seam::Evidence) -> aseam::Evidence {
    aseam::Evidence {
        authority: authority_out(evidence.authority),
        claims: evidence.claims.into_iter().map(claim_out).collect(),
    }
}

const fn authority_out(authority: evidence::AuthorityClass) -> aseam::Authority {
    match authority {
        evidence::AuthorityClass::Intent => aseam::Authority::Intent,
        evidence::AuthorityClass::Documentation => aseam::Authority::Documentation,
        evidence::AuthorityClass::Behaviour => aseam::Authority::Behaviour,
    }
}

// Open per-kind claim fields do not cross the compact seam record.
fn claim_out(claim: evidence::Claim) -> aseam::Claim {
    let backing = claim.backing().map(|backing| match backing {
        evidence::Backing::Payload(payload) => aseam::Backing::Payload(payload),
        evidence::Backing::Path(path) => aseam::Backing::Path(path),
    });
    aseam::Claim {
        kind: kind_out(claim.kind),
        id: claim.id,
        path: claim.path,
        synopsis: claim.synopsis,
        backing,
    }
}

const fn kind_out(kind: evidence::ClaimKind) -> aseam::ClaimKind {
    match kind {
        evidence::ClaimKind::Intent => aseam::ClaimKind::Intent,
        evidence::ClaimKind::Requirement => aseam::ClaimKind::Requirement,
        evidence::ClaimKind::Criterion => aseam::ClaimKind::Criterion,
        evidence::ClaimKind::Decision => aseam::ClaimKind::Decision,
        evidence::ClaimKind::Section => aseam::ClaimKind::Section,
        evidence::ClaimKind::Diagram => aseam::ClaimKind::Diagram,
        evidence::ClaimKind::Contract => aseam::ClaimKind::Contract,
        evidence::ClaimKind::Example => aseam::ClaimKind::Example,
        evidence::ClaimKind::Excerpt => aseam::ClaimKind::Excerpt,
        evidence::ClaimKind::Type => aseam::ClaimKind::Type,
        evidence::ClaimKind::Call => aseam::ClaimKind::Call,
        evidence::ClaimKind::Region => aseam::ClaimKind::Region,
        evidence::ClaimKind::Container => aseam::ClaimKind::Container,
        evidence::ClaimKind::Leaf => aseam::ClaimKind::Leaf,
    }
}

fn input_in(input: &aseam::Input) -> seam::Input {
    match input {
        aseam::Input::Proposal(body) => seam::Input::Proposal(body.clone()),
        aseam::Input::Design(body) => seam::Input::Design(body.clone()),
        aseam::Input::Tasks(body) => seam::Input::Tasks(body.clone()),
        aseam::Input::Spec(body) => seam::Input::Spec(body.clone()),
        aseam::Input::Other(body) => seam::Input::Other(body.clone()),
    }
}

const fn phase_in(phase: aseam::MergePhase) -> seam::MergePhase {
    match phase {
        aseam::MergePhase::Preflight => seam::MergePhase::Preflight,
        aseam::MergePhase::Postflight => seam::MergePhase::Postflight,
    }
}

// Narrow the fixture's stamped `BuildReport` to the seam report: the
// envelope keys (`version`, `slice`, `target`) stay caller-owned, and
// the fixture never emits findings or a UI surface.
fn report_out(report: wire::BuildReport) -> aseam::Report {
    aseam::Report {
        status: match report.status {
            wire::BuildStatus::Success => aseam::Status::Success,
            wire::BuildStatus::Failure => aseam::Status::Failure,
        },
        findings: Vec::new(),
        outputs: report.outputs.into_iter().map(output_out).collect(),
        ui_surface: None,
    }
}

fn output_out(output: wire::BuildOutput) -> aseam::BuildOutput {
    aseam::BuildOutput {
        platform: platform_out(output.platform),
        path: output.path,
    }
}

const fn platform_out(platform: project::platform::Platform) -> aseam::Platform {
    match platform {
        project::platform::Platform::Core => aseam::Platform::Core,
        project::platform::Platform::Ios => aseam::Platform::Ios,
        project::platform::Platform::Android => aseam::Platform::Android,
        project::platform::Platform::Web => aseam::Platform::Web,
        project::platform::Platform::Desktop => aseam::Platform::Desktop,
    }
}
