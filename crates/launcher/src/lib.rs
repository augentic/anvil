//! Deployment policy for the shipped `emery` binary: the macro-facing
//! mount and resolver expressions evaluated by `src/main.rs`. One
//! invocation captures the layout exactly once ([`Policy::new`]).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use project::adapter::{AdapterSelector, RoutedId};
use project::config::ProjectConfig;
use project::handler::{
    CHANGE_ROOT_ENV, DETACHED_ENV, ExecutionPaths, GUEST_CACHE_MOUNT, Locations, PROJECT_ROOT_ENV,
};
use transport::command::selectors::refresh_request;

mod anchor;
mod install;
mod resolver;

pub use install::Registry;
pub use resolver::Resolver;

/// Guest-visible preopen name of the per-project derived cache.
pub const CACHE_MOUNT: &str = GUEST_CACHE_MOUNT;

/// One invocation's deployment policy: the anchored layout plus the
/// adapter refresh set.
///
/// [`Policy::new`] is the pure assembly over explicit inputs (the
/// integration seam); the module-level accessors evaluate it exactly
/// once per process over argv, the working directory, and
/// `Locations::from_env`.
#[derive(Debug)]
pub struct Policy {
    paths: ExecutionPaths,
    /// Bare adapter names this invocation explicitly upgrades — the
    /// resolver's registry check runs for these even when a store
    /// entry exists. Derived from argv (`init <bare-name>`) plus the
    /// recorded `project.yaml` binding for `init --upgrade`.
    refresh: BTreeSet<String>,
}

impl Policy {
    /// Assemble the policy for one invocation: anchor the project
    /// root, capture the layout, create the writable mount
    /// directories, and derive the refresh set.
    ///
    /// Total by design — the runtime must always boot so the guest
    /// renders every diagnostic: argv the grammar refuses anchors at
    /// the walked working directory, and mount-directory creation
    /// failures are left for the runtime's own preopen error.
    #[must_use]
    pub fn new(invoked_dir: &Path, argv: &[String], locations: Locations) -> Self {
        // The refresh projection parses the guest grammar, so it must
        // see the same peeled view Omnia gives the guest — a stray
        // `--debug` would fail the parse and drop the anchoring.
        let argv: Vec<String> =
            argv.iter().filter(|arg| *arg != "--debug" && *arg != "--quiet").cloned().collect();
        let roots = anchor::roots(invoked_dir);
        let paths = ExecutionPaths::from_roots(&roots, locations);
        // The global store is host-owned (no guest mount); the install
        // leg creates it on demand.
        for dir in [paths.cache_dir(), paths.project_root().to_path_buf()] {
            drop(std::fs::create_dir_all(dir));
        }
        let refresh = refresh_names(&argv, paths.project_root());
        Self { paths, refresh }
    }

    /// Host directory of the writable `.` project mount.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        self.paths.project_root()
    }

    /// Bare adapter names this invocation forces a registry check for
    /// (the `init` refresh surface).
    #[must_use]
    pub const fn refresh(&self) -> &BTreeSet<String> {
        &self.refresh
    }

    /// Host directory of the writable cache mount, named
    /// [`CACHE_MOUNT`].
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.paths.cache_dir()
    }

    /// The fail-closed adapters-only guest resolver over this
    /// invocation's captured layout and refresh set.
    #[must_use]
    pub fn resolver(&self) -> Resolver {
        Resolver::new(self.paths.clone(), self.refresh.clone())
    }
}

/// The bare adapter names one invocation explicitly upgrades: argv's
/// refresh projection, widened with the recorded `project.yaml`
/// binding for `init --upgrade`. Best-effort by design — an unreadable
/// or non-bare record simply refreshes nothing (the guest handler
/// renders the diagnostic).
fn refresh_names(argv: &[String], project_root: &Path) -> BTreeSet<String> {
    let request = refresh_request(argv);
    let mut names: BTreeSet<String> = request.names.into_iter().collect();
    if request.recorded_adapter
        && let Ok(config) = ProjectConfig::load(project_root)
        && let Some(adapter) = config.adapter
        && let Ok(AdapterSelector::Bare { name }) = AdapterSelector::parse(&adapter)
    {
        names.insert(name);
    }
    names
}

