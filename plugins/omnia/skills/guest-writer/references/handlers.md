# Handlers

HTTP routing, message subscriptions, WebSocket events, and lib.rs wiring for WASM guests.

For the canonical export patterns (struct definitions, trait implementations, handler invocation patterns, and error handling), see [guest-patterns.md](omnia/guest-patterns.md).

---

## Complete lib.rs Example

Full working example of a WASM guest module.

```rust
#![cfg(target_arch = "wasm32")]
//! WASM Guest for User Service
//!
//! Wraps the user-domain crate and exposes:
//! - HTTP endpoints for user CRUD operations
//! - Message topic subscriptions for user events
//! - WebSocket event handling for real-time updates

use anyhow::Result;
use axum::Router;
use axum::extract::Path;
use axum::routing::{get, post, put};
use bytes::Bytes;
use omnia_sdk::{Config, Handler, HttpRequest, HttpResult, Identity, Publish, Reply, StateStore, ensure_env};
use omnia_wasi_messaging::types::{Error, Message};
use omnia_wasi_websocket::types::{Error as WsError, Event};
use tracing::Level;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types as p3;

// Import domain crate handlers and types
use user_domain::{
    CreateUserRequest, CreateUserResponse,
    GetUserRequest, GetUserResponse,
    UpdateUserRequest, UpdateUserResponse,
    UserCreatedEvent, UserUpdatedEvent,
    UserNotification,
};

// ============================================================================
// HTTP Handler
// ============================================================================

pub struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[omnia_wasi_otel::instrument(name = "http_guest_handle", level = Level::INFO)]
    async fn handle(request: p3::Request) -> Result<p3::Response, p3::ErrorCode> {
        let router = Router::new()
            .route("/api/users", post(create_user))
            .route("/api/users/{user_id}", get(get_user))
            .route("/api/users/{user_id}", put(update_user));

        omnia_wasi_http::serve(router, request).await
    }
}

async fn create_user(body: Bytes) -> HttpResult<Reply<CreateUserResponse>> {
    CreateUserRequest::handler(body.to_vec())?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}

async fn get_user(Path(user_id): Path<String>) -> HttpResult<Reply<GetUserResponse>> {
    GetUserRequest::handler(user_id)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}

async fn update_user(
    Path(user_id): Path<String>,
    body: Bytes,
) -> HttpResult<Reply<UpdateUserResponse>> {
    UpdateUserRequest::handler((user_id, body.to_vec()))?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map_err(Into::into)
}

// ============================================================================
// Messaging Handler
// ============================================================================

pub struct Messaging;
omnia_wasi_messaging::export!(Messaging with_types_in omnia_wasi_messaging);

impl omnia_wasi_messaging::incoming_handler::Guest for Messaging {
    #[omnia_wasi_otel::instrument(name = "messaging_guest_handle")]
    async fn handle(message: Message) -> Result<(), Error> {
        if let Err(e) = match &message.topic().unwrap_or_default() {
            t if t.contains("user-created.v1") => {
                handle_user_created(message.data()).await
            }
            t if t.contains("user-updated.v1") => {
                handle_user_updated(message.data()).await
            }
            _ => {
                return Err(Error::Other("Unhandled topic".to_string()));
            }
        } {
            return Err(Error::Other(e.to_string()));
        }
        Ok(())
    }
}

#[omnia_wasi_otel::instrument]
async fn handle_user_created(payload: Vec<u8>) -> Result<()> {
    UserCreatedEvent::handler(payload)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map(|_| ())
        .map_err(Into::into)
}

#[omnia_wasi_otel::instrument]
async fn handle_user_updated(payload: Vec<u8>) -> Result<()> {
    UserUpdatedEvent::handler(payload)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map(|_| ())
        .map_err(Into::into)
}

// ============================================================================
// WebSocket Handler
// ============================================================================

struct WebSocketGuest;
omnia_wasi_websocket::export!(WebSocketGuest);

impl omnia_wasi_websocket::incoming_handler::Guest for WebSocketGuest {
    #[omnia_wasi_otel::instrument(name = "websocket_guest_handle")]
    async fn handle(event: Event) -> Result<(), WsError> {
        handle_user_notification(event.data()).await
            .map_err(|e| WsError::Other(e.to_string()))
    }
}

#[omnia_wasi_otel::instrument]
async fn handle_user_notification(payload: Vec<u8>) -> Result<()> {
    UserNotification::handler(payload)?
        .provider(&Provider::new())
        .owner("at")
        .await
        .map(|_| ())
        .map_err(Into::into)
}

// ============================================================================
// Provider Configuration
// ============================================================================

#[derive(Clone, Default)]
pub struct Provider;

impl Provider {
    #[must_use]
    pub fn new() -> Self {
        ensure_env!(
            "API_URL",
            "SERVICE_NAME",
            "AZURE_IDENTITY",
        );
        Self
    }
}

impl Config for Provider {}
impl HttpRequest for Provider {}
impl Identity for Provider {}
impl Publish for Provider {}
impl StateStore for Provider {}
```

### Key Points

1. **wasm32 guard** -- `#![cfg(target_arch = "wasm32")]` at top of file
2. **HTTP export** -- `wasip3::http::proxy::export!(Http);`
3. **Messaging export** -- `omnia_wasi_messaging::export!(Messaging with_types_in omnia_wasi_messaging);`
4. **WebSocket export** -- `omnia_wasi_websocket::export!(WebSocketGuest);`
5. **Handler builder API** -- `Type::handler(input)?.provider(&provider).owner("owner").await`
6. **Owner** -- hardcoded string identifying the Omnia component owner (e.g. `"at"`)
7. **Reply wrapper** -- HTTP handlers return `HttpResult<Reply<T>>`, not `HttpResult<T>`
8. **Provider validation** -- `ensure_env!` validates required config at startup (optional)
9. **Instrumentation** -- `#[omnia_wasi_otel::instrument]` for tracing
10. **Unhandled topics** -- return `Err(Error::Other(...))`, not `Ok(())`
11. **Route params** -- use `{param}` syntax (Axum 0.8), not `:param`
12. **WebSocket error alias** -- when both messaging and WebSocket errors are in scope, alias WebSocket's as `WsError`

For detailed handler patterns, route methods, error handling, and individual handler signatures, see [guest-patterns.md](omnia/guest-patterns.md).
