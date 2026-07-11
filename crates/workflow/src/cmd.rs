//! Command-runner boundary for workspace-slot Git inspection.

use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

/// Borrowed callable that executes a fully-prepared [`Command`].
pub type CmdRunner<'a> = &'a dyn Fn(&mut Command) -> io::Result<Output>;

/// Default [`CmdRunner`] body that actually spawns the child process.
///
/// # Errors
///
/// Returns any I/O error encountered while spawning or waiting on the
/// child process.
pub fn real_cmd(cmd: &mut Command) -> io::Result<Output> {
    cmd.output()
}

/// Run `git [-C <cwd>] <args>` through `runner` — the shared git
/// boundary for the registry / init / workspace wrappers.
///
/// # Errors
///
/// Returns the spawn [`io::Error`] when the child cannot start. A
/// non-zero git exit returns `Ok(Output)` with a non-success status so
/// callers map it to their own command-failure diagnostic.
pub fn git<I, S>(runner: CmdRunner<'_>, cwd: Option<&Path>, args: I) -> io::Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("git");
    if let Some(cwd) = cwd {
        command.arg("-C").arg(cwd);
    }
    command.args(args);
    runner(&mut command)
}
