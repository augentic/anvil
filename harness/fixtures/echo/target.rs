//! Echo target-adapter guest component (test fixture).
//!
//! Exports the `specify:adapter` `target-adapter` world with trivial,
//! model-free operations plus `wasi:http/incoming-handler` serving a
//! compiled-in single-document MCP references. The component
//! exists so host-side tests (init platform gates, metadata-driven
//! resolve, guest-leg discovery) can exercise a *real* target
//! component: `metadata` keys its platforms capability off the routed
//! `adapter-id`, letting one binary stand in for both a
//! platform-agnostic target and a platforms-requiring one.
#![cfg(target_arch = "wasm32")]

mod bindings {
    //! `wit_bindgen::generate!` output for the `target-adapter` world. The
    //! `export!` shim is invoked here too: lint levels resolve at the macro
    //! invocation's syntactic context, so the generated `unsafe(export_name)`
    //! plumbing must expand inside this allow scope.
    #![allow(
        missing_docs,
        unsafe_code,
        clippy::pedantic,
        clippy::nursery,
        reason = "wit-bindgen generated bindings are not hand-maintained; the generated code cannot carry this workspace's lint posture"
    )]

    use super::EchoTarget;

    wit_bindgen::generate!({
        world: "target-adapter",
        path: "../wit",
        // Asyncness follows the WIT declarations: the judgment operations
        // are `async func`s and async-lift; `metadata` is a plain `func`
        // (deterministic, effect-free) and sync-lifts — forcing it
        // async would fail component validation at load.
        generate_all,
    });

    export!(EchoTarget);
}

use bindings::exports::specify::adapter::target::{
    AdapterId, Changeset, Error, Guest, Input, Metadata, Platform, PlatformsCapability, Report,
    Status, WorkingTree,
};
use omnia_guest::mcp::{
    self, CallToolResult, Implementation, McpError, McpServer, Resource, ResourceContents, Tool,
};
use serde_json::{Value, json};
use wasip3::http::types as http;

struct EchoTarget;

impl Guest for EchoTarget {
    fn metadata(id: AdapterId) -> Metadata {
        // Deterministic per identity — fixture-only branching so one
        // binary stands in for several capability shapes (real adapters
        // compile in one answer):
        //   `…limited…`   -> platforms required, allowed {core, ios}
        //   `…platforms…` -> platforms required, allowed {core, ios, android}
        //   anything else -> platform-agnostic
        let platforms = if id.contains("limited") {
            Some(PlatformsCapability {
                required: true,
                allowed: vec![Platform::Core, Platform::Ios],
                default: vec![Platform::Core, Platform::Ios],
            })
        } else if id.contains("platforms") {
            Some(PlatformsCapability {
                required: true,
                allowed: vec![Platform::Core, Platform::Ios, Platform::Android],
                default: vec![Platform::Core, Platform::Ios, Platform::Android],
            })
        } else {
            None
        };
        Metadata {
            specify_floor: None,
            inputs: Vec::new(),
            platforms,
        }
    }

    async fn guidance(id: AdapterId) -> Result<String, Error> {
        Ok(format!("echo guidance from {id}"))
    }

    async fn build(
        _id: AdapterId, _slice: String, _inputs: Vec<Input>, _tree: WorkingTree,
    ) -> Result<Report, Error> {
        Ok(empty_report())
    }

    async fn merge(
        _id: AdapterId, _slice: String, _delta: Changeset, _tree: WorkingTree,
    ) -> Result<Report, Error> {
        Ok(empty_report())
    }
}

fn empty_report() -> Report {
    Report {
        status: Status::Success,
        findings: Vec::new(),
        outputs: Vec::new(),
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

const REFERENCE_NAME: &str = "echo-target-reference";
const REFERENCE_BODY: &str = "# Echo Target Reference\n\n\
     The echo target adapter reports success from every operation and \
     describes an id-conditioned platforms capability.\n";

/// The compiled-in single-document references served over MCP.
struct References;

impl McpServer for References {
    fn info(&self) -> Implementation {
        Implementation::new("echo-target-references", env!("CARGO_PKG_VERSION"))
    }

    fn tools(&self) -> Vec<Tool> {
        vec![Tool::new(
            "read_reference",
            "Read the echo target adapter's single reference document in full.",
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
            "The echo target adapter's single reference document.",
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
