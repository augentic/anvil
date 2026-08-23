//! Output rendering and process exit codes.
//! [`Exit::from`] is the failure-code authority.

use std::io::Write;
use std::process::ExitCode;

use clap::ValueEnum;
use emery_engine::handler::Render;
use emery_error::Error;
use serde::Serialize;

/// CLI output format.
#[derive(Copy, Clone, Debug, Default, ValueEnum, PartialEq, Eq)]
pub enum Format {
    /// Human-readable text.
    #[default]
    Text,
    /// Pretty-printed JSON.
    Json,
}

/// Writes `payload` in the requested format.
///
/// # Errors
///
/// Returns serialization or I/O failures.
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

/// Process exit classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Exit {
    /// Success (exit 0).
    Success,
    /// Unclassified failure (exit 1).
    GenericFailure,
    /// Validation failure (exit 2).
    ValidationFailed,
    /// Incompatible old version (exit 3).
    VersionTooOld,
    /// Post-parse argument failure (exit 2).
    ArgumentError,
}

impl Exit {
    /// Returns the numeric exit code.
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
            Error::AdapterCliTooOld { .. } => Self::VersionTooOld,
            Error::Validation { .. } => Self::ValidationFailed,
            Error::Argument { .. } => Self::ArgumentError,
            _ => Self::GenericFailure,
        }
    }
}

/// Serialized command failure.
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

/// Renders command-success bytes outside router dispatch.
///
/// # Errors
///
/// Returns serialization or I/O failures.
pub fn render_success<T: Serialize + Render>(format: Format, body: &T) -> Result<Vec<u8>, Error> {
    let mut stdout = Vec::new();
    emit(&mut stdout, format, body, |w, v| v.render(w))?;
    Ok(stdout)
}

/// Renders command-failure bytes and their exit code.
///
/// Rendering failures become a plain exit-1 line.
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
