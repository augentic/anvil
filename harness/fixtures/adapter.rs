//! The combined fixture-adapter guest component.
//!
//! Exports the `specify:adapter` `adapter` world — both the `source`
//! and `target` interfaces from one component — plus
//! `wasi:http/incoming-handler`, serving a compiled-in single-document
//! MCP references. The shim is nothing but generated WIT conversions
//! delegating to the native fixture core (this package's library), so
//! hosted WASM and native tests exercise identical adapter behaviour.
#![cfg(target_arch = "wasm32")]

mod bindings {
    //! `wit_bindgen::generate!` output for the combined `adapter`
    //! world. The `export!` shim is invoked here too: lint levels
    //! resolve at the macro invocation's syntactic context, so the
    //! generated `unsafe(export_name)` plumbing must expand inside
    //! this allow scope.
    #![allow(
        missing_docs,
        unsafe_code,
        clippy::pedantic,
        clippy::nursery,
        reason = "wit-bindgen generated bindings are not hand-maintained; the generated code cannot carry this workspace's lint posture"
    )]

    use super::FixtureAdapter;

    wit_bindgen::generate!({
        world: "adapter",
        path: "../../wit",
        // Asyncness follows the WIT declarations: the judgment
        // operations are `async func`s and async-lift; `metadata` is a
        // plain `func` (deterministic, effect-free) and sync-lifts —
        // forcing it async would fail component validation at load.
        generate_all,
    });

    export!(FixtureAdapter);
}

use std::path::Path;

use bindings::exports::specify::adapter::{source, target};
use omnia_guest::mcp::{
    self, CallToolResult, Implementation, McpError, McpServer, Resource, ResourceContents, Tool,
};
use serde_json::{Value, json};
use wasip3::http::types as http;

struct FixtureAdapter;

impl source::Guest for FixtureAdapter {
    fn metadata(_id: source::AdapterId) -> source::AdapterMetadata {
        source::AdapterMetadata { specify_floor: None }
    }

    async fn survey(id: source::AdapterId) -> Result<Vec<source::Lead>, source::Error> {
        let leads = fixtures::survey(&id).map_err(map_error)?;
        Ok(leads.into_iter().map(wire_lead).collect())
    }

    async fn extract(
        id: source::AdapterId, lead: source::Lead,
    ) -> Result<source::Evidence, source::Error> {
        let core_lead = fixtures::Lead {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        };
        let evidence = fixtures::extract(&id, &core_lead).map_err(map_error)?;
        Ok(source::Evidence {
            authority: wire_authority(evidence.authority),
            claims: evidence.claims.into_iter().map(wire_claim).collect(),
        })
    }
}

impl target::Guest for FixtureAdapter {
    fn metadata(id: target::AdapterId) -> target::AdapterMetadata {
        target::AdapterMetadata {
            specify_floor: None,
            inputs: Vec::new(),
            platforms: fixtures::target_platforms(&id).map(wire_platforms),
        }
    }

    async fn guidance(id: target::AdapterId) -> Result<String, target::Error> {
        fixtures::guidance(&id).map_err(map_target_error)
    }

    async fn build(
        id: target::AdapterId, slice: String, inputs: Vec<target::Input>,
        _tree: target::WorkingTree,
    ) -> Result<target::Report, target::Error> {
        // Every guest shares the deployment's `[[mount]]` preopens, so
        // the build writes through its own `"."` preopen.
        let inputs: Vec<fixtures::Input> = inputs.into_iter().map(core_input).collect();
        let report =
            fixtures::build(Path::new("."), &id, &slice, &inputs).map_err(map_target_error)?;
        Ok(wire_report(report))
    }

    async fn merge(
        id: target::AdapterId, slice: String, phase: target::MergePhase, _tree: target::WorkingTree,
    ) -> Result<target::Report, target::Error> {
        let core_phase = match phase {
            target::MergePhase::Preflight => fixtures::MergePhase::Preflight,
            target::MergePhase::Postflight => fixtures::MergePhase::Postflight,
        };
        let report =
            fixtures::merge(Path::new("."), &id, &slice, core_phase).map_err(map_target_error)?;
        Ok(wire_report(report))
    }
}

fn map_error(error: fixtures::Error) -> source::Error {
    match error {
        fixtures::Error::InvalidRequest(detail) => source::Error::InvalidRequest(detail),
        fixtures::Error::Io(detail) => source::Error::Io(detail),
        fixtures::Error::Internal(detail) => source::Error::Internal(detail),
    }
}

fn wire_platforms(capability: fixtures::PlatformsCapability) -> target::PlatformsCapability {
    target::PlatformsCapability {
        required: capability.required,
        allowed: capability.allowed.into_iter().map(wire_platform).collect(),
        default: capability.default.into_iter().map(wire_platform).collect(),
    }
}

