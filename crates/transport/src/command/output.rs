//! Output format, the single [`emit`] entry point, and the exit-code
//! contract shared by the native binary and the workflow guest.
//! `Exit::from(&Error)` is the single source of truth for failures.

use std::io::Write;
use std::process::ExitCode;

use clap::ValueEnum;
use emery_engine::handler::Render;
use emery_error::Error;
use serde::Serialize;

/// Structured (`json`) or human (`text`) CLI output.
#[derive(Copy, Clone, Debug, Default, ValueEnum, PartialEq, Eq)]
pub enum Format {
    /// Human-readable lines on stdout/stderr.
    #[default]
    Text,
    /// Pretty-printed JSON envelopes for skill/CI consumption.
    Json,
}

/// Emit `payload` through `writer` in the requested format.
///
/// JSON serialises the body directly via
/// `serde_json::to_writer_pretty`; Text delegates to `render_text`.
/// The single signature covers both success (stdout) and failure
/// (stderr) — there is one entry point for all structured output.
/// Callers construct the locked writer at the boundary so the sink
/// choice is visible at the call site.
///
/// # Errors
///
/// Propagates the underlying serialization or I/O error.
pub fn emit<T: Serialize>(
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

/// Process exit code the CLI returns, mapped from a handler result.
///
/// [`Exit::from`] (`&Error`) is the single source of truth for the
/// failure mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Exit {
    /// Command succeeded (exit 0).
    Success,
    /// Any error without a more specific code (exit 1).
    GenericFailure,
    /// Validation findings or `Error::Validation` (exit 2).
    ValidationFailed,
    /// `Error::CliTooOld` — the binary is older than the project floor (exit 3).
    VersionTooOld,
    /// Argument-shape failure: `clap` exits 2 for unknown flags / missing
    /// arguments; we mirror that for argument errors discovered after
    /// parsing (kebab-case checks, mutually exclusive payloads, etc.).
    ArgumentError,
}

impl Exit {
    /// Numeric process exit code for this outcome.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::GenericFailure => 1,
            Self::ArgumentError | Self::ValidationFailed => 2,
            Self::VersionTooOld => 3,
        }
    }
}

impl From<Exit> for ExitCode {
    fn from(r: Exit) -> Self {
        Self::from(r.code())
    }
}

impl From<&Error> for Exit {
    fn from(err: &Error) -> Self {
        match err {
            Error::CliTooOld { .. } | Error::AdapterCliTooOld { .. } => Self::VersionTooOld,
            Error::Validation { .. } => Self::ValidationFailed,
            Error::Argument { .. } => Self::ArgumentError,
            _ => Self::GenericFailure,
        }
    }
}

/// Failure envelope used by the transport projectors for every error
/// variant.
///
/// Payload-free: `error` carries the variant discriminant, `message`
/// the rendered detail, `exit-code` the numeric exit, and `hint` the
/// optional recovery guidance (present in text and JSON alike). No
/// per-finding rows — handlers render `emery_diagnostics::DiagnosticReport`
/// on stdout before returning this. Construct via `ErrorBody::from`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ErrorBody {
    pub(crate) error: std::borrow::Cow<'static, str>,
    pub(crate) message: String,
    pub(crate) exit_code: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'static str>,
}

impl From<&Error> for ErrorBody {
    fn from(err: &Error) -> Self {
        Self {
            error: err.variant_str(),
            message: err.to_string(),
            exit_code: Exit::from(err).code(),
            hint: err.hint(),
        }
    }
}

/// Render a success `body` as the stdout bytes the command projector
/// would emit — the success envelope for callers outside a router
/// dispatch.
///
/// # Errors
///
/// Propagates the underlying serialization or I/O failure; callers
/// route it through [`render_failure`].
pub fn render_success<T: Serialize + Render>(format: Format, body: &T) -> Result<Vec<u8>, Error> {
    let mut stdout = Vec::new();
    emit(&mut stdout, format, body, |w, v| v.render(w))?;
    Ok(stdout)
}

/// Render `error` as the stderr bytes and exit code the command
/// projector would emit — the failure envelope for callers outside a
/// router dispatch.
///
/// Rendering failures collapse onto a plain exit-1 line, mirroring the
/// projector's terminal fallback.
#[must_use]
pub fn render_failure(format: Format, error: &Error) -> (Vec<u8>, u8) {
    let body = ErrorBody::from(error);
    let mut stderr = Vec::new();
    match emit(&mut stderr, format, &body, write_error_text) {
        Ok(()) => (stderr, Exit::from(error).code()),
        Err(fallback) => (format!("error: {fallback}\n").into_bytes(), 1),
    }
}

pub fn write_error_text(w: &mut dyn Write, body: &ErrorBody) -> std::io::Result<()> {
    let (red, reset) = error_style();
    writeln!(w, "{red}error: {}{reset}", body.message)?;
    if let Some(hint) = body.hint {
        writeln!(w, "hint: {hint}")?;
    }
    Ok(())
}

// ANSI red for the `error:` line. `NO_COLOR` (any non-empty value),
// a missing `TERM`, and `TERM=dumb` opt out; under wasm32 there is no
// terminal probe, so the env guards alone decide.
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
