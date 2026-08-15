//! Deployment policy for the shipped `emery` binary: the macro-facing
//! mount and resolver expressions evaluated by `src/main.rs`. One
//! invocation captures the layout exactly once ([`Policy::new`]).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use project::adapter::{AdapterSelector, RoutedId};
use project::config::ProjectConfig;
use project::handler::{
    CHANGE_ROOT_ENV, DETACHED_ENV, ExecutionPaths, GUEST_CACHE_MOUNT, GUEST_STAGING_MOUNT,
    GUEST_WORKSPACES_MOUNT, Locations, PROJECT_ROOT_ENV,
};
use transport::command::selectors::{
    SeedRequest, change_request, refresh_request, seed_request, system_request,
};

mod anchor;
mod blobstore;
mod exec_mode;
mod install;
mod resolver;
mod vcs;

pub use blobstore::Blobstore;
pub use exec_mode::ExecMode;
pub use install::Registry;
pub use resolver::Resolver;
pub use vcs::Vcs;

/// Compiled first-party adapter catalog for detached binding.
#[must_use]
pub fn catalog() -> project::adapter::catalog::Catalog {
    project::adapter::catalog::Catalog::first_party()
}

/// Compiled model-capability profile table.
#[must_use]
pub fn profiles() -> project::profile::Table {
    project::profile::Table::compiled()
}

/// Guest-visible preopen name of the per-project derived cache.
pub const CACHE_MOUNT: &str = GUEST_CACHE_MOUNT;

/// Guest-visible preopen name of the private-workspaces root.
pub const WORKSPACES_MOUNT: &str = GUEST_WORKSPACES_MOUNT;

/// Guest-visible preopen name of the VCS staging root.
pub const STAGING_MOUNT: &str = GUEST_STAGING_MOUNT;

