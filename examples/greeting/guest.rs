//! The greeting example's combined source/target WASM component.
//!
//! Exports the `specify:adapter` `adapter` world — both the `source`
//! and `target` interfaces from one component — plus
//! `wasi:http/incoming-handler`, serving a compiled-in single-document
//! MCP reference. The shim delegates to the shared `testkit::adapter`
//! core, so hosted WASM and native tests exercise identical adapter
//! behaviour.
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
        reason = "wit-bindgen generated bindings are not hand-maintained;"
    )]

    use super::FixtureAdapter;

    wit_bindgen::generate!({
        world: "adapter",
        path: "../wit",
        // Asyncness follows the WIT declarations: the judgment
        // operations are `async func`s and async-lift; `metadata` is a
        // plain `func` (deterministic, effect-free) and sync-lifts —
        // forcing it async would fail component validation at load.
        generate_all,
    });

    export!(FixtureAdapter);
}

use omnia_guest::mcp::{
    self, CallToolResult, Implementation, McpError, McpServer, Resource, ResourceContents, Tool,
};
use serde_json::{Value, json};
use wasip3::http::types as http;

struct FixtureAdapter;

struct HttpGuest;

wasip3::http::service::export!(HttpGuest);

impl wasip3::exports::http::handler::Guest for HttpGuest {
    async fn handle(request: http::Request) -> Result<http::Response, http::ErrorCode> {
        omnia_wasi_http::serve(mcp::router(References), request).await
    }
}

const REFERENCE_NAME: &str = "adapter-reference";
const REFERENCE_BODY: &str = "# Adapter Reference\n\n\
     The harness adapter serves both axes from one component: \
     deterministic survey/extract data on the source interface and \
     guidance/build/merge on the target interface.\n";

/// The compiled-in single-document references served over MCP.
struct References;

impl McpServer for References {
    fn info(&self) -> Implementation {
        Implementation::new("specify-adapter-references", env!("CARGO_PKG_VERSION"))
    }

