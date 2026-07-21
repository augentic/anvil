//! Launcher-facing adapter-selector projection over the shared command
//! grammar.
//!
//! The deployment launcher (RFC-70) derives each invocation's
//! component closure before the runtime starts, so it must see every
//! adapter selector argv can carry. Rather than duplicating the clap
//! surface, [`from_argv`] parses argv through the *same* assembled
//! router grammar the engine guest executes and folds the typed values
//! of the selector-bearing routes into [`CommandSelectors`]. Argv that
//! fails the shared grammar is rejected with clap's own rendered
//! diagnostic — the launcher fails closed before anything is started,
//! byte-identical to what the guest would print.
//!
//! Help and version displays are answered host-side
//! ([`Projection::Display`]): the shared grammar renders them
//! byte-identically to the guest, so no deployment is assembled just
//! to print usage. Command *semantics* stay with the engine guest —
//! only clap's own displays short-circuit.

use clap::FromArgMatches;
use clap::error::ErrorKind;
use omnia_guest::api::invoke::Invoker;
#[cfg(not(target_arch = "wasm32"))]
use omnia_guest::model::{Reply, Request};
use project::adapter::{AdapterSelector, ResolvedSource, ResolvedTarget, Resolver};
use project::handler::{Anchor, CachePlacement, ExecutionPaths, Locations};
use project::seam::wire::BuildReport;
use project::seam::{self, Evidence, Input, Lead, MergePhase, WorkingTree};

use super::{Format, Globals};

/// The adapter selectors one argv carries, per axis, plus the route's
/// state-derived closure legs and the parsed output format (for
/// rendering any pre-run failure the way the guest would).
///
/// Values are the raw argv tokens (`omnia`, `typescript@1.2.0`,
/// `./mock.wasm`) — the launcher owns their [`AdapterSelector`]
/// interpretation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandSelectors {
    /// Parsed `--format` global (default `text`).
    pub format: Format,
    /// The state-derived closure legs this route can dispatch.
    pub scope: ClosureScope,
    /// Source-axis selector tokens (`source resolve <value>`,
    /// `plan author --source <key>=<adapter>[:…]`, the `--intent`
    /// sugar's implicit `intent`).
    pub sources: Vec<String>,
    /// Target-axis selector tokens (`init <adapter>`,
    /// `target resolve <value>`).
    pub targets: Vec<String>,
    /// The `adapter add` cache-seed request, when this argv carries
    /// one. Not a closure requirement — the launcher performs the
    /// seed host-side and renders its report without starting the
    /// runtime, because the operator path may live outside the engine
    /// guest's mounts.
    pub seed: Option<SeedRequest>,
}

/// The `adapter add` arguments the launcher needs to seed the project
/// component cache host-side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedRequest {
    /// Local `.wasm` component path exactly as argv carried it;
    /// relative paths anchor at the selected project directory.
    pub component: std::path::PathBuf,
    /// The `--project-dir` value, when supplied; relative values
    /// anchor at the invocation directory.
    pub project_dir: Option<std::path::PathBuf>,
}

/// The state-derived closure legs one command route can dispatch —
/// the launcher joins a leg's adapters only when the routed verb can
/// reach them, so read-only verbs deploy the engine guest alone.
///
/// Argv-carried selectors ([`CommandSelectors::sources`] /
/// [`CommandSelectors::targets`]) always join the closure and are not
/// a leg here. The default (all `false`) is the engine-only scope.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClosureScope {
    /// The project's bound target adapter (`project.yaml.adapter`).
    pub project_target: bool,
    /// Every plan-bound source (`plan.yaml.sources.<key>`).
    pub plan_sources: bool,
    /// Pinned workspace-slot targets from `.specify/topology.lock`.
    pub slot_targets: bool,
}

impl ClosureScope {
    /// Engine guest only: the verb dispatches no adapter.
    pub const ENGINE: Self = Self {
        project_target: false,
        plan_sources: false,
        slot_targets: false,
    };
    /// Every leg — verbs that can reach any bound adapter.
    pub const FULL: Self = Self {
        project_target: true,
        plan_sources: true,
        slot_targets: true,
    };
    /// Every plan-bound source adapter.
    pub const PLAN_SOURCES: Self = Self {
        plan_sources: true,
        ..Self::ENGINE
    };
    /// The project's bound target adapter.
    pub const PROJECT_TARGET: Self = Self {
        project_target: true,
        ..Self::ENGINE
    };
    /// Project target plus plan sources (the refine rhythm: extract
    /// fan-out, then synthesis reading the target's guidance).
    pub const REFINE: Self = Self {
        project_target: true,
        plan_sources: true,
        slot_targets: false,
    };
    /// Workspace-slot targets (the topology re-derivation surface).
    pub const SLOT_TARGETS: Self = Self {
        slot_targets: true,
        ..Self::ENGINE
    };
}

