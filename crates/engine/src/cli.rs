//! The CLI surface: clap grammar, handler dispatch, and the exit contract.

mod output;

use std::ffi::OsString;
use std::fmt::{self, Display};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use emery_source::Source;
use omnia_guest::api::{Client, Metadata};
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore};
use output::Format;
pub use output::Response;
use serde::Serialize;

use crate::show::ShowInput;
use crate::specify::SpecifyInput;

const ABOUT: &str = "Deterministic primitives for spec-driven development";
const SPECIFY_DESC: &str = "Generate spec.md and design.md from source adapters.\n\n\
    Name one or more adapters, use `--description <adapter>=<text>` for inline input, \
    or use `--config [<path>]` (default: `emery.toml`). With no bindings, Emery looks \
    for `emery.toml` in the project root. Config and command-line bindings cannot be \
    combined.\n\n\
    Adapter paths are project-relative. Each run reloads adapters, verifies optional \
    digest pins, reconciles their claims, and atomically commits a new generation.";
const SHOW_DESC: &str = "Print a document from the current generation.\n\n\
    Text output contains only the document body. `--format json` also includes the \
    generation id.";
const COMPLETIONS_DESC: &str = "Generate shell completions.\n\n\
    Pipe into your shell's completion directory. Example: \
    `emery completions zsh > ~/.zsh/_emery`";

// Clap's usage-error status; help and version print to stdout and exit 0.
const EXIT_USAGE: u8 = 2;

/// Parse and execute one argument vector over `provider`, buffering both channels.
pub async fn run<P, I, T>(provider: P, argv: I) -> Response
where
    P: Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static,
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let app = match App::try_parse_from(argv) {
        Ok(app) => app,
        Err(err) => {
            let rendered = err.render().to_string();
            return if err.use_stderr() {
                Response::failure(rendered, EXIT_USAGE)
            } else {
                Response::success(rendered)
            };
        }
    };

    let client = Client::new("emery", provider);
    let metadata = Metadata::default();

    match app.verb {
        Verb::Completions { shell } => {
            let mut out = Vec::new();
            clap_complete::generate(shell, &mut App::command(), "emery", &mut out);
            Response::success(out)
        }
        Verb::Specify(input) => project(app.format, client.call(input, &metadata).await),
        Verb::Show(input) => project(app.format, client.call(input, &metadata).await),
    }
}

// `bin_name` pins usage text to `emery`: Omnia forwards the routed id as
// argv[0], and clap only reads argv[0] when `bin_name` is unset.
#[derive(Debug, Parser)]
#[command(
    name = "emery",
    bin_name = "emery",
    version = env!("CARGO_PKG_VERSION"),
    about = ABOUT,
    disable_help_subcommand = true,
    subcommand_required = true,
    arg_required_else_help = true
)]
struct App {
    #[command(subcommand)]
    verb: Verb,

    /// Select the output format.
    #[arg(long, env = "EMERY_FORMAT", default_value = "text", global = true)]
    format: Format,
}

#[derive(Debug, Subcommand)]
enum Verb {
    /// Generate spec.md and design.md from the named sources
    #[command(long_about = SPECIFY_DESC)]
    Specify(SpecifyInput),
    /// Print a reviewable document of the current generation to stdout
    #[command(long_about = SHOW_DESC)]
    Show(ShowInput),
    /// Print a shell-completion script for `<shell>` to stdout
    #[command(long_about = COMPLETIONS_DESC)]
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

// The command projector: the success body rides stdout, the failure
// envelope rides stderr with its exit status.
fn project<T: Serialize + Display>(format: Format, outcome: Result<T, Error>) -> Response {
    match outcome {
        Ok(body) => Response::success(format.encode(&body)),
        Err(error) => {
            let failure = Failure::from(&error);
            Response::failure(format.encode(&failure), failure.exit_code)
        }
    }
}

/// The failure envelope.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Failure {
    error: String,
    message: String,
    exit_code: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'static str>,
}

impl From<&Error> for Failure {
    fn from(err: &Error) -> Self {
        let error = err.code();
        Self {
            hint: hint(&error),
            error,
            message: err.description(),
            exit_code: exit_code(err),
        }
    }
}

impl Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "error[{}]: {}", self.error, self.message)?;
        if let Some(hint) = self.hint {
            writeln!(f, "hint: {hint}")?;
        }
        Ok(())
    }
}

/// The failure-code authority: the Omnia 1:1 exit map.
const fn exit_code(error: &Error) -> u8 {
    match error {
        Error::BadRequest { .. } => 1,
        Error::NotFound { .. } => 2,
        Error::ServerError { .. } => 3,
        Error::BadGateway { .. } => 4,
    }
}

fn hint(code: &str) -> Option<&'static str> {
    match code {
        "adapter-cli-too-old" => Some(
            "update the installed binary through its install channel: `brew upgrade emery`, or `cargo install --git https://github.com/augentic/emery --locked`",
        ),
        "specify-source-required" => Some(
            "`emery specify <adapter>...` generates the spec over the sources named on the invocation; a bindingless run reads the project-root `emery.toml` when present — there is no other persisted binding list",
        ),
        "spec-not-generated" => {
            Some("run `emery specify <adapter>...` to commit a generation, then re-run show")
        }
        "refused" => Some(
            "the loader refused the request: a mismatched or malformed `digest` pin, an invalid component, a missing source-seam export, or a location kind this deployment does not serve; the message names which",
        ),
        "unavailable" => Some(
            "the deployment's acquirer could not produce the package: check the network, that the exact version exists at the registry, and the binding's `registry` override (the default endpoint is compiled into the binary)",
        ),
        _ => None,
    }
}
