//! The guest's non-MCP HTTP surface: one typed refusal (C3).
//!
//! The deployment's pre-bound listener carries no authentication, so
//! mutating ingress over HTTP is disabled until an operator ingress
//! is designed (target-architecture §7). The engine guest serves only
//! the MCP reference shelf (`slice::shelf::PATH`, routed back to the
//! guest by the deployment's `http_paths` hook); every other path and
//! method answers the typed 404 below. Reintroducing an operation
//! route table here is an ingress design decision, not a wiring
//! change — `crates/transport/tests/router.rs` holds the refusal.

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
