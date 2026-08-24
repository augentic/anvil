//! The CLI surface: command grammar, output projection, and the exit
//! contract. `exit_code` is the failure-code authority.

use std::borrow::Cow;
use std::convert::Infallible;
use std::io::Write;

use clap::Args;
use emery_adapter::Source;
use omnia_guest::api::Provider;
use omnia_guest::api::command::{
    BuildError, CommandResponse, Completions, Outcome, Projector, Router, RouterBuilder, run,
};
use omnia_guest::api::invoke::Invoker;
use omnia_guest::{BlobStore, Model, StateStore};
use serde::Serialize;

use crate::handler::{Error, Render};
use crate::show::{Show, ShowInput};
use crate::specify::{Specify, SpecifyInput};

/// The Emery command router bound over one provider.
pub type Cli<P> = Router<P, Globals>;

const ABOUT: &str = "Deterministic primitives for spec-driven development";

/// Global command arguments.
#[derive(Clone, Copy, Debug, Args)]
pub struct Globals {
    /// Select the output format.
    #[arg(long, env = "EMERY_FORMAT", default_value = "text")]
    format: Format,
}

/// CLI output format.
#[derive(Copy, Clone, Debug, Default, clap::ValueEnum, PartialEq, Eq)]
enum Format {
    /// Human-readable text.
    #[default]
    Text,
    /// Pretty-printed JSON.
    Json,
}

/// Builds the Emery command router.
///
/// Each operation input doubles as its clap surface, so route
/// decoding is infallible by construction.
///
/// # Errors
///
/// Returns route or argument conflicts.
pub fn router<P>(invoker: Invoker<P>) -> Result<Cli<P>, BuildError>
where
    P: Provider + Model + Source + StateStore + BlobStore,
{
    let command = clap::Command::new("emery").version(env!("CARGO_PKG_VERSION")).about(ABOUT);
    RouterBuilder::new(command, invoker)
        .completions(
            Completions::new()
                .about("Print a shell-completion script for `<shell>` to stdout")
                .long_about("Print a shell-completion script for `<shell>` to stdout.\n\nPipe into your shell's completion directory (e.g. `emery completions zsh > ~/.zsh/_emery`). The output tracks the live clap surface so every new verb is auto-discovered."),
        )
        .route(
            ["specify"],
            run::<SpecifyInput, Specify>()
                .about("Generate spec.md and design.md from the named sources")
                .long_about("Generate spec.md and design.md from the named sources.\n\nPass one or more `<adapter>` values (first-party shorthand, package reference, or project-relative local component path) for workspace-backed sources, and `--value <adapter>=<text>` for inline sources — or point at an operator-owned binding list with `--sources [<path>]`; omitting the path explicitly selects `sources.toml`. Mixing the file carrier with argv bindings refuses typed (exit 2). Each run resolves and, for a local component, mirrors its adapters before extracting; nothing about the binding list persists between runs. No sources fails typed with `specify-source-required` (exit 2).\n\nFilesystem inputs are relative to the project preopen `.` and may not escape it. Extraction reconciles the typed claims under authority precedence (intent > documentation > behaviour), synthesises the two reviewable documents, and commits them as one generation behind the atomically swapped `current` pointer (ADR-0001). Gaps stay `[unknown]`; disagreement surfaces inline as `[conflict]` / `[divergence]` (ADR-0004). Re-running over identical sources is byte-stable and reports an empty re-mine diff; a changed source names its changed artifacts and spec sections in the success envelope (ADR-0010) — nothing is persisted for the diff.")
                .project_with(EmeryProjector),
        )
        .route(
            ["show"],
            run::<ShowInput, Show>()
                .about("Print a reviewable document of the current generation to stdout")
                .long_about("Print a reviewable document of the current generation to stdout.\n\n`emery show spec` and `emery show design` render the named document of the generation the `current` pointer names — a verifiable, non-authoritative projection of the store. Text output is the document body alone, so it pipes cleanly; `--format json` wraps it with the generation id. Before any generation is committed the verb fails typed with `spec-not-generated` (exit 1).")
                .project_with(EmeryProjector),
        )
        .build()
}

