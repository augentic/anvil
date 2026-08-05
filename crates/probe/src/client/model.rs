//! [`DevModel`] — the case runner's live [`Model`] backend.
//!
//! A lazily connected spawned-agent backend (`model::ModelBackend`, the
//! host-side `WasiModelCtx` backend the shipped binary also links)
//! behind the shared [`Native`] bridge, which performs the
//! guest-request mapping, the host request gate, the workspace-lend →
//! tool-host path resolution, and the answer projection. The connection
//! happens on first use so deterministic commands never require an
//! agent CLI on `PATH`; clones share the connection cell, so each
//! constructed backend connects at most once (the case runner
//! constructs one per run).
//!
//! `ModelBackend::connect` reads `EMERY_MODEL_BACKEND` to pick the
//! agent, then defers to that backend's own options:
//! - `CURSOR_MODEL` / `CLAUDE_MODEL` — default when a request leaves
//!   `model` unset; blank/unset lets the CLI choose. A guest-supplied
//!   id always wins.
//! - `CURSOR_TIMEOUT_SECS` / `CLAUDE_TIMEOUT_SECS` — per-spawn
//!   wall-clock bound (backend default 600s); the `cargo make eval`
//!   tasks raise it for live cases.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use model::{ModelBackend, Selection};
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
    cell: Arc<tokio::sync::OnceCell<Native<ModelBackend>>>,
}

impl DevModel {
    /// A lazily connected spawned-agent backend rooted at `project_dir`.
    /// Backend selection, model id, and timeout all come from the
    /// environment via `ModelBackend::connect`.
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
                let client = ModelBackend::connect().await?;
                Ok::<_, anyhow::Error>(Native::new(client, self.root.clone()))
            })
            .await
            .map_err(|err| {
                // Naming the selection re-reads the environment rather than
                // asking the backend, which is exactly what failed to connect.
                let selection = Selection::from_env().unwrap_or_default();
                Error::Backend(format!(
                    "{} backend unavailable: {err:#}; {}",
                    selection.name(),
                    selection.install_hint()
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
