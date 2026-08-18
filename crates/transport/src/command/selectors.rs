//! Launcher-facing anchoring projection over the shared command
//! grammar (never a duplicated clap surface). Argv that fails the
//! grammar projects nothing — the guest renders the rejection.

use clap::FromArgMatches;
use omnia_guest::api::invoke::Invoker;
use project::adapter::{AdapterSelector, ResolvedSource, ResolvedTarget, Resolver};
use project::handler::{Anchor, CachePlacement, ExecutionPaths, Locations};

/// The adapter refresh facts one invocation carries: the explicit
/// upgrade surface the launcher forces a registry check for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefreshRequest {
    /// Bare adapter names argv refreshes directly (`init <bare-name>`).
    pub names: Vec<String>,
    /// `init --upgrade`: the launcher additionally refreshes the
    /// project's recorded adapter binding (read from `project.yaml`
    /// at the anchored root) when that binding is bare.
    pub recorded_adapter: bool,
}

/// Project the adapter refresh set out of `argv` (without the program
/// name) through the shared command grammar.
///
/// Only `init` refreshes: `init <bare-name>` refreshes that name and
/// `init --upgrade` flags the recorded binding. Everything else — and
/// argv the grammar refuses — projects the empty default, so normal
/// resolution stays local-first.
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
        ["init"] => {
            let Ok(init) = super::routes::InitArgs::from_arg_matches(leaf) else {
                return RefreshRequest::default();
            };
            RefreshRequest {
                names: init.adapter.as_deref().and_then(bare_name).into_iter().collect(),
                recorded_adapter: init.upgrade,
            }
        }
        _ => RefreshRequest::default(),
    }
}

/// The kebab name when `value` parses as a bare adapter selector —
/// pinned references are immutable and local components are seeded at
/// init, so neither joins the refresh set.
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
/// router's — built over a provider that never dispatches. Assembled
/// once per process: every selector projection clones the cached tree
/// instead of re-running the full router assembly.
fn grammar() -> clap::Command {
    static GRAMMAR: std::sync::OnceLock<clap::Command> = std::sync::OnceLock::new();
    GRAMMAR
        .get_or_init(|| {
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
            super::router(invoker)
                .expect("the emery route inventory is statically valid")
                .command()
                .clone()
        })
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

impl Resolver for Grammar {
    fn resolve_source(
        &self, _selector: &AdapterSelector, _paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, error::Error> {
        unreachable!("the grammar-only provider never dispatches")
    }

    fn resolve_target(
        &self, _selector: &AdapterSelector, _paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, error::Error> {
        unreachable!("the grammar-only provider never dispatches")
    }
}
