//! Deployment policy for the shipped `emery` binary: the macro-facing
//! mount and resolver expressions behind a dynamic Omnia deployment.
//!
//! There is no host front door. The `omnia::runtime!` invocation in
//! `src/main.rs` embeds the engine guest as static component bytes
//! and evaluates this crate's expressions for everything the
//! deployment needs before boot: the well-known mounts (anchored from
//! argv and the working directory), the fail-closed guest
//! [`Resolver`], the pre-bound HTTP trigger listener
//! ([`http_listener`]), and the MCP reference-shelf path hook
//! ([`mcp_route`]). Every invocation then runs in the guest — help,
//! version, grammar rejections, and `adapter add` included; argv and
//! the engine guest's exit code pass through byte-for-byte, except
//! the reserved host log flags (`--debug` / `--quiet`), which Omnia
//! peels before the guest sees argv.
//!
//! Pinned routed ids resolve the immutable global store and install a
//! missing entry from the compiled first-party OCI registry
//! (pull-on-miss — the launcher is the only downloader in the
//! deployment); unpinned ids resolve the anchored project's seeded
//! component cache, verify-and-load only. Store resolves stay
//! digest-gated fail closed at load time, and the store itself is
//! host-owned — the guest gets no store mount. The embedded engine
//! never touches the resolver: it is registered statically at boot.
//!
//! `EMERY_HOME` remains a relocation override only — the Cargo
//! model: everything anchors at the user home or the project root by
//! default, and one invocation captures the layout exactly once
//! ([`Policy::new`] over `Locations::from_env`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use project::adapter::{AdapterSelector, RoutedId};
use project::config::ProjectConfig;
use project::handler::{ExecutionPaths, GUEST_CACHE_MOUNT, GUEST_WORKSPACES_MOUNT, Locations};
use transport::command::selectors::{SeedRequest, refresh_request, seed_request};

mod anchor;
mod install;
mod resolver;
mod workspaces;

pub use install::Registry;
pub use resolver::Resolver;
pub use workspaces::Workspaces;

/// Guest-visible preopen name of the per-project derived cache.
pub const CACHE_MOUNT: &str = GUEST_CACHE_MOUNT;

/// Guest-visible preopen name of the private-workspaces root.
pub const WORKSPACES_MOUNT: &str = GUEST_WORKSPACES_MOUNT;

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
    /// Bare adapter names this invocation explicitly upgrades — the
    /// resolver's registry check runs for these even when a store
    /// entry exists. Derived from argv (`adapter upgrade <name>` /
    /// `--all`, `init <bare-name>`) plus the recorded `project.yaml`
    /// binding for `init --upgrade` and the project's bare bindings
    /// for `adapter upgrade --all`.
    refresh: BTreeSet<String>,
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
        // Omnia peels the reserved host log flags before the guest sees
        // argv; the seed projection parses the guest grammar, so it must
        // see the same peeled view or a stray `--debug` would fail the
        // parse and drop the anchoring.
        let argv: Vec<String> =
            argv.iter().filter(|arg| *arg != "--debug" && *arg != "--quiet").cloned().collect();
        let seed = seed_request(&argv);
        let root = anchor::project_root(invoked_dir, seed.as_ref());
        let paths = ExecutionPaths::new(root, locations);
        // The global store is host-owned (no guest mount); the install
        // leg creates it on demand. Same for the snapshot store — the
        // workspace backend's kernel creates it on first write.
        let workspaces_dir = paths.locations().workspaces_root().to_path_buf();
        for dir in [paths.project_root().to_path_buf(), paths.cache_dir(), workspaces_dir] {
            drop(std::fs::create_dir_all(dir));
        }
        let seed_dir = seed.as_ref().and_then(|request| seed_dir(request, paths.project_root()));
        let refresh = refresh_names(&argv, paths.project_root());
        Self {
            paths,
            seed_dir,
            refresh,
        }
    }

    /// Host directory of the writable project mount, named `.`.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        self.paths.project_root()
    }

    /// Bare adapter names this invocation forces a registry check for
    /// (`adapter upgrade` / `init` refresh surface).
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

    /// Host directory of the writable private-workspaces mount, named
    /// [`WORKSPACES_MOUNT`].
    #[must_use]
    pub fn workspaces_dir(&self) -> PathBuf {
        self.paths.locations().workspaces_root().to_path_buf()
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
    /// invocation's captured layout and refresh set.
    #[must_use]
    pub fn resolver(&self) -> Resolver {
        Resolver::new(self.paths.clone(), self.refresh.clone())
    }
}

