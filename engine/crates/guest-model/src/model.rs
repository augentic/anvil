//! The `Model` capability: judgment calls through `omnia:model/completion`.
//!
//! The request and reply types here are wasm-free mirrors of the
//! `omnia:model@0.1.0` records, minus the one thing that cannot cross into
//! a wasm-free core: the `grants.workspace` descriptor lend. A core asks
//! for the lend with the plain [`Request::lend_workspace`] flag, and the
//! `wasm32` default body resolves it against the guest's own `"."` preopen
//! at the call site — a host call from the adapter's own instance, exactly
//! where RFC-61 says the descriptor belongs.

use std::future::Future;

/// Chat turn author.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// System / instructions channel.
    System,
    /// End-user turn.
    User,
    /// Model turn.
    Assistant,
}

/// One chat turn passed to the provider API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// Turn author.
    pub role: Role,
    /// Turn body text.
    pub content: String,
}

/// JSON Schema constrained output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaFormat {
    /// Schema name passed to the provider (e.g. `report`).
    pub name: String,
    /// JSON Schema document the answer must conform to.
    pub schema: String,
}

/// Output shape constraint for the completion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Format {
    /// Answer is plain text.
    Text,
    /// Answer must parse as a JSON object.
    Json,
    /// Answer must validate against the given JSON Schema; the host
    /// enforces this at the `create` gate.
    Schema(SchemaFormat),
}

/// Remote MCP server offered to the model for this completion — the
/// adapter's own reference shelf route, carrying its endpoint URL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpGrant {
    /// Logical server name (e.g. in `.cursor/mcp.json`).
    pub name: String,
    /// Tool allowlist; empty exposes every tool the server advertises.
    pub tools: Vec<String>,
    /// MCP server endpoint URL.
    pub url: String,
}

/// One judgment completion request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// Opaque model id hint; passed through unchanged. Backend may override.
    pub model: Option<String>,
    /// System / instructions channel.
    pub system: Option<String>,
    /// Chat turns sent to the provider. Must not be empty.
    pub messages: Vec<Message>,
    /// Required output shape and validation rules.
    pub format: Format,
    /// MCP server grants offered to the model.
    pub mcp: Vec<McpGrant>,
    /// Lend the guest's `"."` preopen through `grants.workspace`, giving
    /// the backend (and any spawned agent) the shared project mount.
    pub lend_workspace: bool,
}

/// One validated completion result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    /// The validated answer, per [`Request::format`].
    pub answer: String,
}

/// Typed completion failure, mirroring the `omnia:model` error variant.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The request itself is malformed; retrying without changing it is pointless.
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// Backend produced output that never passed validation.
    #[error("invalid answer: {0}")]
    InvalidAnswer(String),
    /// Iteration, token, time, or verify budget exhausted.
    #[error("budget exhausted: {0}")]
    BudgetExhausted(String),
    /// Non-repairable tool error.
    #[error("tool failed: {0}")]
    ToolFailed(String),
    /// Transport, process, or provider failure.
    #[error("backend failure: {0}")]
    Backend(String),
}

/// Issue judgment completions against the `omnia:model` host.
///
/// The method carries a WASI-backed default body on `wasm32` (delegating
/// to the `omnia-wasi-model` bindings) and a bare signature off `wasm32`,
/// so native tests supply their own provider — the same shape as
/// omnia-guest's capability traits.
pub trait Model: Send + Sync {
    /// Single-shot completion — returns one validated reply.
    #[cfg(not(target_arch = "wasm32"))]
    fn create(&self, request: Request) -> impl Future<Output = Result<Reply, Error>> + Send;

    /// Single-shot completion — returns one validated reply.
    #[cfg(target_arch = "wasm32")]
    fn create(&self, request: Request) -> impl Future<Output = Result<Reply, Error>> + Send {
        wasi::create(request)
    }
}

#[cfg(target_arch = "wasm32")]
mod wasi {
    //! The WASI-backed leg: map the wasm-free request onto the
    //! `omnia:model/completion` records and resolve the workspace lend
    //! from the guest's own preopen table.

    use omnia_wasi_model::completion;
    use wasip3::filesystem::preopens;

    use super::{Error, Format, Reply, Request, Role};

    pub(super) async fn create(request: Request) -> Result<Reply, Error> {
        // The lent workspace borrows one of these descriptors, so the
        // table must outlive the `create` call.
        let directories = if request.lend_workspace { preopens::get_directories() } else { vec![] };
        let workspace = directories.iter().find_map(|(dir, name)| (name == ".").then_some(dir));
        if request.lend_workspace && workspace.is_none() {
            return Err(Error::InvalidRequest(
                "workspace lend requested but the `.` preopen is absent".to_string(),
            ));
        }

        let wire = completion::Request {
            model: request.model,
            system: request.system,
            messages: request
                .messages
                .into_iter()
                .map(|message| completion::Message {
                    role: match message.role {
                        Role::System => completion::Role::System,
                        Role::User => completion::Role::User,
                        Role::Assistant => completion::Role::Assistant,
                    },
                    content: message.content,
                })
                .collect(),
            generation: None,
            format: match request.format {
                Format::Text => completion::Format::Text,
                Format::Json => completion::Format::Json,
                Format::Schema(schema) => completion::Format::Schema(completion::Schema {
                    name: schema.name,
                    schema: schema.schema,
                }),
            },
            tools: request
                .mcp
                .into_iter()
                .map(|grant| {
                    completion::Tool::Mcp(completion::Mcp {
                        name: grant.name,
                        tools: grant.tools,
                        url: grant.url,
                    })
                })
                .collect(),
            grants: completion::Grants {
                references: None,
                workspace,
                verify: vec![],
            },
        };

        match completion::create(wire).await {
            Ok(reply) => Ok(Reply { answer: reply.answer }),
            Err(completion::Error::InvalidRequest(detail)) => Err(Error::InvalidRequest(detail)),
            Err(completion::Error::InvalidAnswer(detail)) => Err(Error::InvalidAnswer(detail)),
            Err(completion::Error::BudgetExhausted(detail)) => Err(Error::BudgetExhausted(detail)),
            Err(completion::Error::ToolFailed(detail)) => Err(Error::ToolFailed(detail)),
            Err(completion::Error::Backend(detail)) => Err(Error::Backend(detail)),
        }
    }
}