/// Projects Emery outcomes into command responses.
#[derive(Clone, Copy, Debug, Default)]
struct EmeryProjector;

impl<T> Projector<T, Error, Infallible, Globals> for EmeryProjector
where
    T: Render + Serialize + Send + 'static,
{
    type Error = Error;

    fn project(
        &self, outcome: Outcome<T, Error, Infallible>, globals: &Globals,
    ) -> Result<CommandResponse, Self::Error> {
        match outcome {
            Outcome::Output(output) => {
                let mut stdout = Vec::new();
                emit(&mut stdout, globals.format, &output, |w, v| v.render(w))?;
                Ok(CommandResponse::success(stdout))
            }
            Outcome::Operation(error) => Ok(failure(globals.format, &error)),
            Outcome::Decode(never) => match never {},
        }
    }

    fn project_failure(&self, error: Self::Error, globals: &Globals) -> CommandResponse {
        failure(globals.format, &error)
    }
}

/// Writes `payload` in the requested format.
///
/// # Errors
///
/// Returns serialization or I/O failures.
fn emit<T: Serialize>(
    writer: &mut dyn Write, format: Format, payload: &T,
    render_text: impl FnOnce(&mut dyn Write, &T) -> std::io::Result<()>,
) -> Result<(), Error> {
    match format {
        Format::Json => {
            serde_json::to_writer_pretty(&mut *writer, payload).map_err(|err| Error::Diag {
                code: "json-serialize-failed",
                detail: format!("failed to serialize JSON response: {err}"),
            })?;
            writeln!(writer).map_err(Error::Io)
        }
        Format::Text => render_text(writer, payload).map_err(Error::Io),
    }
}

/// The failure-code authority: the fixed four-slot CLI exit contract.
const fn exit_code(error: &Error) -> u8 {
    match error {
        Error::AdapterCliTooOld { .. } => 3,
        Error::Validation { .. } | Error::Argument { .. } => 2,
        _ => 1,
    }
}

/// Serialized command failure.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct ErrorBody {
    error: Cow<'static, str>,
    message: String,
    exit_code: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'static str>,
}

impl From<&Error> for ErrorBody {
    fn from(err: &Error) -> Self {
        Self {
            error: err.variant_str(),
            message: err.to_string(),
            exit_code: exit_code(err),
            hint: err.hint(),
        }
    }
}

/// Renders command-failure bytes and their exit code.
///
/// Rendering failures become a plain exit-1 line.
fn failure(format: Format, error: &Error) -> CommandResponse {
    let body = ErrorBody::from(error);
    let mut stderr = Vec::new();
    match emit(&mut stderr, format, &body, write_error_text) {
        Ok(()) => CommandResponse::failure(stderr, exit_code(error)),
        Err(fallback) => CommandResponse::failure(format!("error: {fallback}\n").into_bytes(), 1),
    }
}

fn write_error_text(w: &mut dyn Write, body: &ErrorBody) -> std::io::Result<()> {
    let (red, reset) = error_style();
    writeln!(w, "{red}error: {}{reset}", body.message)?;
    if let Some(hint) = body.hint {
        writeln!(w, "hint: {hint}")?;
    }
    Ok(())
}

// `NO_COLOR`, missing `TERM`, and `TERM=dumb` disable ANSI styling.
// Wasm has no terminal probe, so only those environment guards apply.
#[expect(
    clippy::disallowed_methods,
    reason = "the guest is the CLI (wasi:cli/run); NO_COLOR/TERM are the terminal \
              colour convention, not app configuration"
)]
fn error_style() -> (&'static str, &'static str) {
    let opted_out = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty())
        || !std::env::var_os("TERM").is_some_and(|term| !term.is_empty() && term != "dumb");
    if opted_out || !stderr_terminal() {
        return ("", "");
    }
    ("\x1b[1;31m", "\x1b[0m")
}

#[cfg(not(target_arch = "wasm32"))]
fn stderr_terminal() -> bool {
    use std::io::IsTerminal as _;
    std::io::stderr().is_terminal()
}

#[cfg(target_arch = "wasm32")]
const fn stderr_terminal() -> bool {
    true
}
