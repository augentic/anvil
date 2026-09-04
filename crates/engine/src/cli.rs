//! The CLI surface: clap grammar, handler dispatch, and the exit contract.

use std::ffi::OsString;
use std::fmt;
use std::io::{self, Write};

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use emery_source::Source;
use omnia_guest::api::{Client, Handler, Metadata};
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore, server_error};

use crate::handler::Render;
use crate::show::ShowInput;
use crate::specify::SpecifyInput;

const ABOUT: &str = "Deterministic primitives for spec-driven development";
const SPECIFY_DESC: &str = "Generate spec.md and design.md from the named sources.\n\nPass one or more `<adapter>` values (first-party shorthand, package reference, or project-relative local component path) for workspace-backed sources, and `--description <adapter>=<text>` for inline sources — or point at an operator-owned config with `--config [<path>]`; omitting the path explicitly selects `emery.toml`. A run naming no bindings at all discovers the project-root `emery.toml` as a fallback — never merged with argv bindings. Mixing the file carrier with argv bindings refuses typed (exit 1). Each run resolves its adapters before extracting; a local component loads through the deployment loader, read fresh on every run — a deleted source file refuses typed — with the binding's optional `digest` pin verified before validation, and each resolved digest reported in the success envelope (commit one as its binding's pin to make the load reproducible). Nothing about the binding list persists between runs. No sources — and no discoverable `emery.toml` — fails typed with `specify-source-required` (exit 1).\n\nFilesystem inputs are relative to the project preopen `.` and may not escape it. Extraction reconciles the typed claims under authority precedence (intent > documentation > behaviour), synthesises the two reviewable documents, and commits them as one generation behind the atomically swapped `current` pointer. Gaps stay `[unknown]`; disagreement surfaces inline as `[conflict]` / `[divergence]`. Re-running over identical sources is byte-stable and reports an empty re-mine diff; a changed source names its changed artifacts and spec sections in the success envelope — nothing is persisted for the diff.";
const SHOW_DESC: &str = "Print a reviewable document of the current generation to stdout.\n\n`emery show spec` and `emery show design` render the named document of the generation the `current` pointer names — a verifiable, non-authoritative projection of the store. Text output is the document body alone, so it pipes cleanly; `--format json` wraps it with the generation id. Before any generation is committed the verb fails typed with `spec-not-generated` (exit 2).";
const COMPLETIONS_DESC: &str = "Print a shell-completion script for `<shell>` to stdout.\n\nPipe into your shell's completion directory (e.g. `emery completions zsh > ~/.zsh/_emery`). The output tracks the live clap surface so every new verb is auto-discovered.";

/// The Emery command grammar bound over one provider.
pub struct Cli<P> {
    client: Client<P>,
}

impl<P> fmt::Debug for Cli<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cli").finish_non_exhaustive()
    }
}

impl<P> Cli<P>
where
    P: Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static,
{
    /// Create a new Emery command grammar bound over `provider`.
    pub fn new(provider: P) -> Self {
        Self {
            client: Client::new("emery", provider),
        }
    }

    /// Parse and execute one argument vector.
    pub async fn run<I, T>(&self, argv: I) -> Response
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
            Err(err) => {
                let text = err.render().to_string().into_bytes();
                return match err.kind() {
                    ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => Response::success(text),
                    _ => Response::failure(text, 2),
                };
            }
        };
        match parsed.verb {
            Verb::Completions { shell } => {
                let mut command = App::command();
                let name = command.get_name().to_owned();
                let mut output = Vec::new();
                clap_complete::generate(shell, &mut command, name, &mut output);
                Response::success(output)
            }
            Verb::Specify(input) => self.dispatch(input, parsed.format).await,
            Verb::Show(input) => self.dispatch(input, parsed.format).await,
        }
    }

    async fn dispatch<I>(&self, input: I, format: Format) -> Response
    where
        I: Handler<P, Error = Error>,
        I::Output: Render,
    {
        match self.client.call(input, &Metadata::default()).await {
            Ok(output) => {
                let mut buf = Vec::new();
                match emit(&mut buf, format, &output) {
                    Ok(()) => Response::success(buf),
                    Err(fallback) => {
                        Response::failure(format!("error: {fallback}\n").into_bytes(), 3)
                    }
                }
            }
            Err(err) => {
                let exit = exit_code(&err);
                let error = err.code();
                let mut buf = Vec::new();
                let wrote = match format {
                    Format::Json => {
                        let mut body = serde_json::json!({
                            "error": error.as_str(),
                            "message": err.description(),
                            "exit-code": exit,
                        });
                        if let Some(hint) = hint(&error) {
                            body["hint"] = hint.into();
                        }
                        serde_json::to_writer_pretty(&mut buf, &body)
                            .map_err(|err| {
                                server_error!("failed to serialize JSON response: {err}",)
                            })
                            .and_then(|()| writeln!(&mut buf).map_err(|err| server_error!(err)))
                    }
                    Format::Text => {
                        let (red, reset) = error_style();
                        writeln!(&mut buf, "{red}error: {}{reset}", err.description())
                            .and_then(|()| {
                                hint(&error)
                                    .map_or(Ok(()), |hint| writeln!(&mut buf, "hint: {hint}"))
                            })
                            .map_err(|err| server_error!(err))
                    }
                };
                match wrote {
                    Ok(()) => Response::failure(buf, exit),
                    Err(fallback) => {
                        Response::failure(format!("error: {fallback}\n").into_bytes(), 3)
                    }
                }
            }
        }
    }
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
    /// Select the output format.
    #[arg(long, env = "EMERY_FORMAT", default_value = "text", global = true)]
    format: Format,
    #[command(subcommand)]
    verb: Verb,
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
    pub fn write_to(&self, stdout: &mut impl Write, stderr: &mut impl Write) -> io::Result<()> {
        stdout.write_all(&self.stdout)?;
        stderr.write_all(&self.stderr)?;
        Ok(())
    }
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

fn emit(writer: &mut dyn Write, format: Format, payload: &impl Render) -> Result<(), Error> {
    match format {
        Format::Json => {
            serde_json::to_writer_pretty(&mut *writer, payload)
                .map_err(|err| server_error!("failed to serialize JSON response: {err}",))?;
            writeln!(writer).map_err(|err| server_error!(err))
        }
        Format::Text => payload.render(writer).map_err(|err| server_error!(err)),
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

// `NO_COLOR`, missing `TERM`, and `TERM=dumb` disable ANSI styling.
// Wasm has no terminal probe, so only those environment guards apply.
fn error_style() -> (&'static str, &'static str) {
    let opted_out = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty())
        || !std::env::var_os("TERM").is_some_and(|term| !term.is_empty() && term != "dumb");
    let tty = {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::io::IsTerminal as _;
            io::stderr().is_terminal()
        }
        #[cfg(target_arch = "wasm32")]
        {
            true
        }
    };
    if opted_out || !tty { ("", "") } else { ("\x1b[1;31m", "\x1b[0m") }
}
