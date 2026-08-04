//! [`DevModel`] — the case runner's live [`Model`] backend.
//!
//! A lazily connected cursor backend (`omnia_cursor::Client`, the
//! host-side `WasiModelCtx` backend) behind the shared [`Native`]
//! bridge, which performs the guest-request mapping, the host request
//! gate, the workspace-lend → tool-host path resolution, and the answer
//! projection. The connection happens on first use so deterministic
//! commands never require cursor-agent on `PATH`; clones share the
//! connection cell, so each constructed backend connects cursor-agent
//! at most once (the case runner constructs one per run).
//!
//! Cursor `ConnectOptions::from_env` (via `Client::connect`) reads:
//! - `CURSOR_MODEL=<model-id>` — default when a request leaves `model`
//!   unset; blank/unset lets `cursor-agent` choose. A guest-supplied id
//!   always wins.
//! - `CURSOR_TIMEOUT_SECS=<u64>` — per-spawn wall-clock bound (backend
//!   default 600s); the `cargo make eval` tasks raise it for live cases.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use omnia::Backend as _;
use omnia_guest::Model;
use omnia_guest::model::{Error, Reply, Request};
use tracing::Instrument as _;

use super::native::Native;
use crate::telemetry;

/// The case runner's model backend: lazily connected live completions.
#[derive(Clone, Debug)]
pub struct DevModel {
    /// The project root workspace lends resolve to.
    root: PathBuf,
    /// The shared connection, established by the first judgment leg.
    cell: Arc<tokio::sync::OnceCell<Native<omnia_cursor::Client>>>,
}

impl DevModel {
    /// A lazily connected cursor backend rooted at `project_dir`.
    /// Model id and timeout come from cursor's `ConnectOptions::from_env`.
    #[must_use]
    pub fn new(project_dir: &Path) -> Self {
        Self {
            root: project_dir.to_path_buf(),
            cell: Arc::new(tokio::sync::OnceCell::new()),
        }
    }

    async fn request(&self, request: Request) -> Result<Reply, Error> {
        let native = self
            .cell
            .get_or_try_init(|| async {
                let client = omnia_cursor::Client::connect().await?;
                Ok::<_, anyhow::Error>(Native::new(client, self.root.clone()))
            })
            .await
            .map_err(|err| {
                Error::Backend(format!(
                    "cursor-agent backend unavailable: {err:#}; install cursor-agent, \
                     then `cursor-agent login` or export CURSOR_API_KEY (command-mode \
                     credentials, not the IDE login `cursor-agent status` reports)"
                ))
            })?;
        native.create(request).await
    }
}

impl Model for DevModel {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        // The `model.request` span records only the bounded leg and
        // effective model id — never request bodies or project paths.
        // When unset here, cursor may still apply `CURSOR_MODEL`.
        let span = tracing::info_span!(
            "model.request",
            leg = %telemetry::leg(&request),
            model = %request.model.as_deref().unwrap_or("backend-default"),
        );
        self.request(request).instrument(span).await
    }
}
