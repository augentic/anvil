//! PATH-and-credential-gated live test for the Claude Code backend.
//!
//! Mirrors the omnia-cursor `tests/live.rs`: it spawns a real `claude` against
//! a node-local workspace and reads the validated answer back through the
//! `omnia:model/completion` boundary. This is the acceptance gate that the
//! event parser handles a real stream end to end — the unit tests in
//! `src/claude.rs` cover fixtures only.
//!
//! `#[ignore]`d so it never spawns a process in CI. Run it by hand alongside
//! an installed, authenticated CLI:
//!
//! ```text
//! cargo nextest run -p emery-model --run-ignored all
//! ```

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use model::claude::{Client, ConnectOptions};
use omnia::Backend as _;
use omnia_wasi_model::{
    Answer, DirEntry, Format, FutureResult, Grants, Message, Reference, Request, Role, Schema,
    ToolHost, WasiModelCtx as _,
};
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live: needs an installed, authenticated claude CLI; run with --run-ignored"]
async fn live_claude_completes() -> Result<()> {
    let workspace = std::env::temp_dir().join(format!("emery-claude-live-{}", std::process::id()));
    std::fs::create_dir_all(&workspace)?;

    let client = Client::connect_with(ConnectOptions {
        model: None,
        timeout_secs: 120,
        inactivity_secs: 120,
        bare: false,
    })
    .await?;

    let answer: Answer = client
        .complete(verdict_request(), Arc::new(LocalToolHost { workspace }))
        .await
        .map_err(|error| {
            anyhow::anyhow!(
                "live claude completion failed (is the CLI installed and authed?): {error}"
            )
        })?;

    assert!(answer.value.is_object(), "the answer must be a JSON object: {:?}", answer.value);
    assert!(
        answer.value.get("verdict").and_then(serde_json::Value::as_str).is_some(),
        "the answer must carry a string verdict: {:?}",
        answer.value
    );
    let usage = answer.usage.expect("claude reports token usage on the result event");
    assert!(usage.output_tokens > 0, "a real completion produces output tokens");

    Ok(())
}

fn verdict_request() -> Request {
    Request {
        model: None,
        system: Some(
            "You are a terse judge. Decide whether the candidate passes and reply with the \
             required JSON object."
                .to_owned(),
        ),
        messages: vec![Message {
            role: Role::User,
            content: "Judge the trivial candidate and return a verdict of \"pass\" with a \
                      one-line reason.\n\nThe candidate is a no-op; it should pass."
                .to_owned(),
        }],
        generation: None,
        format: Format::Schema(Schema {
            name: "verdict".to_owned(),
            schema: json!({
                "type": "object",
                "properties": {
                    "verdict": { "type": "string", "enum": ["pass", "fail"] },
                    "reason": { "type": "string" },
                },
                "required": ["verdict", "reason"],
                "additionalProperties": false,
            })
            .to_string(),
        }),
        tools: vec![],
        grants: Grants {
            references: None,
            workspace: None,
        },
    }
}

/// The spawned backends read only `local_path`; the bounded-capability methods
/// have no capability table to serve off `wasm32`.
#[derive(Debug)]
struct LocalToolHost {
    workspace: std::path::PathBuf,
}

impl ToolHost for LocalToolHost {
    fn resolve(&self, _reference: Reference) -> FutureResult<Vec<u8>> {
        refuse()
    }

    fn read(&self, _path: String) -> FutureResult<Vec<u8>> {
        refuse()
    }

    fn list(&self, _path: String) -> FutureResult<Vec<DirEntry>> {
        refuse()
    }

    fn write(&self, _path: String, _bytes: Vec<u8>) -> FutureResult<()> {
        refuse()
    }

    fn local_path(&self) -> Option<&Path> {
        Some(&self.workspace)
    }
}

fn refuse<T>() -> FutureResult<T> {
    Box::pin(async { Err(anyhow::anyhow!("the spawned backend ignores the tool host")) })
}