/// Fallback guest name for the definition preopen when argv carries
/// no `--from` (or the named directory does not exist). Distinct from
/// the seed preopen's self-named path so the two never collide.
const DEFINITION_FALLBACK_MOUNT: &str = "/emery-definition";

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
    /// Read-only preopen of `plan author --from`. `None` when argv
    /// carries no `--from` or the named directory does not exist —
    /// the guest then renders the author diagnostic itself.
    definition_dir: Option<PathBuf>,
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
        // The seed projection parses the guest grammar, so it must see
        // the same peeled view Omnia gives the guest — a stray
        // `--debug` would fail the parse and drop the anchoring.
        let argv: Vec<String> =
            argv.iter().filter(|arg| *arg != "--debug" && *arg != "--quiet").cloned().collect();
        let seed = seed_request(&argv);
        let system = system_request(&argv);
        let change = change_request(&argv);
        let (paths, system_invocation) = if let Some(system) = system.as_ref() {
            // A `system *` invocation mounts a definition home: the cache
            // stays under `$EMERY_HOME` on one shared tenant (never keyed
            // off the mounted root), and the home is never created.
            let root = anchor::system_root(invoked_dir, system);
            let locations = locations.shared_cache("system");
            (ExecutionPaths::new(root, locations), true)
        } else {
            let roots = anchor::roots(invoked_dir, seed.as_ref(), change.change_dir.as_deref());
            (ExecutionPaths::from_roots(&roots, locations), false)
        };
        // The global store is host-owned (no guest mount); the install
        // leg creates it on demand. Same for the snapshot store. The
        // `.` mount is product (in-place) or the change home (detached).
        let workspaces_dir = paths.locations().workspaces_root().to_path_buf();
        let staging_dir = paths.locations().staging_root().to_path_buf();
        let mut dirs = vec![paths.cache_dir(), workspaces_dir, staging_dir];
        if !system_invocation {
            dirs.push(paths.project_root().to_path_buf());
        }
        for dir in dirs {
            drop(std::fs::create_dir_all(dir));
        }
        let seed_dir = seed.as_ref().and_then(|request| seed_dir(request, paths.project_root()));
        let definition_dir = change.from.as_ref().and_then(|from| {
            let resolved =
                if from.is_absolute() { from.clone() } else { paths.project_root().join(from) };
            resolved.is_dir().then(|| std::path::absolute(&resolved).unwrap_or(resolved))
        });
        let refresh = refresh_names(&argv, paths.project_root());
        Self {
            paths,
            seed_dir,
            definition_dir,
            refresh,
        }
    }

    /// Host directory of the writable `.` mount: product tree when
    /// in-place, change home when detached.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        self.paths.project_root()
    }

    /// Host directory of the change home.
    #[must_use]
    pub fn change_root(&self) -> &Path {
        self.paths.change_root()
    }

    /// Whether this invocation has no ambient product root.
    #[must_use]
    pub const fn is_detached(&self) -> bool {
        self.paths.is_detached()
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

    /// Host directory of the VCS staging mount, named
    /// [`STAGING_MOUNT`]. Read-only in-guest: the host stages and
    /// discards; the guest only reads (and snapshots) staged trees.
    #[must_use]
    pub fn staging_dir(&self) -> PathBuf {
        self.paths.locations().staging_root().to_path_buf()
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

    /// Host directory of the read-only definition preopen when argv
    /// carries `--from` whose directory exists, else the `.` mount.
    #[must_use]
    pub fn definition_dir(&self) -> &Path {
        self.definition_dir.as_deref().unwrap_or_else(|| self.paths.project_root())
    }

    /// Guest-visible name of the definition preopen: the absolute host
    /// path when `--from` resolved, else `/emery-definition`.
    #[must_use]
    pub fn definition_mount_name(&self) -> String {
        self.definition_dir
            .as_ref()
            .map_or_else(|| DEFINITION_FALLBACK_MOUNT.to_string(), |dir| dir.display().to_string())
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
        // project root), else the walked project root itself.
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
/// and resolver expressions. The first evaluation also exports the
/// host-absolute `.` mount, change home, and detached flag: guests
/// inherit the host environment, and the in-guest kernel derives the
/// agent-visible artifact root from [`PROJECT_ROOT_ENV`].
fn current() -> &'static Policy {
    static POLICY: OnceLock<Policy> = OnceLock::new();
    POLICY.get_or_init(|| {
        let invoked_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // Lossy is safe here: non-UTF-8 argv fails the grammar (no
        // seed) and the runtime itself refuses it with a typed error.
        let argv: Vec<String> =
            std::env::args_os().skip(1).map(|arg| arg.to_string_lossy().into_owned()).collect();
        let policy = Policy::new(&invoked_dir, &argv, Locations::from_env());
        export_roots(&policy.paths);
        policy
    })
}

/// Export the host-absolute `.` mount, change home, and detached flag
/// so guests inherit them.
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
/// Its local address becomes the guest-visible `HTTP_ADDR`, so
/// concurrent invocations get distinct ports. An operator-set
/// `HTTP_ADDR` must bind; without one, an ephemeral loopback port.
/// Any bind failure is a startup failure — there is no
/// run-without-the-trigger fallback.
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

/// Macro expression: host directory of the read-only staging mount.
#[must_use]
pub fn staging_dir() -> PathBuf {
    current().staging_dir()
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

/// Macro expression: guest-visible name of the read-only definition preopen.
#[must_use]
pub fn definition_mount_name() -> String {
    current().definition_mount_name()
}

/// Macro expression: host directory of the read-only definition preopen.
#[must_use]
pub fn definition_mount_path() -> PathBuf {
    current().definition_dir().to_path_buf()
}

/// Macro expression: the fail-closed adapters-only guest resolver.
#[must_use]
pub fn resolver() -> Resolver {
    current().resolver()
}

/// Macro expression: the deployment's `http_paths:` hook, mapping
/// adapter MCP reference-shelf paths to guest identities.
///
/// `/mcp/<axis>/<name>[@<version>]` maps back onto the routed adapter
/// id the guest was faulted in under, so the component's `wasi:http`
/// export serves the shelf. Fail closed: a path outside the grammar
/// or a definitive resolver miss is an ordinary 404 (never a
/// catch-all onto the engine guest); a genuine fault on a claimed
/// shelf is Omnia's error-logged 500.
#[must_use]
pub fn mcp_route(path: &str) -> Option<omnia::GuestId> {
    let rest = path.strip_prefix("/mcp/")?;
    let (axis, rest) = rest.split_once('/')?;
    let adapter = rest.split('/').next().filter(|segment| !segment.is_empty())?;
    let routed = RoutedId::parse(&format!("{axis}:{adapter}")).ok()?;
    Some(omnia::GuestId::from(routed.to_string()))
}
