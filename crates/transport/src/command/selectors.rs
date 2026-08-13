//! Launcher-facing anchoring projection over the shared command
//! grammar (never a duplicated clap surface). Argv that fails the
//! grammar projects nothing — the guest renders the rejection.

use clap::FromArgMatches;
use omnia_guest::api::invoke::Invoker;
#[cfg(not(target_arch = "wasm32"))]
use omnia_guest::model::{Reply, Request};
use project::adapter::{AdapterSelector, ResolvedSource, ResolvedTarget, Resolver};
use project::handler::{Anchor, CachePlacement, ExecutionPaths, Locations};
use project::seam::wire::{BuildReport, PhaseReport, RepairOrigin};
use project::seam::{self, Evidence, Input, Lead, MergePhase};

/// The `adapter add` arguments the launcher needs to anchor the
/// project mount and preopen the operator's component directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedRequest {
    /// Local `.wasm` component path exactly as argv carried it;
    /// relative paths anchor at the selected project directory.
    pub component: std::path::PathBuf,
    /// The `--project-dir` value, when supplied; relative values
    /// anchor at the invocation directory.
    pub project_dir: Option<std::path::PathBuf>,
}

/// Project the `adapter add` seed request out of `argv` (without the
/// program name) through the shared command grammar.
///
/// `None` for every other route and for argv the grammar refuses —
/// including help and version displays, which the guest renders.
#[must_use]
pub fn seed_request(argv: &[String]) -> Option<SeedRequest> {
    let mut full = Vec::with_capacity(argv.len() + 1);
    full.push("emery".to_string());
    full.extend(argv.iter().cloned());
    let matches = grammar().try_get_matches_from(full).ok()?;

    let (path, leaf) = selected(&matches);
    let segments: Vec<&str> = path.iter().map(String::as_str).collect();
    if segments.as_slice() != ["adapter", "add"] {
        return None;
    }
    let add = super::adapter::AddArgs::from_arg_matches(leaf).ok()?;
    Some(SeedRequest {
        component: add.component,
        project_dir: add.project_dir,
    })
}

/// The adapter refresh facts one invocation carries: the explicit
/// upgrade surface the launcher forces a registry check for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefreshRequest {
    /// Bare adapter names argv refreshes directly (`adapter upgrade
    /// <name>`, `init <bare-name>`).
    pub names: Vec<String>,
    /// `init --upgrade`: the launcher additionally refreshes the
    /// project's recorded adapter binding (read from `project.yaml`
    /// at the anchored root) when that binding is bare.
    pub recorded_adapter: bool,
    /// `adapter upgrade --all`: the launcher widens the set with every
    /// bare binding the project records (`project.yaml` target plus
    /// `plan.yaml` sources).
    pub all_bindings: bool,
    /// The `adapter upgrade --project-dir` value, when supplied;
    /// relative values join the mounted project root (guest
    /// `with_root` against `.`).
    pub project_dir: Option<std::path::PathBuf>,
}

/// Project the adapter refresh set out of `argv` (without the program
/// name) through the shared command grammar.
///
/// Only the explicit upgrade surface refreshes: `adapter upgrade
/// <name>` / `--all` and `init` (`init <bare-name>` refreshes that
/// name; `init --upgrade` flags the recorded binding). Everything
/// else — and argv the grammar refuses — projects the empty default,
/// so normal resolution stays local-first.
#[must_use]
pub fn refresh_request(argv: &[String]) -> RefreshRequest {
    let mut full = Vec::with_capacity(argv.len() + 1);
    full.push("emery".to_string());
    full.extend(argv.iter().cloned());
    let Ok(matches) = grammar().try_get_matches_from(full) else {
        return RefreshRequest::default();
    };

    let (path, leaf) = selected(&matches);
    let segments: Vec<&str> = path.iter().map(String::as_str).collect();
    match segments.as_slice() {
        ["adapter", "upgrade"] => {
            let Ok(upgrade) = super::adapter::UpgradeArgs::from_arg_matches(leaf) else {
                return RefreshRequest::default();
            };
            RefreshRequest {
                names: upgrade.name.as_deref().and_then(bare_name).into_iter().collect(),
                recorded_adapter: false,
                all_bindings: upgrade.all,
                project_dir: upgrade.project_dir,
            }
        }
        ["init"] => {
            let Ok(init) = super::routes::InitArgs::from_arg_matches(leaf) else {
                return RefreshRequest::default();
            };
            RefreshRequest {
                names: init.adapter.as_deref().and_then(bare_name).into_iter().collect(),
                recorded_adapter: init.upgrade,
                all_bindings: false,
                project_dir: None,
            }
        }
        _ => RefreshRequest::default(),
    }
}

