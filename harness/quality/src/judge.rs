//! The live semantic judge on the omnia model seam.
//!
//! One `omnia_cursor::Client` completion per rubric: the prompt rides
//! a `Format::Schema` verdict contract so the backend's own
//! parse-and-repair loop enforces the JSON shape, and the trial
//! workspace is lent through a minimal tool host whose `local_path` is
//! all the cursor backend reads.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow};
use omnia::Backend as _;
use omnia_wasi_model::{self as wire, WasiModelCtx as _};
use scenario::evaluate::semantic::Judge;

/// The verdict shape the judge must return; mirrors what
/// `scenario::evaluate::semantic` validates.
const VERDICT_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "score": { "type": "integer", "minimum": 0, "maximum": 100 },
    "outcome": { "type": "string", "enum": ["pass", "fail"] },
    "detail": { "type": "string" }
  },
  "required": ["score", "outcome", "detail"],
  "additionalProperties": false
}"#;

/// Cursor-backed live judge.
#[derive(Debug, Clone)]
pub struct LiveJudge {
    client: omnia_cursor::Client,
    model: Option<String>,
}

impl LiveJudge {
    /// Connect the cursor backend (asserting cursor-agent is on
    /// `PATH`). The optional `SPECIFY_JUDGE_MODEL` override selects the
    /// judge's model independently of the subject model.
    ///
    /// # Errors
    ///
    /// Returns an error when cursor-agent is not runnable.
    pub async fn connect() -> Result<Self> {
        let client = omnia_cursor::Client::connect().await.context("connecting cursor-agent")?;
        Ok(Self {
            client,
            model: std::env::var("SPECIFY_JUDGE_MODEL").ok().filter(|id| !id.trim().is_empty()),
        })
    }

    /// The judge's model identity for `RunMetadata.judge_model`.
    #[must_use]
    pub fn model_identity(&self) -> String {
        self.model.clone().unwrap_or_else(|| "cursor-default".to_owned())
    }
}

impl Judge for LiveJudge {
    async fn judge(&self, prompt: String, workspace: &Path) -> std::result::Result<String, String> {
        let request = wire::Request {
            model: self.model.clone(),
            system: None,
            messages: vec![wire::Message {
                role: wire::Role::User,
                content: prompt,
            }],
            generation: None,
            format: wire::Format::Schema(wire::Schema {
                name: "verdict".to_owned(),
                schema: VERDICT_SCHEMA.to_owned(),
            }),
            tools: vec![],
            grants: wire::Grants {
                references: None,
                workspace: None,
                verify: vec![],
            },
        };
        let host = Arc::new(LocalToolHost {
            workspace: Some(workspace.to_owned()),
        });
        let answer =
            self.client.complete(request, host).await.map_err(|error| format!("{error:#}"))?;
        serde_json::to_string(&answer.value).map_err(|error| error.to_string())
    }
}

/// The minimal per-completion tool host the cursor backend reads: only
/// `local_path` matters (cursor-agent does its own filesystem work);
/// the bounded-capability methods are never called on this backend.
struct LocalToolHost {
    workspace: Option<PathBuf>,
}

impl wire::ToolHost for LocalToolHost {
    fn resolve(&self, _reference: wire::Reference) -> wire::FutureResult<Vec<u8>> {
        Box::pin(async { Err(anyhow!("the judge tool host serves no references")) })
    }

    fn read(&self, _path: String) -> wire::FutureResult<Vec<u8>> {
        Box::pin(async { Err(anyhow!("the judge tool host serves no reads")) })
    }

    fn list(&self, _path: String) -> wire::FutureResult<Vec<wire::DirEntry>> {
        Box::pin(async { Err(anyhow!("the judge tool host serves no listings")) })
    }

    fn write(&self, _path: String, _bytes: Vec<u8>) -> wire::FutureResult<()> {
        Box::pin(async { Err(anyhow!("the judge tool host serves no writes")) })
    }

    fn verify(&self, _check: String) -> wire::FutureResult<wire::VerifyReport> {
        Box::pin(async { Err(anyhow!("the judge tool host serves no verification")) })
    }

    fn local_path(&self) -> Option<&Path> {
        self.workspace.as_deref()
    }
}
