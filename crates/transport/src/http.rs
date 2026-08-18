//! The guest's non-MCP HTTP surface: one typed 404 (C3).
//!
//! The unauthenticated listener serves only the MCP reference shelf;
//! operation routes are an ingress design decision (target-architecture §7).

use omnia_guest::axum::Router;
use omnia_guest::axum::response::{IntoResponse, Response};
use omnia_guest::http::StatusCode;
use omnia_guest::http::header::{CONTENT_TYPE, HeaderValue};

/// The whole non-shelf HTTP surface: every path and method refuses.
pub fn refusal() -> Router {
    Router::new().fallback(refuse)
}

/// The typed refusal, naming the one served surface.
async fn refuse() -> Response {
    (
        StatusCode::NOT_FOUND,
        [(CONTENT_TYPE, HeaderValue::from_static("application/json"))],
        serde_json::json!({
            "error": "http-surface-disabled",
            "message": "the emery HTTP listener serves only MCP reference shelves; \
                        run workflow commands through the emery CLI",
        })
        .to_string(),
    )
        .into_response()
}