/// The `system *` anchoring request: the definition home the launcher
/// mounts as `.` — no `project.yaml` walk and no mkdir (creating the
/// home would be `system init`, which does not exist).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemRequest {
    /// The `--dir` value, when supplied; relative values anchor at
    /// the invocation directory.
    pub dir: Option<std::path::PathBuf>,
}

/// Project the `system *` anchoring request out of `argv` (without
/// the program name) through the shared command grammar.
///
/// `None` for every other route and for argv the grammar refuses —
/// including help and version displays, which the guest renders.
#[must_use]
pub fn system_request(argv: &[String]) -> Option<SystemRequest> {
    let mut full = Vec::with_capacity(argv.len() + 1);
    full.push("emery".to_string());
    full.extend(argv.iter().cloned());
    let matches = grammar().try_get_matches_from(full).ok()?;

    let (path, leaf) = selected(&matches);
    let segments: Vec<&str> = path.iter().map(String::as_str).collect();
    match segments.as_slice() {
        ["system", "survey"] => {
            let args = super::system::SurveyArgs::from_arg_matches(leaf).ok()?;
            Some(SystemRequest { dir: args.dir })
        }
        ["system", "plan"] => {
            let args = super::system::PlanArgs::from_arg_matches(leaf).ok()?;
            Some(SystemRequest { dir: args.dir })
        }
        ["system", "review"] => {
            let args = super::system::ReviewArgs::from_arg_matches(leaf).ok()?;
            Some(SystemRequest { dir: args.dir })
        }
        ["system", "status"] => {
            let args = super::system::StatusArgs::from_arg_matches(leaf).ok()?;
            Some(SystemRequest { dir: args.dir })
        }
        _ => None,
    }
}

/// The kebab name when `value` parses as a bare adapter selector —
/// pinned references are immutable and local components refresh
/// through `adapter add`, so neither joins the refresh set.
fn bare_name(value: &str) -> Option<String> {
    match AdapterSelector::parse(value) {
        Ok(AdapterSelector::Bare { name }) => Some(name),
        _ => None,
    }
}

/// Walk the parsed matches down to the selected leaf route.
fn selected(mut matches: &clap::ArgMatches) -> (Vec<String>, &clap::ArgMatches) {
    let mut path = Vec::new();
    while let Some((name, child)) = matches.subcommand() {
        path.push(name.to_owned());
        matches = child;
    }
    (path, matches)
}

/// The assembled emery clap grammar, identical to the executing
/// router's — built over a provider that never dispatches.
fn grammar() -> clap::Command {
    let invoker = Invoker::new(
        "emery",
        Grammar {
            // Inert explicit locations: the grammar-only provider is
            // never dispatched, so no layout (and no environment
            // capture) is ever reached — including on wasm32.
            paths: ExecutionPaths::new(
                ".",
                Locations::explicit(
                    std::path::PathBuf::new(),
                    CachePlacement::Project(std::path::PathBuf::new()),
                ),
            ),
        },
    );
    super::router(invoker).expect("the emery route inventory is statically valid").command().clone()
}

/// Grammar-only provider: satisfies the router's capability bounds so
/// the clap surface can be assembled, but is never dispatched — parse
/// projection stops at the grammar.
struct Grammar {
    paths: ExecutionPaths,
}

