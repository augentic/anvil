//! The model backend the shipped `emery` binary links into its `WasiModel`
//! host slot.
//!
//! [`ModelBackend`] is a two-variant dispatcher over the spawned agent CLIs:
//! `cursor-agent` (the default, via `omnia_cursor`) and Claude Code (via
//! [`claude`]). `EMERY_MODEL_BACKEND` picks one at connect time, and only the
//! selected variant is connected — each backend's preflight shells out to its
//! own binary, so connecting both would make either mode require both CLIs on
//! `PATH`.
//!
//! Every completion runs through [`retry`], which retries transport failures
//! (an unreachable provider, a killed spawn, a stalled stream) with bounded
//! exponential backoff and passes everything else through untouched. There is
//! no cross-backend fallback: a Claude failure never retries on cursor, or the
//! reverse.
//!
//! ```text
//! EMERY_MODEL_BACKEND       cursor (default) | claude
//! EMERY_MODEL_RETRIES       extra attempts on transport failure (default 2)
//! EMERY_MODEL_RETRY_BACKOFF_MS  first backoff, doubling (default 1000)
//! ```

pub mod claude;
mod env;
pub mod retry;

use std::sync::Arc;

use anyhow::{Result, bail};
use omnia::{Backend, FromEnv};
use omnia_wasi_model::{Answer, FutureResult, Grants, Request, ToolHost, WasiModelCtx};
use tracing::instrument;

/// The spawned-agent model backend the runtime links, selected by
/// `EMERY_MODEL_BACKEND`.
#[derive(Debug, Clone)]
pub enum ModelBackend {
    /// `cursor-agent`, the default.
    Cursor(omnia_cursor::Client, retry::Policy),
    /// Claude Code.
    Claude(claude::Client, retry::Policy),
}

impl ModelBackend {
    /// The connected backend's CLI name, for logs and operator-facing errors.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.selection().name()
    }

    /// Which backend this is.
    #[must_use]
    pub const fn selection(&self) -> Selection {
        match *self {
            Self::Cursor(..) => Selection::Cursor,
            Self::Claude(..) => Selection::Claude,
        }
    }
}

impl Backend for ModelBackend {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let policy = options.retry;
        Ok(match options.selection {
            Selection::Cursor => Self::Cursor(omnia_cursor::Client::connect().await?, policy),
            Selection::Claude => Self::Claude(claude::Client::connect().await?, policy),
        })
    }
}

impl WasiModelCtx for ModelBackend {
    fn complete(&self, request: Request, tool_host: Arc<dyn ToolHost>) -> FutureResult<Answer> {
        let this = self.clone();
        let name = self.name();
        let policy = match *self {
            Self::Cursor(_, policy) | Self::Claude(_, policy) => policy,
        };
        Box::pin(async move {
            retry::with_retry(policy, name, || {
                // Every attempt is a fresh spawn, so each takes its own copy of
                // the request rather than the first one consuming it.
                let request = clone_request(&request);
                let tool_host = Arc::clone(&tool_host);
                async {
                    match &this {
                        Self::Cursor(client, _) => client.complete(request, tool_host).await,
                        Self::Claude(client, _) => client.complete(request, tool_host).await,
                    }
                }
            })
            .await
        })
    }
}

/// A copy of `request` for one retry attempt.
///
/// [`Request`] is not [`Clone`]: `grants.workspace` holds a borrowed
/// `wasi:filesystem` descriptor. That costs nothing here — the host takes the
/// lend before any backend sees the request, so the field is always `None` at
/// this point, and the spawned agents reach the tree through the tool host's
/// `local_path` instead.
fn clone_request(request: &Request) -> Request {
    Request {
        model: request.model.clone(),
        system: request.system.clone(),
        messages: request.messages.clone(),
        generation: request.generation.clone(),
        format: request.format.clone(),
        tools: request.tools.clone(),
        grants: Grants {
            references: request.grants.references.clone(),
            workspace: None,
        },
    }
}

/// Which spawned agent to connect, and how hard to retry it.
#[derive(Debug, Clone, Copy)]
pub struct ConnectOptions {
    /// The backend `EMERY_MODEL_BACKEND` selected.
    pub selection: Selection,
    /// The transport-failure retry policy shared by both backends.
    pub retry: retry::Policy,
}

/// The `EMERY_MODEL_BACKEND` choice.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Selection {
    /// `cursor-agent`.
    #[default]
    Cursor,
    /// Claude Code.
    Claude,
}

impl Selection {
    /// The `EMERY_MODEL_BACKEND` selection, defaulting to cursor when unset.
    ///
    /// # Errors
    ///
    /// Returns an error when the variable names neither backend.
    pub fn from_env() -> Result<Self> {
        env::var("EMERY_MODEL_BACKEND")
            .map(|value| Self::parse(&value))
            .transpose()
            .map(Option::unwrap_or_default)
    }

    /// Parse an `EMERY_MODEL_BACKEND` value.
    ///
    /// # Errors
    ///
    /// Returns an error naming both accepted values when `value` is neither.
    /// A typo must not silently fall back to the default: the operator asked
    /// for a specific backend and would otherwise be billed by the other one.
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_lowercase().as_str() {
            "cursor" | "cursor-agent" => Ok(Self::Cursor),
            "claude" | "claude-code" => Ok(Self::Claude),
            other => bail!("EMERY_MODEL_BACKEND must be `cursor` or `claude`, not `{other}`"),
        }
    }

    /// The selected backend's CLI name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cursor => "cursor-agent",
            Self::Claude => "claude",
        }
    }

    /// What to install and authenticate when this backend fails to connect.
    #[must_use]
    pub const fn install_hint(self) -> &'static str {
        match self {
            Self::Cursor => {
                "install cursor-agent, then `cursor-agent login` or export CURSOR_API_KEY \
                 (command-mode credentials, not the IDE login `cursor-agent status` reports)"
            }
            Self::Claude => {
                "install the Claude Code CLI, then `claude login` or export ANTHROPIC_API_KEY"
            }
        }
    }
}

impl FromEnv for ConnectOptions {
    fn load_env() -> Result<Self> {
        Ok(Self {
            selection: Selection::from_env()?,
            retry: retry::Policy {
                retries: env::count("EMERY_MODEL_RETRIES", 2)?,
                backoff: std::time::Duration::from_millis(env::millis(
                    "EMERY_MODEL_RETRY_BACKOFF_MS",
                    1_000,
                )?),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Selection;

    #[test]
    fn parses_both_backends() {
        assert_eq!(Selection::parse("cursor").expect("cursor"), Selection::Cursor);
        assert_eq!(Selection::parse(" CLAUDE ").expect("claude"), Selection::Claude);
        assert_eq!(Selection::parse("claude-code").expect("alias"), Selection::Claude);
    }

    #[test]
    fn rejects_an_unknown_backend() {
        let error = Selection::parse("gpt").expect_err("a typo must not fall back to the default");
        assert!(error.to_string().contains("`cursor` or `claude`"), "unexpected: {error}");
    }

    #[test]
    fn defaults_to_cursor() {
        assert_eq!(Selection::default(), Selection::Cursor);
    }
}
