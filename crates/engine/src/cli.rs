//! The CLI surface: clap grammar, handler dispatch, and the exit contract.

use std::ffi::OsString;
use std::fmt;
use std::io::{self, Write};

use clap::error::ErrorKind;
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use emery_source::Source;
use omnia_guest::api::{Client, Metadata};
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore, server_error};
use serde::Serialize;

use crate::handler::Render;
use crate::show::ShowInput;
use crate::specify::SpecifyInput;

/// The Emery command grammar bound over one provider.
pub struct Cli<P> {
    client: Client<P>,
    inventory: Vec<RouteInfo>,
}

impl<P: Send + Sync + 'static> fmt::Debug for Cli<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cli").field("inventory", &self.inventory).finish_non_exhaustive()
    }
}

const ABOUT: &str = "Deterministic primitives for spec-driven development";

/// Buffered command output and process exit status.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandResponse {
    /// Bytes written to standard output.
    pub stdout: Vec<u8>,
    /// Bytes written to standard error.
    pub stderr: Vec<u8>,
    /// Numeric process exit status.
    pub exit: u8,
}

impl CommandResponse {
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
    pub fn write_to(&self, stdout: &mut impl Write, stderr: &mut impl Write) -> io::Result<()> {
        stdout.write_all(&self.stdout)?;
        stderr.write_all(&self.stderr)?;
        Ok(())
    }
}

/// A command route selector.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Selector {
    path: Vec<String>,
}

impl Selector {
    /// Return the nested command path.
    #[must_use]
    pub fn path(&self) -> &[String] {
        &self.path
    }
}

/// Read-only metadata for one command binding.
#[derive(Clone, Debug)]
pub struct RouteInfo {
    selector: Selector,
}

impl RouteInfo {
    /// Return the command selector.
    #[must_use]
    pub const fn selector(&self) -> &Selector {
        &self.selector
    }
}

/// Global command arguments.
#[derive(Clone, Copy, Debug, Args)]
struct Globals {
    /// Select the output format.
    #[arg(long, env = "EMERY_FORMAT", default_value = "text", global = true)]
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

#[derive(Debug, Parser)]
#[command(
    name = "emery",
    version = env!("CARGO_PKG_VERSION"),
    about = ABOUT,
    disable_help_subcommand = true,
    subcommand_required = true,
    arg_required_else_help = true
)]
struct App {
    #[command(flatten)]
    globals: Globals,
    #[command(subcommand)]
    verb: Verb,
}

#[derive(Debug, Subcommand)]
enum Verb {
    /// Generate spec.md and design.md from the named sources
    #[command(long_about = SPECIFY_LONG)]
    Specify(SpecifyInput),
    /// Print a reviewable document of the current generation to stdout
    #[command(long_about = SHOW_LONG)]
    Show(ShowInput),
    /// Print a shell-completion script for `<shell>` to stdout
    #[command(long_about = COMPLETIONS_LONG)]
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

const SPECIFY_LONG: &str = "Generate spec.md and design.md from the named sources.\n\nPass one or more `<adapter>` values (first-party shorthand, package reference, or project-relative local component path) for workspace-backed sources, and `--description <adapter>=<text>` for inline sources — or point at an operator-owned config with `--config [<path>]`; omitting the path explicitly selects `emery.toml`. A run naming no bindings at all discovers the project-root `emery.toml` as a fallback — never merged with argv bindings. Mixing the file carrier with argv bindings refuses typed (exit 1). Each run resolves its adapters before extracting; a local component loads through the deployment loader, read fresh on every run — a deleted source file refuses typed — with the binding's optional `digest` pin verified before validation, and each resolved digest reported in the success envelope (commit one as its binding's pin to make the load reproducible). Nothing about the binding list persists between runs. No sources — and no discoverable `emery.toml` — fails typed with `specify-source-required` (exit 1).\n\nFilesystem inputs are relative to the project preopen `.` and may not escape it. Extraction reconciles the typed claims under authority precedence (intent > documentation > behaviour), synthesises the two reviewable documents, and commits them as one generation behind the atomically swapped `current` pointer. Gaps stay `[unknown]`; disagreement surfaces inline as `[conflict]` / `[divergence]`. Re-running over identical sources is byte-stable and reports an empty re-mine diff; a changed source names its changed artifacts and spec sections in the success envelope — nothing is persisted for the diff.";
const SHOW_LONG: &str = "Print a reviewable document of the current generation to stdout.\n\n`emery show spec` and `emery show design` render the named document of the generation the `current` pointer names — a verifiable, non-authoritative projection of the store. Text output is the document body alone, so it pipes cleanly; `--format json` wraps it with the generation id. Before any generation is committed the verb fails typed with `spec-not-generated` (exit 2).";
const COMPLETIONS_LONG: &str = "Print a shell-completion script for `<shell>` to stdout.\n\nPipe into your shell's completion directory (e.g. `emery completions zsh > ~/.zsh/_emery`). The output tracks the live clap surface so every new verb is auto-discovered.";

/// Builds the Emery command grammar over `provider`.
pub fn router<P>(provider: P) -> Cli<P>
where
    P: Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static,
{
    Cli {
        client: Client::new("emery", provider),
        inventory: inventory(),
    }
}

fn inventory() -> Vec<RouteInfo> {
    let mut routes: Vec<RouteInfo> = App::command()
        .get_subcommands()
        .filter(|command| !command.is_hide_set())
        .map(|command| RouteInfo {
            selector: Selector {
                path: vec![command.get_name().to_string()],
            },
        })
        .collect();
    routes.sort_by(|left, right| left.selector.path.cmp(&right.selector.path));
    routes
}

impl<P> Cli<P>
where
    P: Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static,
{
    /// Return the deterministic route inventory.
    #[must_use]
    pub fn inventory(&self) -> &[RouteInfo] {
        &self.inventory
    }

    /// Parse and execute one argument vector.
    pub async fn execute<I, T>(&self, argv: I) -> CommandResponse
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        let mut argv: Vec<OsString> = argv.into_iter().map(Into::into).collect();
        let name = OsString::from("emery");
        if argv.is_empty() {
            argv.push(name);
        } else {
            argv[0] = name;
        }
        let parsed = match App::try_parse_from(&argv) {
            Ok(app) => app,
            Err(error) => return clap_error(&error),
        };
        match parsed.verb {
            Verb::Completions { shell } => completion(shell),
            Verb::Specify(input) => {
                project(self.client.call(input, &Metadata::default()).await, parsed.globals.format)
            }
            Verb::Show(input) => {
                project(self.client.call(input, &Metadata::default()).await, parsed.globals.format)
            }
        }
    }
}