/// The process-wide policy, evaluated once across the macro's mount
/// and resolver expressions. The first evaluation also exports the
/// host-absolute `.` mount: guests inherit the host environment, and
/// the in-guest kernel derives the agent-visible artifact root from
/// [`PROJECT_ROOT_ENV`].
fn current() -> &'static Policy {
    static POLICY: OnceLock<Policy> = OnceLock::new();
    POLICY.get_or_init(|| {
        let invoked_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // Lossy is safe here: non-UTF-8 argv fails the grammar (no
        // refresh) and the runtime itself refuses it with a typed error.
        let argv: Vec<String> =
            std::env::args_os().skip(1).map(|arg| arg.to_string_lossy().into_owned()).collect();
        let policy = Policy::new(&invoked_dir, &argv, Locations::from_env());
        export_roots(&policy.paths);
        policy
    })
}

/// Export the host-absolute `.` mount so guests inherit it. The
/// runtime snapshots the host environment into every guest store, so
/// one read works on both deployments (same inheritance as
/// `HTTP_ADDR`).
fn export_roots(paths: &ExecutionPaths) {
    let mount = std::path::absolute(paths.project_root())
        .unwrap_or_else(|_io| paths.project_root().to_path_buf());
    let change = std::path::absolute(paths.change_root())
        .unwrap_or_else(|_io| paths.change_root().to_path_buf());
    // SAFETY: one write during runtime assembly, before guest stores
    // snapshot the environment and before this process spawns any
    // concurrent environment reader.
    #[expect(unsafe_code, reason = "the guest inherits the roots through the env")]
    let () = unsafe {
        std::env::set_var(PROJECT_ROOT_ENV, mount.as_os_str());
        std::env::set_var(CHANGE_ROOT_ENV, change.as_os_str());
        if paths.is_detached() {
            std::env::set_var(DETACHED_ENV, "1");
        } else {
            std::env::remove_var(DETACHED_ENV);
        }
    };
}

/// Macro expression: this invocation's pre-bound HTTP trigger
/// listener, feeding the `/mcp/<axis>/<name>` reference shelves.
///
/// Its local address becomes the guest-visible `HTTP_ADDR` (distinct
/// ports across concurrent invocations); an operator-set `HTTP_ADDR`
/// must bind, else an ephemeral loopback port — any bind failure is a
/// startup failure. A successful bind also injects the fully-formed
/// `MCP_URL_BASE` (`http://127.0.0.1:<port>`), which guests prefer
/// over re-deriving the base from `HTTP_ADDR`.
///
/// # Errors
///
/// Returns an error when an operator-set `HTTP_ADDR` is invalid or
/// cannot be bound, or when the ephemeral loopback bind fails.
pub fn http_listener() -> anyhow::Result<std::net::TcpListener> {
    use anyhow::Context as _;

    let listener = if let Some(addr) = std::env::var_os("HTTP_ADDR") {
        let addr = addr.to_string_lossy().into_owned();
        std::net::TcpListener::bind(&addr)
            .with_context(|| format!("binding operator-set HTTP_ADDR `{addr}`"))?
    } else {
        std::net::TcpListener::bind(("127.0.0.1", 0))
            .context("binding an ephemeral loopback port for the MCP reference shelves")?
    };
    let port =
        listener.local_addr().context("reading the bound trigger listener's address")?.port();
    // SAFETY: one write during runtime assembly, before guest stores
    // snapshot the environment and before this process spawns any
    // concurrent environment reader.
    #[expect(unsafe_code, reason = "the guest inherits the shelf base through the env")]
    let () = unsafe {
        // The IPv4 loopback literal, never `localhost`: an agent whose
        // resolver prefers `::1` would fail to connect to the listener.
        std::env::set_var("MCP_URL_BASE", format!("http://127.0.0.1:{port}"));
    };
    Ok(listener)
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

/// Macro expression: the fail-closed adapters-only guest resolver.
#[must_use]
pub fn resolver() -> Resolver {
    current().resolver()
}

/// Macro expression: the deployment's `http_paths:` hook, mapping
/// MCP reference-shelf paths to guest identities.
///
/// `/mcp/<axis>/<name>[@<version>]` maps back onto the routed adapter
/// id the guest was faulted in under. Fail closed: an unmatched path
/// or definitive resolver miss is an ordinary 404; a fault on a
/// claimed shelf is an error-logged 500.
#[must_use]
pub fn mcp_route(path: &str) -> Option<omnia::GuestId> {
    let rest = path.strip_prefix("/mcp/")?;
    let (axis, rest) = rest.split_once('/')?;
    let adapter = rest.split('/').next().filter(|segment| !segment.is_empty())?;
    let routed = RoutedId::parse(&format!("{axis}:{adapter}")).ok()?;
    Some(omnia::GuestId::from(routed.to_string()))
}
