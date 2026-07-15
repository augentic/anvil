//! # Example Adapter
//!
//! This crate implements both Specify source and target adapters. It
//! implements a single MCP server for the model agent to use to when
//! requesting adapter reference documents.

#![cfg(target_arch = "wasm32")]
#![allow(missing_docs, unsafe_code)]

mod bindings {
    wit_bindgen::generate!({
        world: "adapter",
        path: "../wit",
        generate_all,
        pub_export_macro: true,
    });
}

mod source;
mod target;

use omnia_guest::mcp::{
    self, CallToolResult, Implementation, McpError, McpServer, Resource, ResourceContents, Tool,
};
use project::seam;
use serde_json::{Value, json};
use wasip3::http::types as http;

use self::bindings::exports::specify::adapter::source::Error;

// ----------------------------------------------
// Specify source + target adapters
// ----------------------------------------------
struct Adapter;
self::bindings::export!(Adapter with_types_in self::bindings);

impl From<seam::Error> for Error {
    fn from(error: seam::Error) -> Self {
        match error {
            seam::Error::InvalidRequest(detail) => Self::InvalidRequest(detail),
            seam::Error::Io(detail) => Self::Io(detail),
            seam::Error::Internal(detail) => Self::Internal(detail),
        }
    }
}

// ----------------------------------------------
// MCP server for adapter references
// ----------------------------------------------
struct HttpGuest;
wasip3::http::service::export!(HttpGuest);

impl wasip3::exports::http::handler::Guest for HttpGuest {
    async fn handle(request: http::Request) -> Result<http::Response, http::ErrorCode> {
        omnia_wasi_http::serve(mcp::router(References), request).await
    }
}

const REF_NAME: &str = "adapter-reference";
const REF_DOC: &str = "# Adapter Reference\n\n\
     The harness adapter serves both axes from one component: \
     deterministic survey/extract data on the source interface and \
     guidance/build/merge on the target interface.\n";

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
            "read_reference" => Ok(CallToolResult::text(REF_DOC)),
            other => Err(McpError::unknown_tool(other)),
        }
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::new(
            format!("doc://{REF_NAME}"),
            REF_NAME,
            "The harness adapter's single reference document.",
            "text/markdown",
        )]
    }

    fn read_resource(&self, uri: &str) -> Result<ResourceContents, McpError> {
        if uri.strip_prefix("doc://").unwrap_or(uri) == REF_NAME {
            Ok(ResourceContents::text(uri, "text/markdown", REF_DOC))
        } else {
            Err(McpError::resource_not_found(uri))
        }
    }
}