const fn wire_platform(platform: fixtures::Platform) -> target::Platform {
    match platform {
        fixtures::Platform::Core => target::Platform::Core,
        fixtures::Platform::Ios => target::Platform::Ios,
        fixtures::Platform::Android => target::Platform::Android,
    }
}

fn map_target_error(error: fixtures::Error) -> target::Error {
    match error {
        fixtures::Error::InvalidRequest(detail) => target::Error::InvalidRequest(detail),
        fixtures::Error::Io(detail) => target::Error::Io(detail),
        fixtures::Error::Internal(detail) => target::Error::Internal(detail),
    }
}

fn wire_lead(lead: fixtures::Lead) -> source::Lead {
    source::Lead {
        lead: lead.lead,
        synopsis: lead.synopsis,
        topics: lead.topics,
    }
}

const fn wire_authority(authority: fixtures::Authority) -> source::Authority {
    match authority {
        fixtures::Authority::Intent => source::Authority::Intent,
        fixtures::Authority::Documentation => source::Authority::Documentation,
        fixtures::Authority::Behaviour => source::Authority::Behaviour,
    }
}

fn wire_claim(claim: fixtures::Claim) -> source::Claim {
    source::Claim {
        kind: wire_claim_kind(claim.kind),
        id: claim.id,
        path: claim.path,
        synopsis: claim.synopsis,
        backing: claim.backing.map(|backing| match backing {
            fixtures::Backing::Payload(payload) => source::Backing::Payload(payload),
            fixtures::Backing::Path(path) => source::Backing::Path(path),
        }),
    }
}

const fn wire_claim_kind(kind: fixtures::ClaimKind) -> source::ClaimKind {
    match kind {
        fixtures::ClaimKind::Requirement => source::ClaimKind::Requirement,
        fixtures::ClaimKind::Criterion => source::ClaimKind::Criterion,
        fixtures::ClaimKind::Section => source::ClaimKind::Section,
    }
}

fn core_input(input: target::Input) -> fixtures::Input {
    match input {
        target::Input::Proposal(body) => fixtures::Input::Proposal(body),
        target::Input::Design(body) => fixtures::Input::Design(body),
        target::Input::Tasks(body) => fixtures::Input::Tasks(body),
        target::Input::Spec(body) => fixtures::Input::Spec(body),
        target::Input::Other(body) => fixtures::Input::Other(body),
    }
}

fn wire_report(report: fixtures::Report) -> target::Report {
    target::Report {
        status: match report.status {
            fixtures::Status::Success => target::Status::Success,
            fixtures::Status::Failure => target::Status::Failure,
        },
        findings: Vec::new(),
        outputs: report
            .outputs
            .into_iter()
            .map(|output| target::BuildOutput {
                platform: target::Platform::Core,
                path: output.path,
            })
            .collect(),
        ui_surface: None,
    }
}

struct HttpGuest;

wasip3::http::service::export!(HttpGuest);

impl wasip3::exports::http::handler::Guest for HttpGuest {
    async fn handle(request: http::Request) -> Result<http::Response, http::ErrorCode> {
        omnia_wasi_http::serve(mcp::router(References), request).await
    }
}

const REFERENCE_NAME: &str = "fixture-reference";
const REFERENCE_BODY: &str = "# Fixture Reference\n\n\
     The fixture adapter serves both axes from one component: \
     deterministic survey/extract data on the source interface and \
     guidance/build/merge on the target interface.\n";

/// The compiled-in single-document references served over MCP.
struct References;

impl McpServer for References {
    fn info(&self) -> Implementation {
        Implementation::new("specify-fixture-references", env!("CARGO_PKG_VERSION"))
    }

    fn tools(&self) -> Vec<Tool> {
        vec![Tool::new(
            "read_reference",
            "Read the fixture adapter's single reference document in full.",
            json!({ "type": "object", "properties": {} }),
        )]
    }

    fn call_tool(&self, name: &str, _arguments: &Value) -> Result<CallToolResult, McpError> {
        match name {
            "read_reference" => Ok(CallToolResult::text(REFERENCE_BODY)),
            other => Err(McpError::unknown_tool(other)),
        }
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::new(
            format!("doc://{REFERENCE_NAME}"),
            REFERENCE_NAME,
            "The fixture adapter's single reference document.",
            "text/markdown",
        )]
    }

    fn read_resource(&self, uri: &str) -> Result<ResourceContents, McpError> {
        if uri.strip_prefix("doc://").unwrap_or(uri) == REFERENCE_NAME {
            Ok(ResourceContents::text(uri, "text/markdown", REFERENCE_BODY))
        } else {
            Err(McpError::resource_not_found(uri))
        }
    }
}
