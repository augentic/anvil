//! The engine's synthesis reference shelf (RFC-96 D9): the embedded
//! synthesis prose corpus served over MCP at [`PATH`], granted to the
//! synthesis judgment so the playbook loads on demand.

use omnia_guest::mcp::{
    CallToolResult, Implementation, McpError, McpServer, Resource, ResourceContents, Tool,
};
use serde_json::{Value, json};

use crate::judgment::prose;

/// The shelf's HTTP path on every deployment's reference listener.
///
/// The launcher's `http_paths` hook routes it back onto the engine
/// guest; the native provider nests it on its loopback reference
/// listener beside the adapter shelves.
pub const PATH: &str = "/mcp/engine/synthesis";

/// The MCP server name reported in the `initialize` handshake and
/// used as the grant name on the synthesis judgment.
pub const SERVER: &str = "synthesis-references";

/// The embedded synthesis prose corpus served over MCP.
#[derive(Clone, Copy, Debug, Default)]
pub struct Shelf;

fn resolve(path: &str) -> Option<&'static str> {
    prose::DOCS.iter().find(|doc| doc.path == path).map(|doc| doc.body)
}

impl McpServer for Shelf {
    fn info(&self) -> Implementation {
        Implementation::new(SERVER, env!("CARGO_PKG_VERSION"))
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool::new(
                "list_docs",
                "List every synthesis playbook document path the engine embeds.",
                json!({ "type": "object", "properties": {} }),
            ),
            Tool::new(
                "read_doc",
                "Read one embedded synthesis playbook document in full by its path.",
                json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Corpus-relative document path, e.g. `synthesis/substeps.md`."
                        }
                    },
                    "required": ["path"]
                }),
            ),
        ]
    }

    fn call_tool(&self, name: &str, arguments: &Value) -> Result<CallToolResult, McpError> {
        match name {
            "list_docs" => {
                let paths: Vec<&str> = prose::DOCS.iter().map(|doc| doc.path).collect();
                Ok(CallToolResult::text(json!(paths).to_string()))
            }
            "read_doc" => {
                let path = arguments.get("path").and_then(Value::as_str).unwrap_or_default();
                resolve(path).map_or_else(
                    || Err(McpError::resource_not_found(path)),
                    |body| Ok(CallToolResult::text(body)),
                )
            }
            other => Err(McpError::unknown_tool(other)),
        }
    }

    fn resources(&self) -> Vec<Resource> {
        prose::DOCS
            .iter()
            .map(|doc| {
                Resource::new(
                    format!("doc://{}", doc.path),
                    doc.path,
                    "Embedded synthesis playbook document.",
                    "text/markdown",
                )
            })
            .collect()
    }

    fn read_resource(&self, uri: &str) -> Result<ResourceContents, McpError> {
        let path = uri.strip_prefix("doc://").unwrap_or(uri);
        resolve(path).map_or_else(
            || Err(McpError::resource_not_found(uri)),
            |body| Ok(ResourceContents::text(uri, "text/markdown", body)),
        )
    }
}
