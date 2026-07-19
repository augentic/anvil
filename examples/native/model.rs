//! Cursor model backend for native linked execution.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use omnia::Backend as _;
use omnia_guest::model::{
    Effort, Error, Format, Message, Model, Reply, Request, Role, Tool, Usage,
};
use omnia_wasi_model::{DirEntry, FutureResult, Reference, ToolHost, VerifyReport, WasiModelCtx};

#[derive(Clone, Debug)]
pub struct DevModel {
    root: PathBuf,
    client: Arc<tokio::sync::OnceCell<Native<omnia_cursor::Client>>>,
}

impl DevModel {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            client: Arc::new(tokio::sync::OnceCell::new()),
        }
    }
}

impl Model for DevModel {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        let native = self
            .client
            .get_or_try_init(|| async {
                let client = omnia_cursor::Client::connect().await?;
                Ok::<_, anyhow::Error>(Native {
                    backend: client,
                    workspace: self.root.clone(),
                })
            })
            .await
            .map_err(|error| Error::Backend(error.to_string()))?;
        native.create(request).await
    }
}

#[derive(Clone, Debug)]
struct Native<B> {
    backend: B,
    workspace: PathBuf,
}

impl<B: WasiModelCtx> Model for Native<B> {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        let workspace = request.lend_workspace.then(|| self.workspace.clone());
        let wire = wire_request(request);
        omnia_wasi_model::validate_request(&wire).map_err(wire_error)?;
        let format = wire.format.clone();
        let answer = self
            .backend
            .complete(wire, Arc::new(LocalToolHost { workspace }))
            .await
            .map_err(|error| Error::Backend(error.to_string()))?;
        let reply = answer.project(&format).map_err(wire_error)?;

        Ok(Reply {
            answer: reply.answer,
            usage: reply.usage.map(|usage| Usage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                reasoning_tokens: usage.reasoning_tokens,
            }),
        })
    }
}

fn wire_request(request: Request) -> omnia_wasi_model::Request {
    omnia_wasi_model::Request {
        model: request.model,
        system: request.system,
        messages: request.messages.into_iter().map(wire_message).collect(),
        generation: request.generation.map(|generation| omnia_wasi_model::Generation {
            temperature: generation.temperature,
            top_p: generation.top_p,
            max_tokens: generation.max_tokens,
            stop: generation.stop,
            seed: generation.seed,
            effort: generation.effort.map(wire_effort),
        }),
        format: match request.format {
            Format::Text => omnia_wasi_model::Format::Text,
            Format::Json => omnia_wasi_model::Format::Json,
            Format::Schema(schema) => omnia_wasi_model::Format::Schema(omnia_wasi_model::Schema {
                name: schema.name,
                schema: schema.schema,
            }),
        },
        tools: request.tools.into_iter().map(wire_tool).collect(),
        grants: omnia_wasi_model::Grants {
            references: request.references,
            workspace: None,
            verify: request.verify,
        },
    }
}

fn wire_message(message: Message) -> omnia_wasi_model::Message {
    omnia_wasi_model::Message {
        role: match message.role {
            Role::System => omnia_wasi_model::Role::System,
            Role::User => omnia_wasi_model::Role::User,
            Role::Assistant => omnia_wasi_model::Role::Assistant,
        },
        content: message.content,
    }
}

fn wire_tool(tool: Tool) -> omnia_wasi_model::Tool {
    match tool {
        Tool::Function(function) => omnia_wasi_model::Tool::Function(omnia_wasi_model::Function {
            name: function.name,
            description: function.description,
            parameters: function.parameters,
        }),
        Tool::Mcp(mcp) => omnia_wasi_model::Tool::Mcp(omnia_wasi_model::Mcp {
            name: mcp.name,
            tools: mcp.tools,
            url: mcp.url,
        }),
    }
}

const fn wire_effort(effort: Effort) -> omnia_wasi_model::Effort {
    match effort {
        Effort::Minimal => omnia_wasi_model::Effort::Minimal,
        Effort::Low => omnia_wasi_model::Effort::Low,
        Effort::Medium => omnia_wasi_model::Effort::Medium,
        Effort::High => omnia_wasi_model::Effort::High,
    }
}

fn wire_error(error: omnia_wasi_model::Error) -> Error {
    match error {
        omnia_wasi_model::Error::InvalidRequest(detail) => Error::InvalidRequest(detail),
        omnia_wasi_model::Error::InvalidAnswer(detail) => Error::InvalidAnswer(detail),
        omnia_wasi_model::Error::BudgetExhausted(detail) => Error::BudgetExhausted(detail),
        omnia_wasi_model::Error::ToolFailed(detail) => Error::ToolFailed(detail),
        omnia_wasi_model::Error::Backend(detail) => Error::Backend(detail),
    }
}

struct LocalToolHost {
    workspace: Option<PathBuf>,
}

impl ToolHost for LocalToolHost {
    fn resolve(&self, _reference: Reference) -> FutureResult<Vec<u8>> {
        refuse("references")
    }

    fn read(&self, _path: String) -> FutureResult<Vec<u8>> {
        refuse("reads")
    }

    fn list(&self, _path: String) -> FutureResult<Vec<DirEntry>> {
        refuse("listings")
    }

    fn write(&self, _path: String, _bytes: Vec<u8>) -> FutureResult<()> {
        refuse("writes")
    }

    fn verify(&self, _check: String) -> FutureResult<VerifyReport> {
        refuse("verification")
    }

    fn local_path(&self) -> Option<&Path> {
        self.workspace.as_deref()
    }
}

fn refuse<T>(capability: &str) -> FutureResult<T> {
    let error = anyhow::anyhow!("the native tool host serves no {capability}");
    Box::pin(async move { Err(error) })
}