impl Anchor for Grammar {
    fn paths(&self) -> &ExecutionPaths {
        &self.paths
    }
}

/// Every capability body: parse projection stops at the grammar, so
/// dispatch is a routing bug, not a runtime condition.
macro_rules! never_dispatched {
    () => {
        unreachable!("the grammar-only provider never dispatches")
    };
}

impl omnia_guest::Model for Grammar {
    #[cfg(not(target_arch = "wasm32"))]
    async fn create(&self, _request: Request) -> Result<Reply, omnia_guest::model::Error> {
        never_dispatched!()
    }
}

impl Resolver for Grammar {
    fn resolve_source(
        &self, _selector: &AdapterSelector, _paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, error::Error> {
        never_dispatched!()
    }

    fn resolve_target(
        &self, _selector: &AdapterSelector, _paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, error::Error> {
        never_dispatched!()
    }
}

impl seam::Source for Grammar {
    async fn survey(
        &self, _id: String, _key: String, _input: seam::SourceInput,
    ) -> Result<Vec<Lead>, seam::Error> {
        never_dispatched!()
    }

    async fn extract(
        &self, _id: String, _key: String, _input: seam::SourceInput, _lead: Lead,
    ) -> Result<Evidence, seam::Error> {
        never_dispatched!()
    }
}

impl seam::Target for Grammar {
    async fn guidance(&self, _id: String) -> Result<String, seam::Error> {
        never_dispatched!()
    }

    async fn build(
        &self, _id: String, _slice: String, _inputs: Vec<Input>, _context: seam::BuildContext,
        _workspace: seam::Workspace,
    ) -> Result<PhaseReport, seam::Error> {
        never_dispatched!()
    }

    async fn verify(
        &self, _id: String, _workspace: seam::Workspace,
    ) -> Result<PhaseReport, seam::Error> {
        never_dispatched!()
    }

    async fn repair(
        &self, _id: String, _slice: String, _origin: RepairOrigin,
        _findings: Vec<diagnostics::Diagnostic>, _continuation: Option<Vec<u8>>,
        _workspace: seam::Workspace,
    ) -> Result<PhaseReport, seam::Error> {
        never_dispatched!()
    }

    async fn review(
        &self, _id: String, _slice: String, _continuation: Option<Vec<u8>>,
        _workspace: seam::Workspace,
    ) -> Result<PhaseReport, seam::Error> {
        never_dispatched!()
    }

    async fn merge(
        &self, _id: String, _slice: String, _phase: MergePhase, _workspace: seam::Workspace,
    ) -> Result<BuildReport, seam::Error> {
        never_dispatched!()
    }
}

impl seam::Origins for Grammar {
    async fn fetch(&self, _locator: String) -> Result<seam::Fetched, seam::Error> {
        never_dispatched!()
    }

    async fn discard_fetched(&self, _root: String) -> Result<(), seam::Error> {
        never_dispatched!()
    }
}

impl seam::Workspaces for Grammar {
    async fn freeze(&self) -> Result<project::snapshot::SnapshotId, seam::Error> {
        never_dispatched!()
    }

    async fn snapshot(&self, _path: String) -> Result<project::snapshot::SnapshotId, seam::Error> {
        never_dispatched!()
    }

    async fn prepare(
        &self, _base: project::snapshot::SnapshotId, _writable: bool,
    ) -> Result<seam::Workspace, seam::Error> {
        never_dispatched!()
    }

    async fn capture(&self, _id: String) -> Result<project::snapshot::CodePatch, seam::Error> {
        never_dispatched!()
    }

    async fn discard(&self, _id: String) -> Result<(), seam::Error> {
        never_dispatched!()
    }

    async fn apply(&self, _patch: project::snapshot::CodePatch) -> Result<(), seam::Error> {
        never_dispatched!()
    }

    async fn sweep(
        &self, _dead: Vec<project::snapshot::SnapshotId>, _live: Vec<project::snapshot::SnapshotId>,
    ) -> Result<usize, seam::Error> {
        never_dispatched!()
    }
}
