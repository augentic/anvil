//! Typed deployment assembly — the value the binary maps onto an
//! Omnia manifest.

use std::path::PathBuf;

use error::Error;
use project::handler::{ExecutionPaths, GUEST_CACHE_MOUNT, GUEST_STORE_MOUNT};

use crate::hydrate::ResolvedClosure;

/// The WIT interfaces the engine guest imports and the host polyfills
/// from adapter exports — the deployment link allow-list. Versions
/// track `wit/specify.wit`.
const LINKS: [&str; 2] = ["specify:adapter/source@0.1.0", "specify:adapter/target@0.1.0"];

/// The engine guest's deployment id: argv routes to it, and it is the
/// only guest with linked imports.
const ENGINE_GUEST_ID: &str = "specify";

/// One derived deployment, in memory — never written as `omnia.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deployment {
    /// The engine guest.
    pub engine: Guest,
    /// One guest per closure adapter, id `<axis>:<name>`.
    pub adapters: Vec<Guest>,
    /// The three well-known mounts: the project root at `.`, the
    /// per-project cache, and the global adapter store.
    pub mounts: Vec<Mount>,
}

/// One deployment guest entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guest {
    /// Deployment guest id (`specify`, `source:<name>`, `target:<name>`).
    pub id: String,
    /// The verified component file.
    pub component: PathBuf,
    /// Host-linked import interfaces (engine only).
    pub links: Vec<String>,
}

/// One writable preopen granted to every guest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    /// Guest-visible mount name.
    pub name: String,
    /// Host directory.
    pub path: PathBuf,
    /// Whether guests may write through the mount.
    pub writable: bool,
}

/// Assemble the typed deployment for a verified closure, deriving the
/// mount sources from the invocation's one carried [`ExecutionPaths`].
pub fn assemble(paths: &ExecutionPaths, closure: ResolvedClosure) -> Result<Deployment, Error> {
    let cache_dir = paths.cache_dir();
    std::fs::create_dir_all(&cache_dir)?;
    let store_root = paths.locations().store_root().to_path_buf();
    std::fs::create_dir_all(&store_root)?;

    Ok(Deployment {
        engine: Guest {
            id: ENGINE_GUEST_ID.to_string(),
            component: closure.engine_component,
            links: LINKS.iter().map(ToString::to_string).collect(),
        },
        adapters: closure
            .adapters
            .into_iter()
            .map(|adapter| Guest {
                id: adapter.guest_id,
                component: adapter.component,
                links: Vec::new(),
            })
            .collect(),
        mounts: vec![
            Mount {
                name: ".".to_string(),
                path: paths.project_root().to_path_buf(),
                writable: true,
            },
            Mount {
                name: GUEST_CACHE_MOUNT.to_string(),
                path: cache_dir,
                writable: true,
            },
            Mount {
                name: GUEST_STORE_MOUNT.to_string(),
                path: store_root,
                writable: true,
            },
        ],
    })
}
