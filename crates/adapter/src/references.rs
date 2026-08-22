//! Shared MCP server for adapter reference documents.

use omnia_guest::mcp::{
    CallToolResult, Implementation, McpError, McpServer, Resource, ResourceContents, Tool,
};
use serde_json::{Value, json};

use crate::registry::{self, Doc};

/// Returns an interned `<name>-references`.
///
/// # Panics
///
/// Panics if the intern table lock is poisoned.
#[must_use]
pub fn server_name(name: &'static str) -> &'static str {
    static NAMES: std::sync::Mutex<std::collections::BTreeMap<&'static str, &'static str>> =
        std::sync::Mutex::new(std::collections::BTreeMap::new());
    NAMES
        .lock()
        .expect("server-name intern table is never poisoned")
        .entry(name)
        .or_insert_with(|| Box::leak(format!("{name}-references").into_boxed_str()))
}

/// MCP server backed by embedded prose.
#[derive(Clone, Copy, Debug)]
pub struct References {
    /// MCP server name.
    pub server_name: &'static str,
    /// Adapter version.
    pub version: &'static str,
    /// Embedded docs, sorted by path.
    pub docs: &'static [Doc],
}

#[cfg(target_arch = "wasm32")]
impl References {
    /// Serves one references request.
    ///
    /// # Errors
    ///
    /// Returns errors from the HTTP router.
    pub async fn serve(
        self, request: wasip3::http::types::Request,
    ) -> Result<wasip3::http::types::Response, wasip3::http::types::ErrorCode> {
        omnia_wasi_http::serve(omnia_guest::mcp::router(self), request).await
    }
}

/// Serves an adapter references request.
///
/// # Errors
///
/// Returns errors from [`References::serve`].
#[cfg(target_arch = "wasm32")]
pub async fn serve(
    name: &'static str, version: &'static str, docs: &'static [Doc],
    request: wasip3::http::types::Request,
) -> Result<wasip3::http::types::Response, wasip3::http::types::ErrorCode> {
    References {
        server_name: server_name(name),
        version,
        docs,
    }
    .serve(request)
    .await
}

impl McpServer for References {
    fn info(&self) -> Implementation {
        Implementation::new(self.server_name, self.version)
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool::new(
                "list_docs",
                "List every reference document path this adapter embeds.",
                json!({ "type": "object", "properties": {} }),
            ),
            Tool::new(
                "read_doc",
                "Read one embedded reference document in full by its path.",
                json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Adapter-relative document path, e.g. `prompts/build.md`."
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
                let paths: Vec<&str> = self.docs.iter().map(|doc| doc.path).collect();
                Ok(CallToolResult::text(json!(paths).to_string()))
            }
            "read_doc" => {
                let path = arguments.get("path").and_then(Value::as_str).unwrap_or_default();
                registry::resolve(self.docs, path).map_or_else(
                    || Err(McpError::resource_not_found(path)),
                    |body| Ok(CallToolResult::text(body)),
                )
            }
            other => Err(McpError::unknown_tool(other)),
        }
    }

    fn resources(&self) -> Vec<Resource> {
        self.docs
            .iter()
            .map(|doc| {
                Resource::new(
                    format!("doc://{}", doc.path),
                    doc.path,
                    "Embedded adapter reference document.",
                    "text/markdown",
                )
            })
            .collect()
    }

    fn read_resource(&self, uri: &str) -> Result<ResourceContents, McpError> {
        let path = uri.strip_prefix("doc://").unwrap_or(uri);
        registry::resolve(self.docs, path).map_or_else(
            || Err(McpError::resource_not_found(uri)),
            |body| Ok(ResourceContents::text(uri, "text/markdown", body)),
        )
    }
}
