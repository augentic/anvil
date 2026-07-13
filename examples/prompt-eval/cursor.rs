//! Native cursor-agent [`Model`] for the prompt-eval harness.
//!
//! Deliberately duplicates the `CursorModel` shim in
//! `specify-adapters/harness/native/src/model.rs` (minus `DevModel`
//! selection and the `SPECIFY_EVAL_MODEL` override): engine tests must
//! not depend on the sibling checkout, and nothing mechanical catches
//! drift — when the guest→wire mapping changes upstream, update both.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use omnia::Backend as _;
use omnia_guest::Model;
use omnia_guest::model::{Effort, Error, Format, Reply, Request, Role, Tool, Usage};
use omnia_wasi_model as wire;
use omnia_wasi_model::WasiModelCtx as _;
use serde_json::Value;

/// The cursor-agent-backed native [`Model`].
#[derive(Clone, Debug)]
pub struct CursorModel {
    client: omnia_cursor::Client,
    root: PathBuf,
}

impl CursorModel {
    /// Connect cursor-agent (asserting it is on `PATH`) rooted at the
    /// project directory the workspace lend resolves to.
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
        // In-guest the `"."` preopen resolves the lent workspace;
        // natively the project root plays that part.
        let workspace = request.lend_workspace.then(|| self.root.clone());
        let format = wire_format(&request.format);
        let wire = wire_request(request);

        let answer = self
            .client
            .complete(wire, Arc::new(LocalToolHost { workspace }))
            .await
            .map_err(|err| Error::Backend(err.to_string()))?;

        format.check(&answer.value).map_err(Error::InvalidAnswer)?;
        reply(answer)
    }
}

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

/// Minimal tool host: cursor-agent only needs `local_path`.
struct LocalToolHost {
    workspace: Option<PathBuf>,
}

impl wire::ToolHost for LocalToolHost {
    fn resolve(&self, _reference: wire::Reference) -> wire::FutureResult<Vec<u8>> {
        Box::pin(async { Err(anyhow::anyhow!("the native tool host serves no references")) })
    }

    fn read(&self, _path: String) -> wire::FutureResult<Vec<u8>> {
        Box::pin(async { Err(anyhow::anyhow!("the native tool host serves no reads")) })
    }

    fn list(&self, _path: String) -> wire::FutureResult<Vec<wire::DirEntry>> {
        Box::pin(async { Err(anyhow::anyhow!("the native tool host serves no listings")) })
    }

    fn write(&self, _path: String, _bytes: Vec<u8>) -> wire::FutureResult<()> {
        Box::pin(async { Err(anyhow::anyhow!("the native tool host serves no writes")) })
    }

    fn verify(&self, _check: String) -> wire::FutureResult<wire::VerifyReport> {
        Box::pin(async { Err(anyhow::anyhow!("the native tool host serves no verification")) })
    }

    fn local_path(&self) -> Option<&Path> {
        self.workspace.as_deref()
    }
}