/// Outcome of projecting one argv through the shared grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Projection {
    /// Argv parses: forward to the engine guest with these selectors.
    Forward(CommandSelectors),
    /// A help or version display: clap's rendered text, to print on
    /// stdout with exit 0 — byte-identical to what the guest would
    /// print, so no deployment is needed.
    Display {
        /// The rendered help/version text.
        rendered: String,
    },
    /// Argv fails the shared grammar: clap's rendered diagnostic, to
    /// print on stderr with exit 2, nothing started.
    Rejected {
        /// The rendered clap error, exactly as the guest would print it.
        rendered: String,
    },
}

/// Project the adapter selectors out of `argv` (without the program
/// name) through the shared command grammar.
#[must_use]
pub fn from_argv(argv: &[String]) -> Projection {
    let mut full = Vec::with_capacity(argv.len() + 1);
    full.push("specify".to_string());
    full.extend(argv.iter().cloned());
    let matches = match grammar().try_get_matches_from(full) {
        Ok(matches) => matches,
        Err(error)
            if matches!(error.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) =>
        {
            return Projection::Display {
                rendered: error.render().to_string(),
            };
        }
        Err(error) => {
            return Projection::Rejected {
                rendered: error.render().to_string(),
            };
        }
    };

    let format = Globals::from_arg_matches(&matches).map_or(Format::Text, |globals| globals.format);
    let (path, leaf) = selected(&matches);
    let segments: Vec<&str> = path.iter().map(String::as_str).collect();
    let mut selectors = CommandSelectors {
        format,
        scope: scope(&segments),
        ..CommandSelectors::default()
    };

    match segments.as_slice() {
        ["init"] => {
            if let Ok(args) = super::InitArgs::from_arg_matches(leaf)
                && let Some(adapter) = args.adapter
            {
                selectors.targets.push(adapter);
            }
        }
        ["adapter", "add"] => {
            if let Ok(args) = super::adapter::AddArgs::from_arg_matches(leaf) {
                selectors.seed = Some(SeedRequest {
                    component: args.component,
                    project_dir: args.project_dir,
                });
            }
        }
        ["source", "resolve"] => {
            if let Ok(args) = super::source::ResolveArgs::from_arg_matches(leaf) {
                selectors.sources.push(args.value);
            }
        }
        ["target", "resolve"] => {
            if let Ok(args) = super::target::ResolveArgs::from_arg_matches(leaf) {
                selectors.targets.push(args.value);
            }
        }
        ["plan", "author"] => {
            if let Ok(args) = super::plan::AuthorArgs::from_arg_matches(leaf) {
                selectors.sources.extend(args.sources.into_iter().map(|assign| assign.adapter));
                if args.intent.is_some() {
                    selectors.sources.push("intent".to_string());
                }
            }
        }
        _ => {}
    }
    Projection::Forward(selectors)
}

/// The command routes [`from_argv`] folds selectors out of.
///
/// The grammar-coverage guard in `tests/selectors.rs` classifies every
/// route in the router inventory against this list, so a new
/// selector-bearing verb cannot land without surfacing there.
pub const SELECTOR_ROUTES: &[&[&str]] = &[
    &["init"],
    &["adapter", "add"],
    &["plan", "author"],
    &["source", "resolve"],
    &["target", "resolve"],
];

