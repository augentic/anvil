//! Deployment policy for the shipped `emery` binary (ADR-0011).
//!
//! The macro-facing mount and resolver expressions the hosts evaluate;
//! one invocation captures the layout once ([`assemble`]).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use engine::handler::{ExecutionPaths, Locations, PROJECT_ROOT_ENV};
use engine::resolve::RoutedId;

mod anchor;
mod resolver;

/// Guest-visible preopen name of the per-project derived cache.
pub use engine::handler::GUEST_CACHE_MOUNT as CACHE_MOUNT;
pub use resolver::Resolver;

/// Assemble one invocation's deployment layout — the pure seam over
/// explicit inputs.
///
/// Anchors the project root, captures it as [`ExecutionPaths`], and
/// creates the writable mount directories. Total by design — the
/// runtime must always boot so the guest renders every diagnostic:
/// an unanchored invocation boots in-place, and mount-creation
/// failures surface as the runtime's preopen error.
#[must_use]
pub fn assemble(invoked_dir: &Path, locations: Locations) -> ExecutionPaths {
    let paths = ExecutionPaths::new(anchor::root(invoked_dir), locations);
    // The global store is host-owned (no guest mount).
    for dir in [paths.cache_dir(), paths.project_root().to_path_buf()] {
        drop(std::fs::create_dir_all(dir));
    }
    paths
}

/// The process-wide layout, evaluated once across the macro's mount
/// and resolver expressions. The first evaluation also exports the
/// host-absolute `.` mount: guests inherit the host environment, and
/// the in-guest kernel derives the agent-visible artifact root from
/// [`PROJECT_ROOT_ENV`].
fn current() -> &'static ExecutionPaths {
    static PATHS: OnceLock<ExecutionPaths> = OnceLock::new();
    PATHS.get_or_init(|| {
        let invoked_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let paths = assemble(&invoked_dir, Locations::from_env());
        export_roots(&paths);
        paths
    })
}

/// Export the host-absolute `.` mount so guests inherit it. The
/// runtime snapshots the host environment into every guest store, so
/// one read works on both deployments (same inheritance as
/// `HTTP_ADDR`).
fn export_roots(paths: &ExecutionPaths) {
    let mount = std::path::absolute(paths.project_root())
        .unwrap_or_else(|_io| paths.project_root().to_path_buf());
    // SAFETY: one write during runtime assembly, before guest stores
    // snapshot the environment and before this process spawns any
    // concurrent environment reader.
    #[expect(unsafe_code, reason = "the guest inherits the root through the env")]
    let () = unsafe {
        std::env::set_var(PROJECT_ROOT_ENV, mount.as_os_str());
    };
}

/// Macro expression: this invocation's pre-bound HTTP trigger
/// listener, feeding the `/mcp/<axis>/<name>` reference shelves.
///
/// Binds an ephemeral loopback port, so concurrent invocations get
/// distinct ports; its local address becomes the guest-visible
/// `HTTP_ADDR`. A successful bind also injects the fully-formed
/// `MCP_URL_BASE` (`http://127.0.0.1:<port>`), which guests prefer
/// over re-deriving the base from `HTTP_ADDR`.
///
/// # Errors
///
/// Returns an error when the loopback bind fails — a startup failure.
pub fn http_listener() -> anyhow::Result<std::net::TcpListener> {
    use anyhow::Context as _;

    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))
        .context("binding an ephemeral loopback port for the MCP reference shelves")?;
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
    Resolver::new(current().clone())
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
