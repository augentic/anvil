//! Complete command-route inventory.

use clap::Args;
use emery_adapter::Source;
use omnia_guest::api::Provider;
use omnia_guest::api::command::{BuildError, Completions, Router, RouterBuilder, run};
use omnia_guest::api::invoke::Invoker;
use omnia_guest::{BlobStore, Model, StateStore};

use super::{EmeryProjector, Globals};

const ABOUT: &str = "Deterministic primitives for spec-driven development";

/// Arguments for `emery specify`.
#[derive(Debug, Args)]
pub(super) struct SpecifyArgs {
    /// Workspace-backed source adapters or local component paths.
    pub(super) adapters: Vec<String>,
    /// Bind an inline source as `<adapter>=<text>`; repeatable.
    #[arg(long = "value")]
    values: Vec<String>,
    /// Operator-owned binding list; defaults to sources.toml.
    #[arg(long, num_args = 0..=1, default_missing_value = "sources.toml")]
    sources: Option<String>,
}

/// Arguments for `emery show`.
#[derive(Debug, Args)]
pub(super) struct ShowArgs {
    /// Reviewable document of the current generation.
    #[arg(value_enum)]
    document: ShowDocument,
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum ShowDocument {
    /// The behavioural specification document.
    Spec,
    /// The rebuild design document.
    Design,
}

impl TryFrom<ShowArgs> for emery_engine::show::ShowInput {
    type Error = emery_error::Error;

    fn try_from(args: ShowArgs) -> Result<Self, Self::Error> {
        let ShowArgs { document } = args;
        Ok(Self {
            document: match document {
                ShowDocument::Spec => emery_engine::show::Document::Spec,
                ShowDocument::Design => emery_engine::show::Document::Design,
            },
        })
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
        ["specify"],
        SpecifyArgs,
        emery_engine::specify::Specify,
        "Generate spec.md and design.md from the named sources",
        "Generate spec.md and design.md from the named sources.\n\nPass one or more `<adapter>` values (first-party shorthand, package reference, or project-relative local component path) for workspace-backed sources, and `--value <adapter>=<text>` for inline sources — or point at an operator-owned binding list with `--sources [<path>]`; omitting the path explicitly selects `sources.toml`. Mixing the file carrier with argv bindings refuses typed (exit 2). Each run resolves and, for a local component, mirrors its adapters before extracting; nothing about the binding list persists between runs. No sources fails typed with `specify-source-required` (exit 2).\n\nFilesystem inputs are relative to the project preopen `.` and may not escape it. Extraction reconciles the typed claims under authority precedence (intent > documentation > behaviour), synthesises the two reviewable documents, and commits them as one generation behind the atomically swapped `current` pointer (ADR-0001). Gaps stay `[unknown]`; disagreement surfaces inline as `[conflict]` / `[divergence]` (ADR-0004). Re-running over identical sources is byte-stable and reports an empty re-mine diff; a changed source names its changed artifacts and spec sections in the success envelope (ADR-0010) — nothing is persisted for the diff."
    );
    route!(
        ["show"],
        ShowArgs,
        emery_engine::show::Show,
        "Print a reviewable document of the current generation to stdout",
        "Print a reviewable document of the current generation to stdout.\n\n`emery show spec` and `emery show design` render the named document of the generation the `current` pointer names — a verifiable, non-authoritative projection of the store. Text output is the document body alone, so it pipes cleanly; `--format json` wraps it with the generation id. Before any generation is committed the verb fails typed with `spec-not-generated` (exit 1)."
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

convert!(SpecifyArgs => emery_engine::specify::SpecifyInput { adapters, values, sources });
