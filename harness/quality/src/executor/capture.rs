//! Process-stream capture around one in-process step.
//!
//! Omnia wires every guest store's stdout/stderr to the host process
//! streams with no per-store injection seam, so the composed executor
//! captures a step the way a subprocess driver would: temporarily
//! redirect the process file descriptors to temporary files while the
//! guest runs, then restore them and read the captured bytes back.

use std::fs::File;
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::os::fd::{AsRawFd as _, FromRawFd as _, OwnedFd, RawFd};

/// Both process streams redirected into temporary files; restored on
/// [`finish`](Self::finish) or drop.
#[derive(Debug)]
pub struct Capture {
    stdout: Redirect,
    stderr: Redirect,
}

impl Capture {
    /// Redirect stdout and stderr into fresh temporary files.
    ///
    /// # Errors
    ///
    /// Returns descriptor-duplication or temporary-file errors.
    pub fn start() -> io::Result<Self> {
        drop(io::stdout().flush());
        drop(io::stderr().flush());
        Ok(Self {
            stdout: Redirect::start(libc::STDOUT_FILENO)?,
            stderr: Redirect::start(libc::STDERR_FILENO)?,
        })
    }

    /// Restore both streams and return the captured `(stdout, stderr)`.
    ///
    /// # Errors
    ///
    /// Returns descriptor-restoration or read-back errors.
    pub fn finish(self) -> io::Result<(String, String)> {
        Ok((self.stdout.finish()?, self.stderr.finish()?))
    }
}

/// One process stream redirected into a temporary file.
#[derive(Debug)]
struct Redirect {
    fd: RawFd,
    saved: OwnedFd,
    sink: File,
}

impl Redirect {
    #[expect(unsafe_code, reason = "stream redirection is only reachable through raw dup/dup2")]
    fn start(fd: RawFd) -> io::Result<Self> {
        let sink = tempfile::tempfile()?;
        // SAFETY: `dup` on a live standard-stream descriptor; the
        // duplicate is owned and closed by `OwnedFd`.
        let saved = unsafe {
            let duplicate = libc::dup(fd);
            if duplicate < 0 {
                return Err(io::Error::last_os_error());
            }
            OwnedFd::from_raw_fd(duplicate)
        };
        // SAFETY: both descriptors are live; `dup2` atomically points
        // the standard stream at the sink file.
        if unsafe { libc::dup2(sink.as_raw_fd(), fd) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { fd, saved, sink })
    }

    fn finish(mut self) -> io::Result<String> {
        self.restore()?;
        self.sink.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        self.sink.read_to_end(&mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    #[expect(unsafe_code, reason = "stream restoration is only reachable through raw dup2")]
    fn restore(&self) -> io::Result<()> {
        drop(io::stdout().flush());
        drop(io::stderr().flush());
        // SAFETY: `saved` is the descriptor duplicated in `start`;
        // `dup2` atomically restores the original stream.
        if unsafe { libc::dup2(self.saved.as_raw_fd(), self.fd) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for Redirect {
    fn drop(&mut self) {
        // Restore on the error path too, so a failed step never leaves
        // the process streams pointing at a dropped temporary file.
        drop(self.restore());
    }
}
