//! Host ingest: Git/HTTPS I/O re-exported from `project::binding`, plus
//! the WIT backend the engine guest calls during wave binding.

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use omnia::Backend;
use omnia_wasi_ingest::{Fetched, FutureResult, WasiIngestCtx};
use project::binding::{Cache, Meter, Policy};
pub use project::binding::{checkout, fetch_https as fetch, resolve as ingest};
use project::handler::{ExecutionPaths, GUEST_WORKSPACES_MOUNT};
use project::snapshot::SnapshotId;
use project::workspace::Store;

/// The ingest backend over this invocation's captured layout.
#[derive(Clone, Debug)]
pub struct Ingest {
    paths: ExecutionPaths,
    cache: Arc<Mutex<Cache>>,
}

impl Backend for Ingest {
    type ConnectOptions = omnia::NoOptions;

    async fn connect_with(_options: omnia::NoOptions) -> anyhow::Result<Self> {
        Ok(Self {
            paths: super::current().paths.clone(),
            cache: Arc::new(Mutex::new(Cache::new())),
        })
    }
}

impl WasiIngestCtx for Ingest {
    fn fetch(
        &self, locator: String, recorded: Option<String>, prior: Option<String>,
    ) -> FutureResult<Fetched> {
        let paths = self.paths.clone();
        let cache = Arc::clone(&self.cache);
        let recorded = recorded.and_then(|value| SnapshotId::parse(&value).ok());
        Box::pin(async move {
            let store = Store::new(paths.locations().snapshots_root());
            let scratch = paths.locations().workspaces_root().join("ingest");
            std::fs::create_dir_all(&scratch)
                .with_context(|| format!("create ingest scratch {}", scratch.display()))?;
            let policy = Policy::standard();
            let mut meter = Meter::new();
            let mut intern = cache.lock().expect("ingest cache").clone();
            let fetched = {
                let mut session = project::binding::Session {
                    store: &store,
                    scratch: &scratch,
                    change_root: paths.change_root(),
                    cache: &mut intern,
                    policy: &policy,
                    meter: &mut meter,
                };
                project::binding::fetch_locator(
                    &mut session,
                    &locator,
                    recorded.as_ref(),
                    prior.as_deref(),
                )
                .await
                .map_err(|err| anyhow::anyhow!(err.to_string()))?
            };
            *cache.lock().expect("ingest cache") = intern;
            Ok(Fetched {
                locator: fetched.locator,
                cid: fetched.cid.to_string(),
                root: guest_root(&paths, Path::new(&fetched.root)),
                warning: fetched.warning,
            })
        })
    }
}

fn guest_root(paths: &ExecutionPaths, host: &Path) -> String {
    if let Ok(rel) = host.strip_prefix(paths.locations().workspaces_root()) {
        let rel = rel.display();
        if rel.to_string().is_empty() {
            return GUEST_WORKSPACES_MOUNT.to_string();
        }
        return format!("{GUEST_WORKSPACES_MOUNT}/{rel}");
    }
    if let Ok(rel) = host.strip_prefix(paths.project_root()) {
        let rel = rel.display().to_string();
        if rel.is_empty() {
            return ".".into();
        }
        return rel;
    }
    host.display().to_string()
}
