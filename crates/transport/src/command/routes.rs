//! The exhaustive route inventory (ADR-0008 §3): `init`, the reserved
//! `specify` stub, and the auto-derived `completions`. Deleted verbs
//! are gone from the grammar — no hidden routes, no aliases.

use clap::Args;
use omnia_guest::api::Provider;
use omnia_guest::api::command::{BuildError, Completions, Router, RouterBuilder, run};
use omnia_guest::api::invoke::Invoker;
use project::adapter::Resolver;
use project::handler::Anchor;

use super::specify::SpecifyArgs;
use super::{EmeryProjector, Globals, specify};

/// One-line application description.
const ABOUT: &str = "Deterministic primitives for spec-driven development";

/// Flags for `emery init`.
#[derive(Debug, Args)]
pub(super) struct InitArgs {
    /// Adapter identifier or local component path.
    pub(super) adapter: Option<String>,
    /// Project name.
    #[arg(long)]
    name: Option<String>,
    /// Project description.
    #[arg(long)]
    description: Option<String>,
    /// Comma-separated target platforms.
    #[arg(long)]
    platforms: Option<String>,
    /// Re-enter initialization to bump the Emery version pin.
    #[arg(long, conflicts_with_all = ["adapter", "name", "description"])]
    pub(super) upgrade: bool,
}

/// Assemble the complete Emery command router.
///
/// # Errors
///
/// Returns a deterministic route or argument conflict.
pub fn router<P>(invoker: Invoker<P>) -> Result<Router<P, Globals>, BuildError>
where
    P: Provider + Anchor + Resolver,
{
    let command = clap::Command::new("emery").version(env!("CARGO_PKG_VERSION")).about(ABOUT);
    let mut router = RouterBuilder::new(command, invoker)
        .completions(
            Completions::new()
                .about("Print a shell-completion script for `<shell>` to stdout")
                .long_about("Print a shell-completion script for `<shell>` to stdout.\n\nPipe into your shell's completion directory (e.g. `emery completions zsh > ~/.zsh/_emery`). Generated via `clap_complete`; the output tracks the live clap surface so every new verb is auto-discovered."),
        );

    macro_rules! route {
        ($path:expr, $args:ty, $operation:ty, $about:literal, $long_about:literal) => {
            router = router.route(
                $path,
                run::<$args, $operation>()
                    .about($about)
                    .long_about($long_about)
                    .project_with(EmeryProjector),
            );
        };
    }

    route!(
        ["init"],
        InitArgs,
        project::init::handlers::Init,
        "Initialize .emery/ in a project",
        "Initialize .emery/ in a project.\n\nPass `<adapter>` (first-party shorthand, package reference, or local component path). A missing `<adapter>` fails typed with `init-adapter-required` (exit 2). Re-running `init` in an already-initialized project changes nothing and exits 0 routing to `emery init --upgrade`."
    );
    route!(
        ["specify"],
        SpecifyArgs,
        specify::Specify,
        "Generate specifications from bound sources (reserved — not yet implemented)",
        "Generate specifications from bound sources.\n\nReserved for the spec generator (ADR-0008); until the walking skeleton lands this verb fails typed with `specify-not-implemented` (exit 1)."
    );
    router.build()
}

macro_rules! convert {
    // The destructuring pattern is exhaustive on purpose: a new clap
    // flag missing from the field list is a compile error.
    ($args:path => $input:path { $($field:ident),* $(,)? }) => {
        impl TryFrom<$args> for $input {
            type Error = error::Error;

            fn try_from(args: $args) -> Result<Self, Self::Error> {
                let $args { $($field),* } = args;
                Ok(Self { $($field),* })
            }
        }
    };
}

convert!(InitArgs => project::init::handlers::InitInput { adapter, name, description, platforms, upgrade });
