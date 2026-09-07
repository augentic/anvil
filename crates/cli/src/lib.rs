//! The `emery` command line
//!
//! The operator-facing surface of Emery: the `specify`, `show`, and
//! `completions` verbs, their help text, and the rules that turn a parsed
//! command into an engine operation and an engine result into terminal
//! output and an exit code.
//!
//! The engine knows nothing about arguments, text, or exit codes. Keeping
//! that vocabulary here means the same operations can be driven by another
//! transport, and the command grammar can change without touching the
//! engine.

mod bindings;
mod output;
mod text;

use std::ffi::OsString;
use std::fmt;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use emery_engine::Provider;
use emery_engine::show::{Document, Show};
use emery_engine::specify::Specify;
use omnia_guest::Error;
use omnia_guest::api::{Client, Metadata};
use output::Format;
pub use output::Response;
use serde::Serialize;
use text::Text;

const ABOUT: &str = "Deterministic primitives for spec-driven development";
const SPECIFY_DESC: &str = "Generate spec.md and design.md from source adapters.\n\n\
    Name one or more adapters, use `--description <adapter>=<text>` for inline input, \
    or use `--config [<path>]` (default: `emery.toml`). With no bindings, Emery looks \
    for `emery.toml` in the project root. Config and command-line bindings cannot be \
    combined.\n\n\
    Adapter paths are project-relative. Each run reloads adapters, verifies optional \
    digest pins, reconciles their claims, and atomically commits a new revision.";
const SHOW_DESC: &str = "Print a document from the current revision.\n\n\
    Text output contains only the document body. `--format json` also includes the \
    revision id.";
const COMPLETIONS_DESC: &str = "Generate shell completions.\n\n\
    Pipe into your shell's completion directory. Example: \
    `emery completions zsh > ~/.zsh/_emery`";

// The program name: the clap surface, the completions target, and the
// `Client` owner are one spelling.
const NAME: &str = "emery";

// Clap's usage-error status; help and version print to stdout and exit 0.
const EXIT_USAGE: u8 = 2;

/// Parse and execute one argument vector over `provider`, buffering both channels.
pub async fn run<P, I, T>(provider: P, argv: I) -> Response
where
    P: Provider,
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match decode(argv) {
        Ok(app) => dispatch(app, &Client::new(NAME, provider)).await,
        Err(response) => response,
    }
}

// `bin_name` pins usage text to `emery`: Omnia forwards the routed id as
// argv[0], and clap only reads argv[0] when `bin_name` is unset.
#[derive(Debug, Parser)]
#[command(
    name = NAME,
    bin_name = NAME,
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
    Specify(SpecifyArgs),
    /// Print a reviewable document of the current revision to stdout
    #[command(long_about = SHOW_DESC)]
    Show(ShowArgs),
    /// Print a shell-completion script for `<shell>` to stdout
    #[command(long_about = COMPLETIONS_DESC)]
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

// The `specify` grammar; field docs are its `--help` text. Decoding
// builds the engine input by exhaustive struct literal, so an engine
// field the grammar does not carry fails to compile here.
#[derive(Debug, clap::Args)]
struct SpecifyArgs {
    /// Workspace-backed source adapters or local component paths.
    adapters: Vec<String>,
    /// Bind an inline source as `<adapter>=<text>`; repeatable.
    #[arg(long = "description", short = 'd')]
    descriptions: Vec<String>,
    /// Operator-owned config; the omitted value selects emery.toml.
    #[arg(long, short = 'c', num_args = 0..=1, default_missing_value = bindings::CONFIG_FILE)]
    config: Option<String>,
}

impl SpecifyArgs {
    fn decode(self) -> Result<Specify, Error> {
        let Self {
            adapters,
            descriptions,
            config,
        } = self;
        let bindings = bindings::decode(&adapters, &descriptions, config.as_deref())?;
        Ok(Specify { bindings })
    }
}

// The `show` grammar; field docs are its `--help` text.
#[derive(Debug, clap::Args)]
struct ShowArgs {
    /// Reviewable document of the current revision.
    #[arg(value_enum)]
    document: DocumentArg,
}

impl ShowArgs {
    fn decode(self) -> Show {
        let Self { document } = self;
        Show {
            document: document.into(),
        }
    }
}

// The closed document vocabulary as clap values; the exhaustive
// conversion pins it to the engine's `Document`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum DocumentArg {
    /// The behavioural specification document.
    Spec,
    /// The rebuild design document.
    Design,
}

impl From<DocumentArg> for Document {
    fn from(document: DocumentArg) -> Self {
        match document {
            DocumentArg::Spec => Self::Spec,
            DocumentArg::Design => Self::Design,
        }
    }
}

// The failure envelope.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Failure {
    error: String,
    message: String,
    exit_code: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'static str>,
}

impl Failure {
    fn new(err: &Error, exit_code: u8) -> Self {
        let error = err.code();
        Self {
            hint: hint(&error),
            error,
            message: err.description(),
            exit_code,
        }
    }
}

impl Text for Failure {
    fn text(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        writeln!(out, "error[{}]: {}", self.error, self.message)?;
        if let Some(hint) = self.hint {
            writeln!(out, "hint: {hint}")?;
        }
        Ok(())
    }
}

// The decode leg: clap parses argv, and its own outcomes — usage errors,
// help, version — are already complete responses.
fn decode<I, T>(argv: I) -> Result<App, Response>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    App::try_parse_from(argv).map_err(|err| {
        let rendered = err.render().to_string();
        if err.use_stderr() {
            Response::failure(rendered, EXIT_USAGE)
        } else {
            Response::success(rendered)
        }
    })
}

// The call-and-encode leg: each verb decodes into its engine input, runs
// through the client, and projects. `completions` never reaches a handler.
async fn dispatch<P: Provider>(app: App, client: &Client<P>) -> Response {
    // A wasm32 guest has no clock or entropy to mint a request id from.
    let metadata = Metadata::default();
    match app.verb {
        Verb::Completions { shell } => {
            let mut out = Vec::new();
            clap_complete::generate(shell, &mut App::command(), NAME, &mut out);
            Response::success(out)
        }
        Verb::Specify(grammar) => match grammar.decode() {
            Ok(input) => project(app.format, client.call(input, &metadata).await),
            Err(error) => refuse(app.format, &error),
        },
        Verb::Show(grammar) => project(app.format, client.call(grammar.decode(), &metadata).await),
    }
}

// The command projector: the success body rides stdout, the failure
// envelope rides stderr with its exit status.
fn project<T: Serialize + Text>(format: Format, outcome: Result<T, Error>) -> Response {
    match outcome {
        Ok(body) => Response::success(format.encode(&body)),
        Err(error) => refuse(format, &error),
    }
}

fn refuse(format: Format, error: &Error) -> Response {
    let exit = exit_code(error);
    Response::failure(format.encode(&Failure::new(error, exit)), exit)
}

// The failure-code authority: the Omnia 1:1 exit map.
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
        "unsupported-version" => Some(
            "update emery: `brew upgrade emery`, or `cargo install --git https://github.com/augentic/emery --locked`",
        ),
        "specify-source-required" => Some(
            "pass one or more adapters to `emery specify`, or add an `emery.toml` at the project root",
        ),
        "spec-not-generated" => {
            Some("run `emery specify <adapter>...` to commit a revision, then re-run show")
        }
        "refused" => Some(
            "the loader refused the component; the message above names why (digest, export, or location)",
        ),
        "unavailable" => Some(
            "the registry could not supply the package: check the network, the exact version, and any `registry` override",
        ),
        _ => None,
    }
}
