//! Deployment policy for the shipped `emery` binary (ADR-0011).
//!
//! The macro-facing mount and resolver expressions the hosts evaluate;
//! one invocation captures the layout once ([`assemble`]).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Guest-visible preopen name of the per-project derived cache.
pub use engine::handler::GUEST_CACHE_MOUNT as CACHE_MOUNT;
use engine::handler::{ExecutionPaths, Locations, PROJECT_ROOT_ENV};
use engine::resolve::{AdapterSelector, FIRST_PARTY_NAMESPACE, RoutedId, resolver as locate};
use error::Error;

// The embedded first-party registry (ADR-0002 §2): `build.rs` stages
// `EMERY_EMBED_DIR` components (release: first-party adapters;
// journey: the mock). Without the env the table is empty.
include!(concat!(env!("OUT_DIR"), "/embedded.rs"));

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
    let paths = ExecutionPaths::new(root(invoked_dir), locations);
    // The global store is host-owned (no guest mount).
    for dir in [paths.cache_dir(), paths.project_root().to_path_buf()] {
        drop(std::fs::create_dir_all(dir));
    }
    paths
}

/// The nearest ancestor carrying `.emery/project.yaml`. A miss
/// anchors in-place — pre-init, so `emery init` stays legal and
/// later verbs fail typed in-guest (`not-initialized`).
fn root(invoked_dir: &Path) -> PathBuf {
    invoked_dir
        .ancestors()
        .find(|candidate| engine::project::Project::path(candidate).try_exists().unwrap_or(false))
        .map_or_else(|| invoked_dir.to_path_buf(), Path::to_path_buf)
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

/// The Emery guest resolver over one captured [`ExecutionPaths`].
///
/// Local-only resolution: project cache seed, embedded first-party
/// registry, verified store entry. There is no download path
/// (ADR-0002 deletions): nothing local is a typed miss.
#[derive(Clone, Debug)]
pub struct Resolver {
    paths: ExecutionPaths,
}

impl Resolver {
    /// Bind the resolver to the invocation's captured layout.
    #[must_use]
    pub const fn new(paths: ExecutionPaths) -> Self {
        Self { paths }
    }

    /// Resolve one adapter identity to its verified component bytes.
    ///
    /// # Errors
    ///
    /// `adapter-not-found` when nothing local satisfies the identity;
    /// `adapter-sidecar-missing` / `adapter-digest-mismatch` /
    /// `adapter-store-unreadable` when a store entry fails
    /// verify-on-read; `adapter-routed-id-malformed` for identities
    /// outside the routed grammar.
    pub fn resolve_component(&self, id: &str) -> Result<Vec<u8>, Error> {
        let routed = RoutedId::parse(id)?;
        // The co-dev seed always wins, pins included — a locally
        // built component would otherwise be shadowed at dispatch.
        if let Some(bytes) = self.seed(&routed)? {
            return Ok(bytes);
        }
        // The embedded entry is the unpinned *default* (ADR-0002 §2):
        // an exact pin is an explicit operator decision, so it resolves
        // the verified store only, never the binary's own bytes.
        if routed.version.is_none()
            && let Some(bytes) = embedded(&routed.name)
        {
            log_use(&routed, None, "embedded");
            return Ok(bytes);
        }
        let selector = match routed.version.clone() {
            Some(version) => AdapterSelector::Package {
                namespace: FIRST_PARTY_NAMESPACE.to_string(),
                name: routed.name.clone(),
                version,
            },
            None => AdapterSelector::Bare {
                name: routed.name.clone(),
            },
        };
        let location = locate::locate(routed.axis, &selector, &routed.name, &self.paths)?;
        log_use(&routed, routed.version.as_ref(), "store");
        Ok(std::fs::read(location.path())?)
    }

    /// The seeded project-cache entry for this identity's name, when
    /// one exists. The seed answers pinned and bare identities alike.
    fn seed(&self, routed: &RoutedId) -> Result<Option<Vec<u8>>, Error> {
        let name = routed.name.as_str();
        let bare = AdapterSelector::Bare {
            name: name.to_string(),
        };
        let Ok(location) = locate::locate(routed.axis, &bare, name, &self.paths) else {
            return Ok(None);
        };
        log_use(routed, None, "cache seed");
        Ok(Some(std::fs::read(location.path())?))
    }
}

/// The embedded first-party component for `name`, when the binary
/// carries one.
fn embedded(name: &str) -> Option<Vec<u8>> {
    EMBEDDED.iter().find_map(|(entry, bytes)| (*entry == name).then(|| bytes.to_vec()))
}

/// One stderr line per settled adapter identity — with the host
/// version, the run's version audit trail (project files record no
/// adapter versions).
fn log_use(routed: &RoutedId, version: Option<&semver::Version>, origin: &str) {
    let identity = version.map_or_else(
        || format!("{}:{}", routed.axis.prefix(), routed.name),
        |version| format!("{}:{}@{version}", routed.axis.prefix(), routed.name),
    );
    eprintln!("emery {}: using {identity} ({origin})", env!("CARGO_PKG_VERSION"));
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
