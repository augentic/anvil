//! Native deployment policy for the shipped `emery` binary (ADR-0011).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Guest preopen name of the per-project cache.
pub use engine::handler::GUEST_CACHE_MOUNT as CACHE_MOUNT;
use engine::handler::{ExecutionPaths, Locations, PROJECT_ROOT_ENV};
use engine::resolve::resolver::locate;
use engine::resolve::{AdapterSelector, FIRST_PARTY_NAMESPACE, RoutedId};
use error::Error;

// Empty unless `EMERY_EMBED_DIR` stages components.
include!(concat!(env!("OUT_DIR"), "/embedded.rs"));

/// Anchor the project, capture the layout, and create the cache mount.
///
/// Total: an unanchored CWD boots in-place; a failed mkdir surfaces as
/// the runtime's preopen error so the guest can still render.
#[must_use]
pub fn assemble(invoked_dir: &Path, locations: Locations) -> ExecutionPaths {
    let root = invoked_dir
        .ancestors()
        .find(|candidate| engine::project::Project::path(candidate).try_exists().unwrap_or(false))
        .map_or_else(|| invoked_dir.to_path_buf(), Path::to_path_buf);
    let paths = ExecutionPaths::new(root, locations);
    // Cache mount only — the store is host-owned and has no guest preopen.
    drop(std::fs::create_dir_all(paths.cache_dir()));
    paths
}

/// Bind the MCP trigger listener and export `MCP_URL_BASE`.
///
/// # Errors
///
/// The loopback bind fails.
pub fn http_listener() -> anyhow::Result<std::net::TcpListener> {
    use anyhow::Context as _;

    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))
        .context("binding an ephemeral loopback port for the MCP reference shelves")?;
    let port =
        listener.local_addr().context("reading the bound trigger listener's address")?.port();
    // SAFETY: once, at assembly, before guest stores snapshot the env.
    #[expect(unsafe_code, reason = "the guest inherits the shelf base through the env")]
    let () = unsafe {
        // IPv4 literal: `localhost` can resolve to `::1` and miss the bind.
        std::env::set_var("MCP_URL_BASE", format!("http://127.0.0.1:{port}"));
    };
    Ok(listener)
}

/// Host path for the writable `.` mount.
#[must_use]
pub fn project_root() -> PathBuf {
    current().project_root().to_path_buf()
}

/// Host path for the writable cache mount.
#[must_use]
pub fn cache_dir() -> PathBuf {
    current().cache_dir()
}

/// Fail-closed adapters-only guest resolver.
#[must_use]
pub fn resolver() -> Resolver {
    Resolver::new(current().clone())
}

fn current() -> &'static ExecutionPaths {
    // Shared across the macro's separate mount/resolver expressions.
    static PATHS: OnceLock<ExecutionPaths> = OnceLock::new();
    PATHS.get_or_init(|| {
        let invoked_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let paths = assemble(&invoked_dir, Locations::from_env());
        let mount = std::path::absolute(paths.project_root())
            .unwrap_or_else(|_io| paths.project_root().to_path_buf());
        // SAFETY: once, at assembly, before guest stores snapshot the env.
        #[expect(unsafe_code, reason = "the guest inherits the root through the env")]
        let () = unsafe {
            std::env::set_var(PROJECT_ROOT_ENV, mount.as_os_str());
        };
        paths
    })
}

/// Map `/mcp/<axis>/<name>` to a guest id; anything else is `None`.
#[must_use]
pub fn mcp_route(path: &str) -> Option<omnia::GuestId> {
    let rest = path.strip_prefix("/mcp/")?;
    let (axis, rest) = rest.split_once('/')?;
    let adapter = rest.split('/').next().filter(|segment| !segment.is_empty())?;
    let routed = RoutedId::parse(&format!("{axis}:{adapter}")).ok()?;
    Some(omnia::GuestId::from(routed.to_string()))
}

/// Local-only adapter resolution over one captured layout.
#[derive(Clone, Debug)]
pub struct Resolver {
    paths: ExecutionPaths,
}

impl Resolver {
    /// Bind to this invocation's layout.
    #[must_use]
    pub const fn new(paths: ExecutionPaths) -> Self {
        Self { paths }
    }

    /// Resolve `id` to verified component bytes.
    ///
    /// # Errors
    ///
    /// A typed miss, a store verify failure, or a malformed routed id.
    pub fn resolve_component(&self, id: &str) -> Result<Vec<u8>, Error> {
        let routed = RoutedId::parse(id)?;
        let cache = self.paths.locations().component(self.paths.project_root(), &routed.name);
        if cache.is_file() {
            log_use(&routed, "cache seed");
            return Ok(std::fs::read(cache)?);
        }
        if routed.version.is_none()
            && let Some(bytes) = EMBEDDED
                .iter()
                .find_map(|(entry, bytes)| (*entry == routed.name).then(|| bytes.to_vec()))
        {
            log_use(&routed, "embedded");
            return Ok(bytes);
        }
        let selector = match &routed.version {
            Some(version) => AdapterSelector::Package {
                namespace: FIRST_PARTY_NAMESPACE.to_string(),
                name: routed.name.clone(),
                version: version.clone(),
            },
            None => AdapterSelector::Bare {
                name: routed.name.clone(),
            },
        };
        let location = locate(routed.axis, &selector, &routed.name, &self.paths)?;
        log_use(&routed, "store");
        Ok(std::fs::read(location.path())?)
    }
}

impl omnia::GuestResolver for Resolver {
    fn resolve(
        &self, guest: omnia::GuestId, _expected_export: String,
    ) -> omnia::FutureResult<Option<omnia::GuestArtifact>> {
        let resolver = self.clone();
        Box::pin(async move {
            let bytes = resolver.resolve_component(guest.as_str()).map_err(anyhow::Error::new)?;
            Ok(Some(omnia::GuestArtifact::wasm(bytes)))
        })
    }
}

/// Project files record no adapter versions — this is the audit trail.
fn log_use(routed: &RoutedId, origin: &str) {
    eprintln!("emery {}: using {routed} ({origin})", env!("CARGO_PKG_VERSION"));
}
