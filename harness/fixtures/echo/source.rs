//! Echo source-adapter guest component (test fixture).
//!
//! Exports the `specify:adapter` `source-adapter` world — `survey` returns one
//! hardcoded lead echoing the `adapter-id` argument and `extract` returns one
//! trivial claim echoing the lead — plus `wasi:http/incoming-handler`, serving
//! a compiled-in single-document MCP references. Deliberately model-free:
//! the component exists to exercise the runtime seams, not Specify logic.
#![cfg(target_arch = "wasm32")]

mod bindings {
    //! `wit_bindgen::generate!` output for the `source-adapter` world. The
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

    use super::EchoAdapter;

    wit_bindgen::generate!({
        world: "source-adapter",
        path: "../wit",
        // Asyncness follows the WIT declarations: the judgment operations
        // are `async func`s (judgment legs await the async `omnia:model`
        // import mid-call) and async-lift; `metadata` is a plain `func`
        // (deterministic, effect-free) and sync-lifts — forcing it
        // async would fail component validation at load.
        generate_all,
    });

    export!(EchoAdapter);
}

use bindings::exports::specify::adapter::source::{
    AdapterId, AdapterMetadata, Authority, Claim, ClaimKind, Error, Evidence, Guest, Lead,
};
use omnia_guest::mcp::{
    self, CallToolResult, Implementation, McpError, McpServer, Resource, ResourceContents, Tool,
};
use serde_json::{Value, json};
use wasip3::http::types as http;

struct EchoAdapter;

impl Guest for EchoAdapter {
    fn metadata(_id: AdapterId) -> AdapterMetadata {
        AdapterMetadata { specify_floor: None }
    }

    async fn survey(id: AdapterId) -> Result<Vec<Lead>, Error> {
        Ok(vec![Lead {
            lead: "echo".to_string(),
            synopsis: format!("echo lead from {id}"),
            topics: Vec::new(),
        }])
    }

    async fn extract(_id: AdapterId, lead: Lead) -> Result<Evidence, Error> {
        Ok(Evidence {
            authority: Authority::Documentation,
            claims: vec![Claim {
                kind: ClaimKind::Excerpt,
                id: None,
                path: None,
                synopsis: Some(lead.synopsis),
                backing: None,
            }],
        })
    }
}

struct HttpGuest;

wasip3::http::service::export!(HttpGuest);

impl wasip3::exports::http::handler::Guest for HttpGuest {
    async fn handle(request: http::Request) -> Result<http::Response, http::ErrorCode> {
        omnia_wasi_http::serve(mcp::router(References), request).await
    }
}

const REFERENCE_NAME: &str = "echo-reference";
const REFERENCE_BODY: &str = "# Echo Reference\n\n\
     The echo source adapter surveys one hardcoded lead and extracts one \
     claim that repeats the lead's synopsis verbatim.\n";

/// The compiled-in single-document references served over MCP.
struct References;

impl McpServer for References {
    fn info(&self) -> Implementation {
        Implementation::new("specify-echo-references", env!("CARGO_PKG_VERSION"))
    }

    fn tools(&self) -> Vec<Tool> {
        vec![Tool::new(
            "read_reference",
            "Read the echo adapter's single reference document in full.",
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
            "The echo adapter's single reference document.",
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
