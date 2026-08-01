//! Launcher-facing anchoring projection over the shared command
//! grammar.
//!
//! Every invocation runs in the guest — help, version, grammar
//! rejections, and `adapter add` included — so the launcher projects
//! only pre-boot facts from argv: the `adapter add` seed request
//! ([`seed_request`]), whose `--project-dir` anchors the project
//! mount and whose component path earns a read-only preopen (the
//! operator's component may live outside every other mount), and the
//! adapter refresh set ([`refresh_request`]) naming the bare adapters
//! an `adapter upgrade` / `init` invocation explicitly refreshes
//! through the resolver's registry check. Rather than duplicating the clap
//! surface, both parse argv through the *same* assembled router
//! grammar the engine guest executes. Argv that fails the grammar
//! projects nothing: the deployment falls back to cwd-anchored mounts
//! and the guest renders the rejection.

use clap::FromArgMatches;
use omnia_guest::api::invoke::Invoker;
#[cfg(not(target_arch = "wasm32"))]
use omnia_guest::model::{Reply, Request};
use project::adapter::{AdapterSelector, ResolvedSource, ResolvedTarget, Resolver};
use project::handler::{Anchor, CachePlacement, ExecutionPaths, Locations};
use project::seam::wire::BuildReport;
use project::seam::{self, Evidence, Input, Lead, MergePhase, WorkingTree};

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
    /// relative values anchor at the invocation directory.
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
    async fn survey(&self, _id: String) -> Result<Vec<Lead>, seam::Error> {
        never_dispatched!()
    }

    async fn extract(&self, _id: String, _lead: Lead) -> Result<Evidence, seam::Error> {
        never_dispatched!()
    }
}

impl seam::Target for Grammar {
    async fn guidance(&self, _id: String) -> Result<String, seam::Error> {
        never_dispatched!()
    }

    async fn build(
        &self, _id: String, _slice: String, _inputs: Vec<Input>, _context: seam::BuildContext,
        _tree: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        never_dispatched!()
    }

    async fn merge(
        &self, _id: String, _slice: String, _phase: MergePhase, _tree: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        never_dispatched!()
    }
}
