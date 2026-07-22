//! Deployment policy for the shipped `specify` binary: the macro-facing
//! mount and resolver expressions behind a dynamic Omnia deployment
//! (RFC-70 Stage 3).
//!
//! There is no host front door. The `omnia::runtime!` invocation in
//! `src/omnia.rs` embeds the engine guest as static component bytes
//! and evaluates this crate's expressions for everything the
//! deployment needs before boot: the well-known mounts (anchored from
//! argv and the working directory) and the fail-closed guest
//! [`Resolver`]. Every invocation then runs in the guest — help,
//! version, grammar rejections, and `adapter add` included; argv and
//! the engine guest's exit code pass through byte-for-byte.
//!
//! Every adapter identity is verify-and-load only: pinned routed ids
//! resolve the immutable global store, unpinned ids the anchored
//! project's seeded component cache — both populated exclusively by
//! the engine guest's own ensure legs through the writable mounts
//! before any dispatch can miss. Store resolves stay digest-gated
//! fail closed at load time. The embedded engine never touches the
//! resolver: it is registered statically at boot.
//!
//! `SPECIFY_HOME` remains a relocation override only — the Cargo
//! model: everything anchors at the user home or the project root by
//! default, and one invocation captures the layout exactly once
//! ([`Policy::new`] over `Locations::from_env`).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use project::handler::{ExecutionPaths, GUEST_CACHE_MOUNT, GUEST_STORE_MOUNT, Locations};
use transport::command::selectors::{SeedRequest, seed_request};

mod anchor;
mod resolver;

pub use resolver::Resolver;

/// Guest-visible preopen name of the per-project derived cache.
pub const CACHE_MOUNT: &str = GUEST_CACHE_MOUNT;

/// Guest-visible preopen name of the global adapter store.
pub const STORE_MOUNT: &str = GUEST_STORE_MOUNT;

/// One invocation's deployment policy: the anchored layout plus the
/// optional `adapter add` seed preopen.
///
/// [`Policy::new`] is the pure assembly over explicit inputs (the
/// integration seam); the module-level accessors evaluate it exactly
/// once per process over argv, the working directory, and
/// `Locations::from_env`.
#[derive(Debug)]
pub struct Policy {
    paths: ExecutionPaths,
    /// Read-only preopen of the `adapter add` component's parent
    /// directory, named by its absolute host path so the guest opens
    /// the argv path unchanged. `None` when argv carries no seed (or
    /// the directory does not exist — the guest then renders
    /// `adapter-component-missing` itself).
    seed_dir: Option<PathBuf>,
}

impl Policy {
    /// Assemble the policy for one invocation: anchor the project
    /// root, capture the layout, create the writable mount
    /// directories, and derive the seed preopen.
    ///
    /// Total by design — the runtime must always boot so the guest
    /// renders every diagnostic: argv the grammar refuses anchors at
    /// the walked working directory, and mount-directory creation
    /// failures are left for the runtime's own preopen error.
    #[must_use]
    pub fn new(invoked_dir: &Path, argv: &[String], locations: Locations) -> Self {
        let seed = seed_request(argv);
        let root = anchor::project_root(invoked_dir, seed.as_ref());
        let paths = ExecutionPaths::new(root, locations);
        let store_root = paths.locations().store_root().to_path_buf();
        for dir in [paths.project_root().to_path_buf(), paths.cache_dir(), store_root] {
            drop(std::fs::create_dir_all(dir));
        }
        let seed_dir = seed.as_ref().and_then(|request| seed_dir(request, paths.project_root()));
        Self { paths, seed_dir }
    }

    /// Host directory of the writable project mount, named `.`.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        self.paths.project_root()
    }

    /// Host directory of the writable cache mount, named
    /// [`CACHE_MOUNT`].
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.paths.cache_dir()
    }

    /// Host directory of the writable store mount, named
    /// [`STORE_MOUNT`].
    #[must_use]
    pub fn store_dir(&self) -> PathBuf {
        self.paths.locations().store_root().to_path_buf()
    }

    /// The read-only self-named preopen: the `adapter add` component's
    /// parent directory when argv carries a seed whose directory
    /// exists, else the project root — a harmless duplicate of the `.`
    /// mount that also serves absolute in-project paths.
    #[must_use]
    pub fn seed_dir(&self) -> &Path {
        self.seed_dir.as_deref().unwrap_or_else(|| self.paths.project_root())
    }

    /// Guest-visible name of the seed preopen: its own absolute host
    /// path, so the guest opens the operator's argv path unchanged.
    #[must_use]
    pub fn seed_mount_name(&self) -> String {
        self.seed_dir().display().to_string()
    }

    /// The fail-closed adapters-only guest resolver over this
    /// invocation's captured layout.
    #[must_use]
    pub fn resolver(&self) -> Resolver {
        Resolver::new(self.paths.clone())
    }
}

/// The absolute parent directory of the seed component, resolved the
/// way the in-guest kernel resolves it (relative paths anchor at the
/// project root); `None` when it does not exist.
fn seed_dir(request: &SeedRequest, project_root: &Path) -> Option<PathBuf> {
    let component = if request.component.is_absolute() {
        request.component.clone()
    } else {
        project_root.join(&request.component)
    };
    let parent = component.parent()?;
    parent.is_dir().then(|| parent.to_path_buf())
}

/// The process-wide policy, evaluated once across the macro's mount
/// and resolver expressions.
fn current() -> &'static Policy {
    static POLICY: OnceLock<Policy> = OnceLock::new();
    POLICY.get_or_init(|| {
        let invoked_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // Lossy is safe here: non-UTF-8 argv fails the grammar (no
        // seed) and the runtime itself refuses it with a typed error.
        let argv: Vec<String> =
            std::env::args_os().skip(1).map(|arg| arg.to_string_lossy().into_owned()).collect();
        Policy::new(&invoked_dir, &argv, Locations::from_env())
    })
}

/// Macro expression: host directory of the writable `.` project mount.
#[must_use]
pub fn project_root() -> PathBuf {
    current().project_root().to_path_buf()
}

/// Macro expression: host directory of the writable cache mount.
#[must_use]
pub fn cache_dir() -> PathBuf {
    current().cache_dir()
}

/// Macro expression: host directory of the writable store mount.
#[must_use]
pub fn store_dir() -> PathBuf {
    current().store_dir()
}

/// Macro expression: guest-visible name of the read-only seed preopen.
#[must_use]
pub fn seed_mount_name() -> String {
    current().seed_mount_name()
}

/// Macro expression: host directory of the read-only seed preopen.
#[must_use]
pub fn seed_mount_path() -> PathBuf {
    current().seed_dir().to_path_buf()
}

/// Macro expression: the fail-closed adapters-only guest resolver.
#[must_use]
pub fn resolver() -> Resolver {
    current().resolver()
}
