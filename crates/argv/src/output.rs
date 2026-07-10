//! Output format, the single [`emit`] entry point, and the exit-code
//! contract shared by the native `specify` binary and the workflow
//! guest.
//!
//! `Exit::from(&Error)` is the single source of truth for the failure
//! mapping.

use std::io::Write;
use std::process::ExitCode;

use clap::ValueEnum;
use error::Error;
use serde::Serialize;

/// Structured (`json`) or human (`text`) CLI output.
#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum Format {
    /// Human-readable lines on stdout/stderr.
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
    /// WASI tool exit-code passthrough.
    Code(u8),
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
            Self::Code(code) => code,
        }
    }
}

/// The closed exit-code table as `(code, name, meaning)` rows.
///
/// [`Exit::Code`] (the WASI tool passthrough) is open-ended by design
/// and intentionally absent. The `exit_code_table_matches_exit` test
/// pins each row to the matching [`Exit`] variant.
pub const EXIT_CODES: &[(u8, &str, &str)] = &[
    (0, "success", "Command succeeded."),
    (
        1,
        "generic-failure",
        "Any error without a more specific code (I/O, YAML, schema, merge, tool resolver/runtime, …).",
    ),
    (
        2,
        "validation-failed",
        "Validation findings, invalid arguments, or an undeclared/over-permissioned tool request.",
    ),
    (3, "version-too-old", "project.yaml.specify is newer than the binary."),
];

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

/// Render `err` as a failure envelope and return the matching exit
/// code.
///
/// JSON serialises the body directly; Text writes `error: {err}` plus
/// any long-form hint for the variant. Both formats route through
/// [`emit`] against `std::io::stderr()` so failure output never
/// interleaves with the structured success stream skills consume.
///
/// Single dispatcher entry point: handlers return
/// `Result<T, error::Error>` and the run loop in
/// [`crate::commands`] hands the error here. The body shape is
/// always [`ErrorBody`]. `Error::Validation` is payload-free — its
/// `code` becomes the wire `error` discriminant and its `detail` the
/// `message`; per-finding rows are rendered by the producing handler on
/// stdout as a `schema::diagnostics::DiagnosticReport` before the
/// payload-free error is returned.
pub fn report(format: Format, err: &Error) -> Exit {
    let code = Exit::from(err);
    let body = ErrorBody::from(err);
    let result = emit(&mut std::io::stderr().lock(), format, &body, write_error_text);
    if let Err(serialise_err) = result {
        eprintln!("error: {err}");
        eprintln!("error: {serialise_err}");
    }
    code
}

/// Failure envelope used by [`report`] for every error variant.
///
/// The shape is now payload-free: `error` carries the variant
/// discriminant (the `code` for `Error::Validation`), `message` the
/// rendered detail, and `exit-code` the numeric exit. The error body
/// carries no per-finding rows — handlers render
/// `schema::diagnostics::DiagnosticReport` on stdout before
/// returning the payload-free error.
///
/// Construct via `ErrorBody::from(&err)` — the variant is the only
/// shape on the wire.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ErrorBody {
    pub(crate) error: std::borrow::Cow<'static, str>,
    pub(crate) message: String,
    pub(crate) exit_code: u8,
    #[serde(skip)]
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

fn write_error_text(w: &mut dyn Write, body: &ErrorBody) -> std::io::Result<()> {
    writeln!(w, "error: {}", body.message)?;
    if let Some(hint) = body.hint {
        writeln!(w, "hint: {hint}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{EXIT_CODES, Exit};

    #[test]
    fn exit_code_table_matches_exit() {
        // Every fixed Exit variant has exactly one table row whose
        // numeric code matches `Exit::code()`; `Exit::Code` is the
        // open-ended WASI passthrough and stays out of the table.
        let by_code = |code: u8| {
            EXIT_CODES
                .iter()
                .find(|(c, _, _)| *c == code)
                .unwrap_or_else(|| panic!("EXIT_CODES missing a row for code {code}"))
        };
        assert_eq!(by_code(Exit::Success.code()).1, "success");
        assert_eq!(by_code(Exit::GenericFailure.code()).1, "generic-failure");
        assert_eq!(by_code(Exit::ValidationFailed.code()).1, "validation-failed");
        assert_eq!(by_code(Exit::ArgumentError.code()).1, "validation-failed");
        assert_eq!(by_code(Exit::VersionTooOld.code()).1, "version-too-old");
        assert_eq!(EXIT_CODES.len(), 4, "one row per fixed exit code");
    }
}