fn project<T: Render + Serialize>(result: Result<T, Error>, format: Format) -> CommandResponse {
    match result {
        Ok(output) => {
            let mut stdout = Vec::new();
            match emit(&mut stdout, format, &output, |writer, value| value.render(writer)) {
                Ok(()) => CommandResponse::success(stdout),
                Err(fallback) => {
                    CommandResponse::failure(format!("error: {fallback}\n").into_bytes(), 3)
                }
            }
        }
        Err(error) => failure(format, &error),
    }
}

fn clap_error(error: &clap::Error) -> CommandResponse {
    let rendered = error.render().to_string().into_bytes();
    match error.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => CommandResponse::success(rendered),
        _ => CommandResponse::failure(rendered, 2),
    }
}

fn completion(shell: Shell) -> CommandResponse {
    let mut command = App::command();
    let name = command.get_name().to_owned();
    let mut output = Vec::new();
    clap_complete::generate(shell, &mut command, name, &mut output);
    CommandResponse::success(output)
}

/// Writes `payload` in the requested format.
///
/// # Errors
///
/// Returns serialization or I/O failures.
fn emit<T: Serialize>(
    writer: &mut dyn Write, format: Format, payload: &T,
    render_text: impl FnOnce(&mut dyn Write, &T) -> io::Result<()>,
) -> Result<(), Error> {
    match format {
        Format::Json => {
            serde_json::to_writer_pretty(&mut *writer, payload)
                .map_err(|err| server_error!("failed to serialize JSON response: {err}",))?;
            writeln!(writer).map_err(|err| server_error!(err))
        }
        Format::Text => render_text(writer, payload).map_err(|err| server_error!(err)),
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

/// Serialized command failure.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct ErrorBody {
    error: String,
    message: String,
    exit_code: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'static str>,
}

impl From<&Error> for ErrorBody {
    fn from(err: &Error) -> Self {
        Self {
            error: err.code(),
            message: err.description(),
            exit_code: exit_code(err),
            hint: hint(&err.code()),
        }
    }
}

/// Renders command-failure bytes and their exit code.
///
/// Rendering failures become a plain `ServerError` line (exit 3).
fn failure(format: Format, error: &Error) -> CommandResponse {
    let body = ErrorBody::from(error);
    let mut stderr = Vec::new();
    match emit(&mut stderr, format, &body, write_error_text) {
        Ok(()) => CommandResponse::failure(stderr, exit_code(error)),
        Err(fallback) => CommandResponse::failure(format!("error: {fallback}\n").into_bytes(), 3),
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

fn write_error_text(writer: &mut dyn Write, body: &ErrorBody) -> io::Result<()> {
    let (red, reset) = error_style();
    writeln!(writer, "{red}error: {}{reset}", body.message)?;
    if let Some(hint) = body.hint {
        writeln!(writer, "hint: {hint}")?;
    }
    Ok(())
}

// `NO_COLOR`, missing `TERM`, and `TERM=dumb` disable ANSI styling.
// Wasm has no terminal probe, so only those environment guards apply.
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
    io::stderr().is_terminal()
}

#[cfg(target_arch = "wasm32")]
const fn stderr_terminal() -> bool {
    true
}
