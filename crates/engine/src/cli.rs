//! The CLI surface: clap grammar, handler dispatch, and the exit contract.

use std::ffi::OsString;
use std::io::{self, Write};
use std::io::{stderr, stdout};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use emery_source::Source;
use omnia_guest::api::{Client, Metadata};
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore};
use serde::Serialize;

use crate::handler::Render;
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

/// Parse and execute one argument vector over `provider`.
pub async fn run<P, I, T>(provider: P, argv: I) -> Response
where
    P: Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static,
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let app = match App::try_parse_from(argv) {
        Ok(app) => app,
        Err(err) => {
            let text = err.render().to_string().into_bytes();
            return if err.use_stderr() {
                Response::failure(text, 2)
            } else {
                Response::success(text)
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

/// CLI output format.
#[derive(Copy, Clone, Debug, Default, clap::ValueEnum, PartialEq, Eq)]
enum Format {
    /// Human-readable text.
    #[default]
    Text,
    /// Pretty-printed JSON.
    Json,
}

/// Buffered command output and process exit status.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Response {
    /// Bytes written to standard output.
    pub stdout: Vec<u8>,
    /// Bytes written to standard error.
    pub stderr: Vec<u8>,
    /// Numeric process exit status.
    pub exit: u8,
}

impl Response {
    /// Create a successful response.
    #[must_use]
    pub fn success(stdout: impl Into<Vec<u8>>) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: Vec::new(),
            exit: 0,
        }
    }

    /// Create a failed response.
    #[must_use]
    pub fn failure(stderr: impl Into<Vec<u8>>, exit: u8) -> Self {
        Self {
            stdout: Vec::new(),
            stderr: stderr.into(),
            exit,
        }
    }

    /// Write both output channels.
    ///
    /// # Errors
    ///
    /// Returns the first output sink error.
    pub fn write(&self) -> io::Result<()> {
        stdout().write_all(&self.stdout)?;
        stderr().write_all(&self.stderr)?;
        Ok(())
    }
}

fn project(format: Format, outcome: Result<impl Render, Error>) -> Response {
    match outcome {
        Ok(body) => Response::success(emit(format, &body)),
        Err(err) => Response::failure(emit(format, &Failure::from(&err)), exit_code(&err)),
    }
}

// Both sinks are in-memory: a plain DTO serializes and a `Vec` never
// refuses a write, so projection has no failure path of its own.
fn emit(format: Format, body: &impl Render) -> Vec<u8> {
    let mut out = Vec::new();
    match format {
        Format::Json => {
            serde_json::to_writer_pretty(&mut out, body).expect("a plain DTO serializes");
            out.push(b'\n');
        }
        Format::Text => body.render(&mut out).expect("a Vec sink never fails"),
    }
    out
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

impl Render for Failure {
    fn render(&self, w: &mut dyn Write) -> io::Result<()> {
        writeln!(w, "error: {}", self.message)?;
        if let Some(hint) = self.hint {
            writeln!(w, "hint: {hint}")?;
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
