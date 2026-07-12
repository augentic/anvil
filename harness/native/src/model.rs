//! The native [`Model`] backends behind [`crate::provider::Provider`].
//!
//! The live cursor backend remains local while the deterministic replay
//! backend is supplied by `omnia-testkit`:
//!
//! - [`CursorModel`] — a thin shim over `omnia_cursor::Client` (the
//!   host-side `WasiModelCtx` backend): map the guest [`Request`] onto
//!   the `omnia:model/completion` wire shape — the same mapping the
//!   wasm default body performs — and translate `lend_workspace: true`
//!   into a `ToolHost` whose `local_path` is the project root, which
//!   is the only thing cursor-agent reads from it. Live-only: dev loop
//!   and on-demand tasks, never CI.
//! - [`omnia_testkit::model::Replay`] — recorded fixtures served through
//!   `omnia_wasi_model::ModelDefault`'s canonical request-key replay.
//!
//! [`DevModel`] is the closed selection the `specify-dev` binary
//! constructs from the environment; tests bypass it and bind
//! `omnia_testkit::model::Harness` around a scripted backend.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use omnia::Backend as _;
use omnia_guest::Model;
use omnia_guest::model::{Effort, Error, Format, Reply, Request, Role, Tool, Usage};
use omnia_testkit::model::Replay;
use omnia_wasi_model as wire;
use omnia_wasi_model::WasiModelCtx as _;
use serde_json::Value;

/// The closed backend selection for the dev binary: `cursor` (default)
/// or `replay`, from `SPECIFY_DEV_MODEL`.
pub enum DevModel {
    /// Live cursor-agent completions, connected on first use so
    /// deterministic verbs never require cursor-agent on `PATH`.
    Cursor {
        /// The project root workspace lends resolve to.
        root: PathBuf,
        /// The connection, established by the first judgment leg.
        cell: tokio::sync::OnceCell<CursorModel>,
    },
    /// Recorded fixtures from `MODEL_REPLAY_DIR`.
    Replay(Replay),
}

impl fmt::Debug for DevModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Cursor { .. } => "DevModel::Cursor",
            Self::Replay(_) => "DevModel::Replay",
        })
    }
}

impl DevModel {
    /// Select the backend from the environment:
    /// `SPECIFY_DEV_MODEL=replay` loads fixtures from
    /// `MODEL_REPLAY_DIR` (default `fixtures`, matching
    /// `ModelDefault`); anything else is cursor-agent, connected
    /// lazily on the first judgment leg.
    ///
    /// # Errors
    ///
    /// Returns an error when a replay fixture file cannot be read or
    /// parsed.
    pub fn from_env(project_dir: &Path) -> Result<Self> {
        match std::env::var("SPECIFY_DEV_MODEL").as_deref() {
            Ok("replay") => {
                let dir = std::env::var_os("MODEL_REPLAY_DIR")
                    .map_or_else(|| PathBuf::from("fixtures"), PathBuf::from);
                Ok(Self::Replay(Replay::from_dir(&dir)?))
            }
            _ => Ok(Self::Cursor {
                root: project_dir.to_path_buf(),
                cell: tokio::sync::OnceCell::new(),
            }),
        }
    }
}

impl Model for DevModel {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        match self {
            Self::Cursor { root, cell } => {
                let model = cell
                    .get_or_try_init(|| CursorModel::connect(root.clone()))
                    .await
                    .map_err(|err| {
                        Error::Backend(format!(
                            "cursor-agent backend unavailable: {err:#}; install cursor-agent, \
                             then `cursor-agent login` or export CURSOR_API_KEY (`cargo make dev \
                             -- doctor --live` verifies command-mode credentials)"
                        ))
                    })?;
                model.create(request).await
            }
            Self::Replay(model) => model.create(request).await,
        }
    }
}

/// The cursor-agent-backed native [`Model`].
///
/// `omnia_cursor::Client`'s spawn/repair/transcript machinery is
/// reused as a library; only the request mapping and the
/// `lend_workspace` → project-root translation live here.
#[derive(Clone, Debug)]
pub struct CursorModel {
    client: omnia_cursor::Client,
    root: PathBuf,
}

impl CursorModel {
    /// Connect cursor-agent (asserting it is on `PATH`) rooted at the
    /// project directory the workspace lend resolves to.
    ///
    /// # Errors
    ///
    /// Returns an error when cursor-agent is not runnable.
    pub async fn connect(root: impl Into<PathBuf>) -> Result<Self> {
        let client = omnia_cursor::Client::connect().await?;
        Ok(Self {
            client,
            root: root.into(),
        })
    }
}