    fn tools(&self) -> Vec<Tool> {
        vec![Tool::new(
            "read_reference",
            "Read the harness adapter's single reference document in full.",
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
            "The harness adapter's single reference document.",
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

mod source {
    use testkit::adapter;

    use crate::FixtureAdapter;
    use crate::bindings::exports::specify::adapter::source;

    impl source::Guest for FixtureAdapter {
        fn metadata(_id: source::AdapterId) -> source::AdapterMetadata {
            source::AdapterMetadata { specify_floor: None }
        }

        async fn survey(id: source::AdapterId) -> Result<Vec<source::Lead>, source::Error> {
            let leads = adapter::survey(&id).map_err(map_error)?;
            Ok(leads.into_iter().map(wire_lead).collect())
        }

        async fn extract(
            id: source::AdapterId, lead: source::Lead,
        ) -> Result<source::Evidence, source::Error> {
            let core_lead = adapter::Lead {
                lead: lead.lead,
                synopsis: lead.synopsis,
                topics: lead.topics,
            };
            let evidence = adapter::extract(&id, &core_lead).map_err(map_error)?;
            Ok(source::Evidence {
                authority: wire_authority(evidence.authority),
                claims: evidence.claims.into_iter().map(wire_claim).collect(),
            })
        }
    }

    fn map_error(error: adapter::Error) -> source::Error {
        match error {
            adapter::Error::InvalidRequest(detail) => source::Error::InvalidRequest(detail),
            adapter::Error::Io(detail) => source::Error::Io(detail),
            adapter::Error::Internal(detail) => source::Error::Internal(detail),
        }
    }

    fn wire_lead(lead: adapter::Lead) -> source::Lead {
        source::Lead {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        }
    }

    const fn wire_authority(authority: adapter::Authority) -> source::Authority {
        match authority {
            adapter::Authority::Intent => source::Authority::Intent,
            adapter::Authority::Documentation => source::Authority::Documentation,
            adapter::Authority::Behaviour => source::Authority::Behaviour,
        }
    }

    fn wire_claim(claim: adapter::Claim) -> source::Claim {
        source::Claim {
            kind: wire_claim_kind(claim.kind),
            id: claim.id,
            path: claim.path,
            synopsis: claim.synopsis,
            backing: claim.backing.map(|backing| match backing {
                adapter::Backing::Payload(payload) => source::Backing::Payload(payload),
                adapter::Backing::Path(path) => source::Backing::Path(path),
            }),
        }
    }

    const fn wire_claim_kind(kind: adapter::ClaimKind) -> source::ClaimKind {
        match kind {
            adapter::ClaimKind::Requirement => source::ClaimKind::Requirement,
            adapter::ClaimKind::Criterion => source::ClaimKind::Criterion,
            adapter::ClaimKind::Section => source::ClaimKind::Section,
        }
    }
}

mod targets {
    use std::path::Path;

    use testkit::adapter;

    use crate::FixtureAdapter;
    use crate::bindings::exports::specify::adapter::target;

    impl target::Guest for FixtureAdapter {
        fn metadata(id: target::AdapterId) -> target::AdapterMetadata {
            target::AdapterMetadata {
                specify_floor: None,
                inputs: Vec::new(),
                platforms: adapter::target_platforms(&id).map(wire_platforms),
            }
        }

        async fn guidance(id: target::AdapterId) -> Result<String, target::Error> {
            adapter::guidance(&id).map_err(map_error)
        }

        async fn build(
            id: target::AdapterId, slice: String, inputs: Vec<target::Input>,
            _tree: target::WorkingTree,
        ) -> Result<target::Report, target::Error> {
            // Every guest shares the deployment's `[[mount]]` preopens, so
            // the build writes through its own `"."` preopen.
            let inputs: Vec<adapter::Input> = inputs.into_iter().map(core_input).collect();
            let report = adapter::build(Path::new("."), &id, &slice, &inputs).map_err(map_error)?;
            Ok(wire_report(report))
        }

        async fn merge(
            id: target::AdapterId, slice: String, phase: target::MergePhase,
            _tree: target::WorkingTree,
        ) -> Result<target::Report, target::Error> {
            let core_phase = match phase {
                target::MergePhase::Preflight => adapter::MergePhase::Preflight,
                target::MergePhase::Postflight => adapter::MergePhase::Postflight,
            };
            let report =
                adapter::merge(Path::new("."), &id, &slice, core_phase).map_err(map_error)?;
            Ok(wire_report(report))
        }
    }

    fn map_error(error: adapter::Error) -> target::Error {
        match error {
            adapter::Error::InvalidRequest(detail) => target::Error::InvalidRequest(detail),
            adapter::Error::Io(detail) => target::Error::Io(detail),
            adapter::Error::Internal(detail) => target::Error::Internal(detail),
        }
    }

    fn wire_platforms(capability: adapter::PlatformsCapability) -> target::PlatformsCapability {
        target::PlatformsCapability {
            required: capability.required,
            allowed: capability.allowed.into_iter().map(wire_platform).collect(),
            default: capability.default.into_iter().map(wire_platform).collect(),
        }
    }

    const fn wire_platform(platform: adapter::Platform) -> target::Platform {
        match platform {
            adapter::Platform::Core => target::Platform::Core,
            adapter::Platform::Ios => target::Platform::Ios,
            adapter::Platform::Android => target::Platform::Android,
        }
    }

    fn core_input(input: target::Input) -> adapter::Input {
        match input {
            target::Input::Proposal(body) => adapter::Input::Proposal(body),
            target::Input::Design(body) => adapter::Input::Design(body),
            target::Input::Tasks(body) => adapter::Input::Tasks(body),
            target::Input::Spec(body) => adapter::Input::Spec(body),
            target::Input::Other(body) => adapter::Input::Other(body),
        }
    }

    fn wire_report(report: adapter::Report) -> target::Report {
        target::Report {
            status: match report.status {
                adapter::Status::Success => target::Status::Success,
                adapter::Status::Failure => target::Status::Failure,
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
}
