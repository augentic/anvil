//! Deterministic model backend for the change workflow.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use omnia::{Backend, FromEnv};
use omnia_wasi_model::{Answer, Format, FutureResult, Request, ToolHost, WasiModelCtx};
use serde_json::Value;

#[derive(Clone, Copy, Debug)]
pub struct Scripted;

#[derive(Clone, Copy, Debug)]
pub struct NoOptions;

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
                    Err(anyhow!("the change example has no answer for format {format:?}"))
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

// The judgment answers are the shared `testkit::answers` greeting
// corpus — the same documents the native suites script — so the smoke
// and the suites cannot drift apart on envelope shape.
fn grouping_answer() -> Value {
    serde_json::from_str(&testkit::answers::greeting_grouping()).expect("grouping answer parses")
}

fn synthesis_answer() -> Value {
    serde_json::from_str(&testkit::answers::greeting_synthesis()).expect("synthesis answer parses")
}
