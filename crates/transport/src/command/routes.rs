//! Complete command-route inventory.

use clap::Args;
use emery_adapter::Source;
use omnia_guest::api::Provider;
use omnia_guest::api::command::{BuildError, Completions, Router, RouterBuilder, run};
use omnia_guest::api::invoke::Invoker;
use omnia_guest::{BlobStore, Model, StateStore};

use super::{EmeryProjector, Globals};

const ABOUT: &str = "Deterministic primitives for spec-driven development";

/// Arguments for `emery init`.
#[derive(Debug, Args)]
pub(super) struct InitArgs {
    /// Workspace-backed source adapters or local component paths.
    pub(super) adapters: Vec<String>,
    /// Bind an inline source as `<adapter>=<text>`; repeatable.
    #[arg(long = "value")]
    values: Vec<String>,
    /// Project name.
    #[arg(long)]
    name: Option<String>,
    /// Project description.
    #[arg(long)]
    description: Option<String>,
    /// Upgrade the Emery version pin.
    #[arg(long, conflicts_with_all = ["adapters", "values", "name", "description"])]
    pub(super) upgrade: bool,
}

/// Arguments for `emery specify`.
#[derive(Debug, Args)]
pub(super) struct SpecifyArgs;

impl TryFrom<SpecifyArgs> for emery_engine::specify::SpecifyInput {
    type Error = emery_error::Error;

    fn try_from(args: SpecifyArgs) -> Result<Self, Self::Error> {
        let SpecifyArgs = args;
        Ok(Self)
    }
}

/// Builds the Emery command router.
///
/// # Errors
///
/// Returns route or argument conflicts.
pub fn router<P>(invoker: Invoker<P>) -> Result<Router<P, Globals>, BuildError>
where
    P: Provider + Model + Source + StateStore + BlobStore,
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
    // Exhaustive destructuring makes an unmapped clap field fail compilation.
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
