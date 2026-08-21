//! The exhaustive route inventory: `init`, the live `specify`
//! generator, and the auto-derived `completions`. Deleted verbs are
//! gone from the grammar — no hidden routes, no aliases.

use clap::Args;
use emery_adapter::Source;
use omnia_guest::Model;
use omnia_guest::api::Provider;
use omnia_guest::api::command::{BuildError, Completions, Router, RouterBuilder, run};
use omnia_guest::api::invoke::Invoker;

use super::{EmeryProjector, Globals};

// One-line application description.
const ABOUT: &str = "Deterministic primitives for spec-driven development";

/// Flags for `emery init`.
#[derive(Debug, Args)]
pub(super) struct InitArgs {
    /// Source adapter identifiers or local component paths, each bound
    /// as a workspace-backed source.
    pub(super) adapters: Vec<String>,
    /// Inline value-backed source binding (`<adapter>=<text>`; repeatable).
    #[arg(long = "value")]
    values: Vec<String>,
    /// Project name.
    #[arg(long)]
    name: Option<String>,
    /// Project description.
    #[arg(long)]
    description: Option<String>,
    /// Re-enter initialization to bump the Emery version pin.
    #[arg(long, conflicts_with_all = ["adapters", "values", "name", "description"])]
    pub(super) upgrade: bool,
}

/// Flags for `emery specify` (none, deliberately).
#[derive(Debug, Args)]
pub(super) struct SpecifyArgs;

impl TryFrom<SpecifyArgs> for emery_engine::specify::SpecifyInput {
    type Error = emery_error::Error;

    fn try_from(args: SpecifyArgs) -> Result<Self, Self::Error> {
        let SpecifyArgs = args;
        Ok(Self)
    }
}

/// Assemble the complete Emery command router.
///
/// # Errors
///
/// Returns a deterministic route or argument conflict.
pub fn router<P>(invoker: Invoker<P>) -> Result<Router<P, Globals>, BuildError>
where
    P: Provider + Model + Source,
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
        emery_engine::init::Init,
        "Initialize .emery/ with source bindings",
        "Initialize .emery/ with source bindings.\n\nPass one or more `<adapter>` values (first-party shorthand, package reference, or local component path) for workspace-backed sources, and `--value <adapter>=<text>` for inline sources. No sources fails typed with `init-source-required` (exit 2). Re-running `init` in an already-initialized project changes nothing and exits 0 routing to `emery init --upgrade`."
    );
    route!(
        ["specify"],
        SpecifyArgs,
        emery_engine::specify::Specify,
        "Generate spec.md and design.md from the bound sources",
        "Generate spec.md and design.md from the bound sources.\n\nExtracts every source binding over the adapter seam, reconciles the typed claims under authority precedence (intent > documentation > behaviour), synthesises the two reviewable documents, and commits them as one generation behind the atomically swapped `current` pointer (ADR-0001). Gaps stay `[unknown]`; disagreement surfaces inline as `[conflict]` / `[divergence]` (ADR-0004). Re-running over identical sources is byte-stable and reports an empty re-mine diff; a changed source names its changed artifacts and spec sections in the success envelope (ADR-0010) — nothing is persisted for the diff."
    );
    router.build()
}

macro_rules! convert {
    // The destructuring pattern is exhaustive on purpose: a new clap
    // flag missing from the field list is a compile error.
    ($args:path => $input:path { $($field:ident),* $(,)? }) => {
        impl TryFrom<$args> for $input {
            type Error = emery_error::Error;

            fn try_from(args: $args) -> Result<Self, Self::Error> {
                let $args { $($field),* } = args;
                Ok(Self { $($field),* })
            }
        }
    };
}

convert!(InitArgs => emery_engine::init::InitInput { adapters, values, name, description, upgrade });
