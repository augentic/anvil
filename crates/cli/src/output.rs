//! Command output
//!
//! How a command's result reaches the operator: the output [`Format`] the
//! `--format` flag selects, and the [`Response`] carrying the bytes for
//! stdout and stderr together with the process exit status.
//!
//! Output is buffered rather than written as it is produced, so a run can be
//! driven in-process by tests and by the wasm guest alike, and the caller
//! decides when and where the channels are flushed.

use serde::Serialize;

use super::text::Text;

/// Command output format.
#[derive(Copy, Clone, Debug, clap::ValueEnum)]
pub enum Format {
    /// Human-readable text.
    Text,
    /// Pretty-printed JSON.
    Json,
}

impl Format {
    // Every body is a derived DTO encoded into memory, so encoding has
    // no failure path; the `expect`s state the invariant.
    pub fn encode<T: Serialize + Text>(self, body: &T) -> Vec<u8> {
        match self {
            Self::Text => {
                let mut out = String::new();
                body.text(&mut out).expect("a String sink never fails");
                out.into_bytes()
            }
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
    pub fn success(stdout: impl Into<Vec<u8>>) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: Vec::new(),
            exit: 0,
        }
    }

    /// Creates a failed response carrying raw stderr.
    pub fn failure(stderr: impl Into<Vec<u8>>, exit: u8) -> Self {
        Self {
            stdout: Vec::new(),
            stderr: stderr.into(),
            exit,
        }
    }
}

// A `command!` entry may return the response directly: the guest writes
// its channels here and `execute_wasi` exits with the status.
#[cfg(target_arch = "wasm32")]
impl omnia_guest::api::command::IntoExit for Response {
    fn into_exit(self) -> Result<(), u8> {
        use std::io::Write as _;

        // A refused process channel is unclassified, so it takes the
        // exit the one map gives a `ServerError`.
        std::io::stdout()
            .write_all(&self.stdout)
            .and_then(|()| std::io::stderr().write_all(&self.stderr))
            .map_err(|error| {
                super::exit_code(&omnia_guest::server_error!("process channel: {error}"))
            })?;
        if self.exit == 0 { Ok(()) } else { Err(self.exit) }
    }
}
