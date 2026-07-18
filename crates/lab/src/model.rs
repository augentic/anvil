//! [`DevModel`] — the trial's live [`Model`] backend.
//!
//! A lazily connected cursor backend (`omnia_cursor::Client`, the
//! host-side `WasiModelCtx` backend) behind the shared [`Native`]
//! bridge, which performs the guest-request mapping, the host request
//! gate, the `lend_workspace` → project-root tool host, and the answer
//! projection. The connection happens on first use so deterministic
//! phases never require cursor-agent on `PATH`; clones share the
//! connection cell, so each constructed backend connects cursor-agent
//! at most once (the trial constructs one per phase).
//!
//! `SPECIFY_EVAL_MODEL=<model-id>` overrides the model for a run: the
//! id fills `Request.model` only when the caller left it `None`, so a
//! guest-supplied id always wins. Read once at construction; unset or
//! blank means no override.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use omnia::Backend as _;
use omnia_guest::Model;
use omnia_guest::model::{Error, Reply, Request};

use crate::native::Native;

/// The trial's model backend: lazily connected live completions.
#[derive(Clone, Debug)]
pub struct DevModel {
    /// The project root workspace lends resolve to.
    root: PathBuf,
    /// Driver-side model-id override from `SPECIFY_EVAL_MODEL`.
    model: Option<String>,
    /// The shared connection, established by the first judgment leg.
    cell: Arc<tokio::sync::OnceCell<Native<omnia_cursor::Client>>>,
}

impl DevModel {
    /// A lazily connected cursor backend rooted at `project_dir`,
    /// reading the optional `SPECIFY_EVAL_MODEL` override once.
    #[must_use]
    pub fn new(project_dir: &Path) -> Self {
        Self {
            root: project_dir.to_path_buf(),
            model: std::env::var("SPECIFY_EVAL_MODEL").ok().filter(|id| !id.trim().is_empty()),
            cell: Arc::new(tokio::sync::OnceCell::new()),
        }
    }
}

impl Model for DevModel {
    async fn create(&self, mut request: Request) -> Result<Reply, Error> {
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
        // A guest-supplied model id always wins over the driver override.
        if request.model.is_none() {
            request.model = self.model.clone();
        }
        native.create(request).await
    }
}
