//! Command-runner abstraction so tests can substitute canned responses.
//!
//! [`CmdRunner`] is a borrowed callable so production sites pass
//! [`real_cmd`] (or `Command::output`) and test mocks pass closures
//! that capture per-test recording state.
//!
//! [`git`] / [`git_as_specify`] are the single `git` boundary:
//! every registry / init / workspace wrapper builds its
//! `git -C <cwd> <args>` invocation here and runs it through the
//! injected [`CmdRunner`], then maps the returned [`Output`] into its
//! own error type. Spawn failures surface as `Err(io::Error)`; a
//! non-zero git exit surfaces as `Ok(Output)` with a non-success
//! status, so callers keep their distinct spawn-vs-command diagnostics.

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

/// Run `git -c user.name=Specify -c user.email=… -C <cwd> <args>`
/// through `runner` — the identity-pinned variant used by workspace
/// materialisation/commit flows.
///
/// # Errors
///
/// See [`git`].
pub fn git_as_specify<I, S>(runner: CmdRunner<'_>, cwd: &Path, args: I) -> io::Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("git");
    command
        .args(["-c", "user.name=Specify", "-c", "user.email=specify@example.invalid"])
        .arg("-C")
        .arg(cwd)
        .args(args);
    runner(&mut command)
}

/// Classified failure from [`git_checked`] / [`git_as_specify_checked`]:
/// the spawn-vs-command split every wrapper used to re-roll by hand.
/// Callers map each arm onto their own diagnostic code family.
#[derive(Debug)]
pub enum GitFailure {
    /// The `git` child process could not start.
    Spawn(io::Error),
    /// `git` ran but exited non-zero; carries the trimmed stderr.
    Exit {
        /// Trimmed stderr text from the failed command.
        stderr: String,
    },
}

/// [`git`] plus exit-status classification: a non-zero git exit becomes
/// [`GitFailure::Exit`] (trimmed stderr captured), so callers receive an
/// [`Output`] only for a successful run.
///
/// # Errors
///
/// [`GitFailure::Spawn`] when the child cannot start;
/// [`GitFailure::Exit`] on a non-zero git exit.
pub fn git_checked<I, S>(
    runner: CmdRunner<'_>, cwd: Option<&Path>, args: I,
) -> Result<Output, GitFailure>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    classify(git(runner, cwd, args))
}

/// [`git_as_specify`] with the same exit-status classification as
/// [`git_checked`].
///
/// # Errors
///
/// See [`git_checked`].
pub fn git_as_specify_checked<I, S>(
    runner: CmdRunner<'_>, cwd: &Path, args: I,
) -> Result<Output, GitFailure>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    classify(git_as_specify(runner, cwd, args))
}

fn classify(result: io::Result<Output>) -> Result<Output, GitFailure> {
    let output = result.map_err(GitFailure::Spawn)?;
    if output.status.success() {
        return Ok(output);
    }
    Err(GitFailure::Exit {
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}
