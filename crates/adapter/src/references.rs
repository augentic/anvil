//! MCP references server every adapter guest serves over `wasi:http`.
//!
//! One [`References`] implementation for all adapters; a shim differs
//! only in server name and doc table.

use omnia_guest::mcp::{
    CallToolResult, Implementation, McpError, McpServer, Resource, ResourceContents, Tool,
};
use serde_json::{Value, json};

use crate::registry::{self, Doc};

/// Adapter MCP URL from `SPECIFY_<ADAPTER>_MCP_URL`.
///
/// Accepts a bare name or axis-qualified id (`target:contracts`); the
/// axis prefix is stripped. Absent means judgment legs run without a
/// reference grant.
#[must_use]
pub fn mcp_url(adapter: &str) -> Option<String> {
    let name = adapter.rsplit(':').next().unwrap_or(adapter);
    let key = format!("SPECIFY_{}_MCP_URL", name.to_uppercase().replace('-', "_"));
    std::env::var(key).ok()
}

/// `<name>-references`, interned once per process so it stays `&'static`.
///
/// # Panics
///
/// If the intern table's lock is poisoned.
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

/// Embedded prose registry served over MCP.
#[derive(Clone, Copy, Debug)]
pub struct References {
    /// e.g. `contracts-references`.
    pub server_name: &'static str,
    /// Declaring crate's `CARGO_PKG_VERSION`.
    pub version: &'static str,
    /// Sorted embedded doc table.
    pub docs: &'static [Doc],
}

#[cfg(target_arch = "wasm32")]
impl References {
    /// # Errors
    ///
    /// As `omnia_wasi_http::serve` over the router.
    pub async fn serve(
        self, request: wasip3::http::types::Request,
    ) -> Result<wasip3::http::types::Response, wasip3::http::types::ErrorCode> {
        omnia_wasi_http::serve(omnia_guest::mcp::router(self), request).await
    }
}

/// Serve a `wasi:http` request for an adapter's references identity.
///
/// # Errors
///
/// As [`References::serve`].
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
