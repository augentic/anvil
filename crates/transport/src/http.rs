//! Typed refusal for the non-MCP HTTP surface.
//!
//! The unauthenticated listener serves only MCP references.

use omnia_guest::axum::Router;
use omnia_guest::axum::response::{IntoResponse, Response};
use omnia_guest::http::StatusCode;
use omnia_guest::http::header::{CONTENT_TYPE, HeaderValue};

/// Builds the catch-all HTTP refusal router.
pub fn refusal() -> Router {
    Router::new().fallback(refuse)
}

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
