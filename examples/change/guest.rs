//! # Example Adapter
//!
//! This crate implements both Specify source and target adapters as a
//! thin shim over testkit's fixture core: `testkit::wit` owns the
//! WIT bindings and the seam mappings, so each operation below is one
//! delegation with `From` conversions at the edges. It also implements
//! a single MCP server for the model agent to use when requesting
//! adapter reference documents.

#![cfg(target_arch = "wasm32")]
#![allow(missing_docs, unsafe_code)]

use std::path::Path;

use omnia_guest::mcp::{
    self, CallToolResult, Implementation, McpError, McpServer, Resource, ResourceContents, Tool,
};
use project::seam;
use serde_json::{Value, json};
use testkit::adapter;
use testkit::wit::exports::specify::adapter::{source, target};
use wasip3::http::types as http;

// ----------------------------------------------
// Specify source + target adapters
// ----------------------------------------------
struct Adapter;
testkit::wit::export!(Adapter with_types_in testkit::wit);

impl source::Guest for Adapter {
    fn metadata(_id: source::AdapterId) -> source::AdapterMetadata {
        source::AdapterMetadata { specify_floor: None }
    }

    async fn survey(id: source::AdapterId) -> Result<Vec<source::Lead>, source::Error> {
        let leads = adapter::survey(&id).map_err(source::Error::from)?;
        Ok(leads.into_iter().map(source::Lead::from).collect())
    }

    async fn extract(
        id: source::AdapterId, lead: source::Lead,
    ) -> Result<source::Evidence, source::Error> {
        Ok(adapter::extract(&id, &lead.into()).map_err(source::Error::from)?.into())
    }
}

impl target::Guest for Adapter {
    fn metadata(id: target::AdapterId) -> target::AdapterMetadata {
        target::AdapterMetadata {
            specify_floor: None,
            inputs: Vec::new(),
            platforms: adapter::target_platforms(&id).map(target::PlatformsCapability::from),
        }
    }

    async fn guidance(id: target::AdapterId) -> Result<String, target::Error> {
        adapter::guidance(&id).map_err(target::Error::from)
    }

    async fn build(
        id: target::AdapterId, slice: String, inputs: Vec<target::Input>,
        _tree: target::WorkingTree,
    ) -> Result<target::Report, target::Error> {
        // Every guest shares the deployment's `[[mount]]` preopens, so
        // the build writes through its own `"."` preopen.
        let inputs: Vec<seam::Input> = inputs.into_iter().map(seam::Input::from).collect();
        let report =
            adapter::build(Path::new("."), &id, &slice, &inputs).map_err(target::Error::from)?;
        Ok(report.into())
    }

    async fn merge(
        id: target::AdapterId, slice: String, phase: target::MergePhase, _tree: target::WorkingTree,
    ) -> Result<target::Report, target::Error> {
        let report = adapter::merge(Path::new("."), &id, &slice, phase.into())
            .map_err(target::Error::from)?;
        Ok(report.into())
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
