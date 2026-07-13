//! Deterministic model backend for the greeting workflow.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use omnia::{Backend, FromEnv};
use omnia_wasi_model::{Answer, Format, FutureResult, Request, ToolHost, WasiModelCtx};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Scripted;

#[derive(Clone, Copy, Debug)]
pub(crate) struct NoOptions;

impl FromEnv for NoOptions {
    fn from_env() -> Result<Self> {
        Ok(Self)
    }
}

impl Backend for Scripted {
    type ConnectOptions = NoOptions;

    async fn connect_with(_options: Self::ConnectOptions) -> Result<Self> {
        Ok(Self)
    }
}

impl WasiModelCtx for Scripted {
    fn complete(&self, request: Request, _tool_host: Arc<dyn ToolHost>) -> FutureResult<Answer> {
        let answer = match request.format {
            Format::Schema(schema) if schema.name == "proposal" => grouping_answer(),
            Format::Schema(schema) if schema.name == "synthesis" => synthesis_answer(),
            format => {
                return Box::pin(async move {
                    Err(anyhow!("the greeting example has no answer for format {format:?}"))
                });
            }
        };
        Box::pin(async move {
            Ok(Answer {
                value: answer,
                usage: None,
                transcript: None,
            })
        })
    }
}

fn grouping_answer() -> Value {
    json!({
        "version": 1,
        "kind": "response",
        "slices": [{
            "name": "greeting",
            "sources": [{ "source": "main", "lead": "greeting" }],
            "rationale": "One fixture lead, one slice."
        }],
        "gate": {
            "change": "## Intent\n\nCharacterise the greeting service.\n\n## Scope\n\nOne slice.",
            "discovery-summary": "Sources: 1. Leads: 1.",
            "discovery-source-inventory": "| key | adapter | binding |\n|---|---|---|\n| main | fixture | \"hello\" |"
        }
    })
}

fn synthesis_answer() -> Value {
    json!({
        "version": 1,
        "kind": "response",
        "slice": "greeting",
        "model": {
            "requirements": [{
                "title": "greeting returns the static string",
                "domain": "greeting",
                "claims": [{ "source": "main", "id": "greeting.behaviour", "kind": "requirement" }],
                "statement": "GET /greeting returns the static string 'hello'.",
                "scenarios": ["A request to /greeting receives 'hello'"]
            }],
            "tasks": [
                { "id": "TASK-001", "text": "Implement the greeting endpoint.", "satisfies": ["REQ-001"] }
            ]
        },
        "artifacts": {
            "proposal": "# greeting\n\n## Why\n\nThe fixture source surfaced it.\n\n## Domains\n\n- greeting — the affected surface\n\n## Non-goals\n\n- Nothing else.\n",
            "design": "# Design\n\nHow the greeting slice lands.\n",
            "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Implement the endpoint (TASK-001)\n",
            "specs": [{ "domain": "greeting", "content": "## greeting\nAgent prose body.\n" }]
        }
    })
}
