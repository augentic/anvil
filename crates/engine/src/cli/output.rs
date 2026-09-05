//! The command output contract: format-aware body encoding and the
//! buffered channel pair a run reports.

use std::fmt::Display;
use std::io::{self, Write};

use serde::Serialize;

/// Command output format.
#[derive(Copy, Clone, Debug, clap::ValueEnum, PartialEq, Eq)]
pub(super) enum Format {
    /// Human-readable text.
    Text,
    /// Pretty-printed JSON.
    Json,
}

impl Format {
    // Every body is a derived DTO encoded into memory, so encoding has
    // no failure path; the `expect` states the invariant.
    pub(super) fn encode<T: Serialize + Display>(self, body: &T) -> Vec<u8> {
        match self {
            Self::Text => body.to_string().into_bytes(),
            Self::Json => {
                let mut out = serde_json::to_vec_pretty(body).expect("a derived DTO serializes");
                out.push(b'\n');
                out
            }
        }
    }
}

/// Buffered command output and process exit status.
#[derive(Debug)]
pub struct Response {
    /// Bytes written to standard output.
    pub stdout: Vec<u8>,
    /// Bytes written to standard error.
    pub stderr: Vec<u8>,
    /// Numeric process exit status.
    pub exit: u8,
}

impl Response {
    /// Creates a successful response carrying raw stdout.
    pub(super) fn success(stdout: impl Into<Vec<u8>>) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: Vec::new(),
            exit: 0,
        }
    }

    /// Creates a failed response carrying raw stderr.
    pub(super) fn failure(stderr: impl Into<Vec<u8>>, exit: u8) -> Self {
        Self {
            stdout: Vec::new(),
            stderr: stderr.into(),
            exit,
        }
    }

    /// Writes both buffered channels to explicit sinks.
    ///
    /// # Errors
    ///
    /// Returns the first sink write error.
    pub fn write_to(&self, stdout: &mut dyn Write, stderr: &mut dyn Write) -> io::Result<()> {
        stdout.write_all(&self.stdout)?;
        stderr.write_all(&self.stderr)
    }
}

// A `command!` entry may return the response directly: the guest writes
// its channels here and `execute_wasi` exits with the status.
#[cfg(target_arch = "wasm32")]
impl omnia_guest::api::command::IntoExit for Response {
    fn into_exit(self) -> Result<(), u8> {
        // A refused process channel is unclassified: the `ServerError` class.
        self.write_to(&mut io::stdout(), &mut io::stderr()).map_err(|_error| 3)?;
        if self.exit == 0 { Ok(()) } else { Err(self.exit) }
    }
}