/// The bare adapter names one invocation explicitly upgrades: argv's
/// refresh projection, widened with the recorded `project.yaml`
/// binding for `init --upgrade` and with every bare project binding
/// for `adapter upgrade --all`. Best-effort by design — an unreadable
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
    if request.all_bindings {
        // Same anchoring as the in-guest kernel: an explicit
        // `--project-dir` wins (relative values join the mounted
        // project root — guest `with_root` against `.`), else the
        // walked project root itself.
        let root = request.project_dir.map_or_else(
            || project_root.to_path_buf(),
            |dir| if dir.is_absolute() { dir } else { project_root.join(dir) },
        );
        if let Ok(bindings) = project::adapter::upgrade::targets(&root) {
            names.extend(bindings);
        }
    }
    names
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

/// Macro expression: this invocation's pre-bound HTTP trigger
/// listener, feeding the `/mcp/<axis>/<name>` reference shelves.
///
/// One listener drives both ends of the loop — the trigger server
/// adopts it, and Omnia injects its local address as the guest-visible
/// `HTTP_ADDR` every adapter guest derives its grant URLs from — so
/// concurrent `emery` invocations get distinct ports instead of
/// contending on a fixed default, with no environment mutation and no
/// drop-then-rebind window.
///
/// Split policy: an operator-set `HTTP_ADDR` must bind — an invalid or
/// occupied address is a startup failure. Without one, bind an
/// ephemeral loopback port. Writing the `http_listener:` key means
/// supplying a listener, so any bind failure is a startup failure —
/// there is no run-without-the-trigger fallback.
///
/// # Errors
///
/// Returns an error when an operator-set `HTTP_ADDR` is invalid or
/// cannot be bound, or when the ephemeral loopback bind fails.
pub fn http_listener() -> anyhow::Result<std::net::TcpListener> {
    use anyhow::Context as _;

    if let Some(addr) = std::env::var_os("HTTP_ADDR") {
        let addr = addr.to_string_lossy().into_owned();
        return std::net::TcpListener::bind(&addr)
            .with_context(|| format!("binding operator-set HTTP_ADDR `{addr}`"));
    }
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .context("binding an ephemeral loopback port for the MCP reference shelves")
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

/// Macro expression: host directory of the writable workspaces mount.
#[must_use]
pub fn workspaces_dir() -> PathBuf {
    current().workspaces_dir()
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

/// Macro expression: the deployment's `http_paths:` hook, mapping
/// adapter MCP reference-shelf paths to guest identities.
///
/// Every judgment dispatch grants the spawned agent
/// `http://127.0.0.1:<port>/mcp/<axis>/<name>[@<version>]` (the
/// adapter SDK's `mcp_url`, on this invocation's pre-bound listener
/// port); this hook maps that path back onto the routed adapter id
/// `<axis>:<name>[@<version>]` — the exact identity the adapter guest
/// was faulted in under, so the registry lookup hits and the
/// component's `wasi:http` `handle()` export serves the shelf. Fail
/// closed: a path outside the routed grammar is `None`, an ordinary
/// 404 — never a catch-all onto the engine guest — and a definitive
/// resolver miss on a claimed identity stays a 404, while a genuine
/// fault (resolution failure, or a routed guest without the
/// `wasi:http` handler export) is Omnia's error-logged 500.
#[must_use]
pub fn mcp_route(path: &str) -> Option<omnia::GuestId> {
    let rest = path.strip_prefix("/mcp/")?;
    let (axis, rest) = rest.split_once('/')?;
    let adapter = rest.split('/').next().filter(|segment| !segment.is_empty())?;
    let routed = RoutedId::parse(&format!("{axis}:{adapter}")).ok()?;
    Some(omnia::GuestId::from(routed.to_string()))
}