/// Every command route's state-derived [`ClosureScope`] — which
/// project/plan/topology legs the launcher joins into the deployment
/// for that verb.
///
/// The classification follows the routed operation's actual dispatch
/// reach: a leg is `true` exactly when the verb (or an orchestration
/// it drives) can resolve or dispatch adapters from that state
/// surface. Argv-carried selectors always join and need no leg.
/// `registry add --adapter` names a *peer project's* target in
/// `registry.yaml` without resolving it; `plan add` / `plan amend`
/// bindings reference `plan.yaml.sources` *keys*, not adapter names —
/// both stay engine-only.
///
/// The grammar-coverage guard in `tests/selectors.rs` requires every
/// route in the router inventory to appear here, so a new verb cannot
/// land unclassified; a route that somehow escapes the guard falls
/// back to [`ClosureScope::FULL`], fail-safe.
pub const SCOPE_ROUTES: &[(&[&str], ClosureScope)] = &[
    // Pre-project and debug resolution: argv selectors only, plus the
    // recorded project adapter for `init --upgrade` re-entry.
    (&["init"], ClosureScope::PROJECT_TARGET),
    // A deterministic cache copy: the component path is the copy's
    // input, not an adapter requirement the deployment must
    // enumerate. The launcher performs the seed and renders its
    // report host-side without starting the runtime (the operator
    // path may live outside the guest's mounts); the route stays
    // classified for the coverage guard and for the native host,
    // which dispatches the verb directly.
    (&["adapter", "add"], ClosureScope::ENGINE),
    (&["completions"], ClosureScope::ENGINE),
    (&["source", "resolve"], ClosureScope::ENGINE),
    (&["target", "resolve"], ClosureScope::ENGINE),
    // Source-axis breakouts resolve their binding from plan.yaml.
    (&["source", "survey"], ClosureScope::PLAN_SOURCES),
    (&["source", "extract"], ClosureScope::PLAN_SOURCES),
    // The slice loop: refine extracts per bound source and synthesis
    // reads the target's guidance; build/merge dispatch the target.
    (&["slice", "refine"], ClosureScope::REFINE),
    (&["slice", "build"], ClosureScope::PROJECT_TARGET),
    (&["slice", "merge", "run"], ClosureScope::PROJECT_TARGET),
    (&["slice", "merge", "preview"], ClosureScope::ENGINE),
    (&["slice", "merge", "conflict-check"], ClosureScope::ENGINE),
    (&["slice", "list"], ClosureScope::ENGINE),
    (&["slice", "validate"], ClosureScope::ENGINE),
    (&["slice", "provenance"], ClosureScope::ENGINE),
    (&["slice", "model", "show"], ClosureScope::ENGINE),
    (&["slice", "drop"], ClosureScope::ENGINE),
    // The plan loop: author surveys its argv-bound sources and
    // resolves the regular project's target; next resolves the claimed
    // entry's best-effort `$TARGET`; validate re-derives workspace
    // slot topology; execute drives the whole refine→build→merge
    // rhythm.
    (&["plan", "author"], ClosureScope::REFINE),
    (&["plan", "next"], ClosureScope::PROJECT_TARGET),
    (&["plan", "validate"], ClosureScope::SLOT_TARGETS),
    (&["plan", "execute"], ClosureScope::FULL),
    (&["plan", "status"], ClosureScope::ENGINE),
    (&["plan", "add"], ClosureScope::ENGINE),
    (&["plan", "amend"], ClosureScope::ENGINE),
    (&["plan", "remove"], ClosureScope::ENGINE),
    (&["plan", "transition"], ClosureScope::ENGINE),
    (&["plan", "archive"], ClosureScope::ENGINE),
    // Read-only projections and deterministic maintenance.
    (&["archive", "prune"], ClosureScope::ENGINE),
    (&["journal", "emit"], ClosureScope::ENGINE),
    (&["journal", "show"], ClosureScope::ENGINE),
    (&["registry", "validate"], ClosureScope::ENGINE),
    (&["registry", "add"], ClosureScope::ENGINE),
    (&["registry", "remove"], ClosureScope::ENGINE),
];

/// Look up one selected route's closure scope. Help and version
/// displays never reach here (they forward the default engine-only
/// scope); an unclassified route — reachable only if the coverage
/// guard were bypassed — joins every leg, fail-safe.
fn scope(segments: &[&str]) -> ClosureScope {
    SCOPE_ROUTES
        .iter()
        .find(|(route, _)| *route == segments)
        .map_or(ClosureScope::FULL, |(_, scope)| *scope)
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

/// The assembled specify clap grammar, identical to the executing
/// router's — built over a provider that never dispatches.
fn grammar() -> clap::Command {
    let invoker = Invoker::new(
        "specify",
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
    super::router(invoker)
        .expect("the specify route inventory is statically valid")
        .command()
        .clone()
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
        &self, _id: String, _slice: String, _inputs: Vec<Input>, _tree: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        never_dispatched!()
    }

    async fn merge(
        &self, _id: String, _slice: String, _phase: MergePhase, _tree: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        never_dispatched!()
    }
}
