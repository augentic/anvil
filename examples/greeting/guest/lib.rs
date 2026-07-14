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

mod source;
mod targets;