impl Model for CursorModel {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        // The lend translation: in-guest the `"."` preopen resolves the
        // lent workspace; natively the project root plays that part.
        // Cursor reads exactly one thing from the tool host — the
        // lent tree's local path.
        let workspace = request.lend_workspace.then(|| self.root.clone());
        let format = wire_format(&request.format);
        let wire = wire_request(request);

        let answer = self
            .client
            .complete(wire, Arc::new(LocalToolHost { workspace }))
            .await
            .map_err(|err| Error::Backend(err.to_string()))?;

        // The same answer gate the host runs after its backends.
        format.check(&answer.value).map_err(Error::InvalidAnswer)?;
        reply(answer)
    }
}

/// Guest [`Request`] → the `omnia:model/completion` wire request — the
/// mapping the wasm default body performs at the WIT boundary. The
/// lent workspace never crosses (`grants.workspace` is host plumbing;
/// cursor resolves the tree through the tool host instead).
fn wire_request(request: Request) -> wire::Request {
    wire::Request {
        model: request.model,
        system: request.system,
        messages: request
            .messages
            .into_iter()
            .map(|message| wire::Message {
                role: match message.role {
                    Role::System => wire::Role::System,
                    Role::User => wire::Role::User,
                    Role::Assistant => wire::Role::Assistant,
                },
                content: message.content,
            })
            .collect(),
        generation: request.generation.map(|generation| wire::Generation {
            temperature: generation.temperature,
            top_p: generation.top_p,
            max_tokens: generation.max_tokens,
            stop: generation.stop,
            seed: generation.seed,
            effort: generation.effort.map(|effort| match effort {
                Effort::Minimal => wire::Effort::Minimal,
                Effort::Low => wire::Effort::Low,
                Effort::Medium => wire::Effort::Medium,
                Effort::High => wire::Effort::High,
            }),
        }),
        format: wire_format(&request.format),
        tools: request
            .tools
            .into_iter()
            .map(|tool| match tool {
                Tool::Function(function) => wire::Tool::Function(wire::Function {
                    name: function.name,
                    description: function.description,
                    parameters: function.parameters,
                }),
                Tool::Mcp(grant) => wire::Tool::Mcp(wire::Mcp {
                    name: grant.name,
                    tools: grant.tools,
                    url: grant.url,
                }),
            })
            .collect(),
        grants: wire::Grants {
            references: request.references,
            workspace: None,
            verify: request.verify,
        },
    }
}

/// Guest [`Format`] → the wire format.
fn wire_format(format: &Format) -> wire::Format {
    match format {
        Format::Text => wire::Format::Text,
        Format::Json => wire::Format::Json,
        Format::Schema(schema) => wire::Format::Schema(wire::Schema {
            name: schema.name.clone(),
            schema: schema.schema.clone(),
        }),
    }
}

/// A backend [`wire::Answer`] → the guest [`Reply`]: `text` answers are
/// plain text, JSON formats carry the serialized document — the host
/// gate's own projection.
fn reply(answer: wire::Answer) -> Result<Reply, Error> {
    let text = match answer.value {
        Value::String(text) => text,
        value => serde_json::to_string(&value)
            .map_err(|err| Error::InvalidAnswer(format!("answer is not serializable: {err}")))?,
    };
    Ok(Reply {
        answer: text,
        usage: answer.usage.map(|usage| Usage {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            reasoning_tokens: usage.reasoning_tokens,
        }),
    })
}

/// The minimal per-completion tool host the cursor backend reads: only
/// `local_path` matters (cursor-agent does its own filesystem work);
/// the bounded-capability methods are never called on this backend.
struct LocalToolHost {
    workspace: Option<PathBuf>,
}

impl wire::ToolHost for LocalToolHost {
    fn resolve(&self, _reference: wire::Reference) -> wire::FutureResult<Vec<u8>> {
        Box::pin(async { Err(anyhow!("the native tool host serves no references")) })
    }

    fn read(&self, _path: String) -> wire::FutureResult<Vec<u8>> {
        Box::pin(async { Err(anyhow!("the native tool host serves no reads")) })
    }

    fn list(&self, _path: String) -> wire::FutureResult<Vec<wire::DirEntry>> {
        Box::pin(async { Err(anyhow!("the native tool host serves no listings")) })
    }

    fn write(&self, _path: String, _bytes: Vec<u8>) -> wire::FutureResult<()> {
        Box::pin(async { Err(anyhow!("the native tool host serves no writes")) })
    }

    fn verify(&self, _check: String) -> wire::FutureResult<wire::VerifyReport> {
        Box::pin(async { Err(anyhow!("the native tool host serves no verification")) })
    }

    fn local_path(&self) -> Option<&Path> {
        self.workspace.as_deref()
    }
}
