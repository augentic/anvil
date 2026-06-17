//! Plan-lock acquisition and runtime enforcement.
//!
//! The `/spec:execute` driver lock is the OS advisory lock on
//! `<plan-root>/.specify/plan.lock`. [`acquire`] takes it for the
//! lifetime of the returned [`PlanLockGuard`] — the holder behind the
//! `specify plan lock -- <cmd>` command-wrapper verb, which spawns
//! `<cmd>` under the lock and releases on the child's exit. The
//! plan-state-writing verbs (`plan next`, per-entry `plan transition`,
//! `slice merge run`) call [`require_held`] and refuse an unlocked
//! driver with `plan-lock-not-held` (exit 2), so dual-driving refusal
//! is a runtime property rather than a per-skill snippet discipline.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use jiff::Timestamp;
use specify_error::Error;

use crate::config::Layout;

/// Probe outcome for the plan lock at one path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockProbe {
    /// Some process holds the lock — a driver session is active.
    Held,
    /// Nobody holds the lock (including: the lockfile does not exist).
    Unheld,
}

/// Refuse the calling verb unless a driver session holds the plan lock.
///
/// # Errors
///
/// [`Error::Validation`] `plan-lock-not-held` (exit 2) when the probe
/// reports [`LockProbe::Unheld`]; I/O failures from the probe itself.
pub fn require_held(layout: Layout<'_>) -> Result<(), Error> {
    let path = layout.plan_lock_path();
    match probe(&path)? {
        LockProbe::Held => Ok(()),
        LockProbe::Unheld => Err(Error::validation_failed(
            "plan-lock-not-held",
            "no driver session holds the plan lock",
            format!(
                "acquire {} with the flock snippet in plan-lock.md before driving plan state \
                 (every /spec:execute and breakout session holds it for the session's lifetime)",
                path.display()
            ),
        )),
    }
}

/// Exclusive hold on `<plan-root>/.specify/plan.lock` for the guard's
/// lifetime.
///
/// Dropping the guard closes the descriptor, which releases the OS
/// advisory lock — so the lock lives exactly as long as the
/// `specify plan lock -- <cmd>` child process the handler spawns.
#[derive(Debug)]
pub struct PlanLockGuard {
    _file: File,
}

/// Acquire the exclusive advisory lock at `layout.plan_lock_path()`.
///
/// Creates `.specify/` and the lockfile as needed, and stamps the
/// holder pid / hostname / acquisition time into the file body as
/// diagnostic noise (the body is never the lock identity — the OS file
/// lock is).
///
/// Non-blocking: a second driver that finds the lock held fails fast
/// rather than waiting.
///
/// `now` is injected so this module never reads the clock itself.
///
/// # Errors
///
/// [`Error::Validation`] `plan-lock-busy` (exit 2) when another process
/// already holds the lock — the message carries the holder pid read
/// from the lockfile body. I/O failures from opening, locking, or
/// writing the lockfile surface as [`Error::Io`].
pub fn acquire(layout: Layout<'_>, now: Timestamp) -> Result<PlanLockGuard, Error> {
    let path = layout.plan_lock_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }
    let mut file = File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(Error::Io)?;
    match file.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => {
            let holder = holder_pid(&path);
            return Err(Error::validation_failed(
                "plan-lock-busy",
                "another driver session holds the plan lock",
                format!("holder-pid={holder}"),
            ));
        }
        Err(std::fs::TryLockError::Error(err)) => return Err(Error::Io(err)),
    }
    write_body(&mut file, now).map_err(Error::Io)?;
    Ok(PlanLockGuard { _file: file })
}

/// Diagnostic lock-body writer: truncate then write the holder pid,
/// hostname, and acquisition timestamp. Best-effort metadata; the OS
/// advisory lock — not these bytes — is the lock identity.
fn write_body(file: &mut File, now: Timestamp) -> std::io::Result<()> {
    file.set_len(0)?;
    let host = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "unknown".to_string());
    write!(file, "pid={}\nhostname={host}\nacquired-at={now}\n", std::process::id())?;
    file.flush()
}

/// Read the `pid=` line from a held lockfile body for the busy
/// diagnostic. Returns `unknown` when the body is missing or carries no
/// `pid=` line (a holder that died mid-write, or a hand-truncated file).
fn holder_pid(path: &Path) -> String {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|body| {
            body.lines()
                .find_map(|line| line.strip_prefix("pid=").map(|pid| pid.trim().to_string()))
        })
        .filter(|pid| !pid.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Probe whether any process holds an exclusive advisory lock on
/// `path`. A missing lockfile is [`LockProbe::Unheld`] — no driver
/// session ever created it.
///
/// A single `flock(2)`-family try-acquire suffices: [`acquire`] is the
/// only writer of this lock, and it takes the same flock-family lock
/// (std's `File::try_lock`). A non-blocking acquire that succeeds is
/// released immediately and reports [`LockProbe::Unheld`]; a would-block
/// means a driver holds it.
///
/// # Errors
///
/// Propagates I/O failures other than a missing file; a `flock` failure
/// other than "would block" surfaces as [`Error::Io`].
pub fn probe(path: &Path) -> Result<LockProbe, Error> {
    let file = match File::options().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(LockProbe::Unheld),
        Err(err) => return Err(Error::Io(err)),
    };
    imp::probe_open(&file)
}

#[cfg(unix)]
mod imp {
    use std::fs::File;

    use specify_error::Error;

    use super::LockProbe;

    /// `flock`-family try-acquire (std's `File::try_lock`) on the
    /// caller's fresh descriptor: success means nobody held it (the
    /// probe lock is released immediately); would-block means a driver
    /// holds it.
    pub(super) fn probe_open(file: &File) -> Result<LockProbe, Error> {
        match file.try_lock() {
            Ok(()) => {
                file.unlock().map_err(Error::Io)?;
                Ok(LockProbe::Unheld)
            }
            Err(std::fs::TryLockError::WouldBlock) => Ok(LockProbe::Held),
            Err(std::fs::TryLockError::Error(err)) => Err(Error::Io(err)),
        }
    }
}

#[cfg(not(unix))]
mod imp {
    use std::fs::File;

    use specify_error::Error;

    use super::LockProbe;

    /// No advisory-lock probe off Unix: report `Held` so enforcement
    /// degrades to permissive rather than bricking every driver verb.
    pub(super) fn probe_open(_file: &File) -> Result<LockProbe, Error> {
        Ok(LockProbe::Held)
    }
}

#[cfg(test)]
mod tests;
