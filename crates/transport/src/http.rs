//! The guest HTTP surface: a read-only MCP spec shelf plus the typed
//! refusal for every other route.
//!
//! The unauthenticated listener serves MCP shelves only — the adapter
//! reference shelves and this spec shelf. Reads were never what C3
//! fences; everything else refuses typed.

use emery_engine::home::{Home, SpecSet};
use omnia_guest::axum::Router;
use omnia_guest::axum::extract::State;
use omnia_guest::axum::response::{IntoResponse, Response};
use omnia_guest::axum::routing::any;
use omnia_guest::http::header::{CONTENT_TYPE, HeaderValue};
use omnia_guest::http::{Method, StatusCode};
use omnia_guest::mcp::{
    self, CallToolResult, Implementation, McpError, McpServer, Resource, ResourceContents, Tool,
};
use omnia_guest::{BlobStore, StateStore};
use serde_json::{Value, json};

/// Listener path serving the spec shelf.
pub const SPEC_ROUTE: &str = "/mcp/emery/spec";

/// Resource URI of the current generation's `spec.md`.
pub const SPEC_URI: &str = "spec://spec.md";

/// Resource URI of the current generation's `design.md`.
pub const DESIGN_URI: &str = "spec://design.md";

/// Resource URI of the current generation id.
pub const GENERATION_URI: &str = "spec://generation";

const NOT_GENERATED: &str = "no specification generation has been committed; run `emery specify \
                             <adapter>...` to commit one";

/// Builds the guest HTTP router: the spec shelf, refusal elsewhere.
pub fn listener<P>(provider: P) -> Router
where
    P: StateStore + BlobStore + Clone + Send + Sync + 'static,
{
    Router::new().route(SPEC_ROUTE, any(shelf::<P>)).fallback(refuse).with_state(provider)
}

/// Builds the catch-all HTTP refusal router.
pub fn refusal() -> Router {
    Router::new().fallback(refuse)
}

// Mirrors `omnia_guest::mcp::router` transport semantics over a
// per-request snapshot: the sync `McpServer` cannot await storage, so
// the current generation loads before the message is handled.
async fn shelf<P>(State(provider): State<P>, method: Method, body: String) -> Response
where
    P: StateStore + BlobStore + Clone + Send + Sync + 'static,
{
    if method != Method::POST {
        return (StatusCode::METHOD_NOT_ALLOWED, "MCP endpoint accepts POST only").into_response();
    }
    let shelf = match Home::new(&provider).current_set().await {
        Ok(None) => SpecShelf::Empty,
        Ok(Some((committed, set))) => SpecShelf::Current {
            id: committed.id,
            set,
        },
        Err(error) => SpecShelf::Failed(error.to_string()),
    };
    mcp::handle_message(&shelf, &body).map_or_else(
        || StatusCode::ACCEPTED.into_response(),
        |text| {
            ([(CONTENT_TYPE, HeaderValue::from_static("application/json"))], text).into_response()
        },
    )
}

async fn refuse() -> Response {
    (
        StatusCode::NOT_FOUND,
        [(CONTENT_TYPE, HeaderValue::from_static("application/json"))],
        serde_json::json!({
            "error": "http-surface-disabled",
            "message": "the emery HTTP listener serves only MCP shelves; \
                        run workflow commands through the emery CLI",
        })
        .to_string(),
    )
        .into_response()
}

// One request's snapshot of the current generation.
#[derive(Debug)]
enum SpecShelf {
    Empty,
    Current { id: String, set: SpecSet },
    Failed(String),
}

impl SpecShelf {
    // Only known URIs reach here; both callers gate on the URI first.
    fn read(&self, uri: &str) -> Result<String, McpError> {
        match self {
            Self::Empty => {
                let mut error = McpError::resource_not_found(uri);
                error.message = format!("{}: {NOT_GENERATED}", error.message);
                Err(error)
            }
            Self::Failed(detail) => Err(McpError::internal(detail.clone())),
            Self::Current { id, set } => match uri {
                SPEC_URI => Ok(set.spec.clone()),
                DESIGN_URI => Ok(set.design.clone()),
                GENERATION_URI => Ok(id.clone()),
                other => Err(McpError::resource_not_found(other)),
            },
        }
    }
}

impl McpServer for SpecShelf {
    fn info(&self) -> Implementation {
        Implementation::new("emery-spec", env!("CARGO_PKG_VERSION"))
    }

    fn tools(&self) -> Vec<Tool> {
        let empty = json!({ "type": "object", "properties": {} });
        vec![
            Tool::new("read_spec", "Read the current generation's spec.md in full.", empty.clone()),
            Tool::new(
                "read_design",
                "Read the current generation's design.md in full.",
                empty.clone(),
            ),
            Tool::new("generation", "Return the current generation id.", empty),
        ]
    }

    fn call_tool(&self, name: &str, _arguments: &Value) -> Result<CallToolResult, McpError> {
        let uri = match name {
            "read_spec" => SPEC_URI,
            "read_design" => DESIGN_URI,
            "generation" => GENERATION_URI,
            other => return Err(McpError::unknown_tool(other)),
        };
        match self {
            // The tool ran and found no generation; surface that to the model.
            Self::Empty => Ok(CallToolResult::error(NOT_GENERATED)),
            _ => self.read(uri).map(CallToolResult::text),
        }
    }

    fn resources(&self) -> Vec<Resource> {
        let Self::Current { id, .. } = self else {
            return Vec::new();
        };
        vec![
            Resource::new(
                SPEC_URI,
                "spec.md",
                format!("Committed `spec.md` of generation `{id}`."),
                "text/markdown",
            ),
            Resource::new(
                DESIGN_URI,
                "design.md",
                format!("Committed `design.md` of generation `{id}`."),
                "text/markdown",
            ),
            Resource::new(GENERATION_URI, "generation", "Current generation id.", "text/plain"),
        ]
    }

    fn read_resource(&self, uri: &str) -> Result<ResourceContents, McpError> {
        let mime = match uri {
            SPEC_URI | DESIGN_URI => "text/markdown",
            GENERATION_URI => "text/plain",
            other => return Err(McpError::resource_not_found(other)),
        };
        Ok(ResourceContents::text(uri, mime, self.read(uri)?))
    }
}
