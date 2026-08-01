//! The D1 create-exclusive guest-run marker
//! (`<plan-root>/.emery/guest.lock`): atomic acquisition, holder
//! diagnostics, and drop-time release.

use std::io::Write as _;
use std::path::PathBuf;

use error::Error;
use jiff::Timestamp;
use project::config::Layout;

/// The D1 create-exclusive advisory marker at
/// `<plan-root>/.emery/guest.lock`, held for one guest execute run.
///
/// `OpenOptions::create_new` makes acquisition atomic — exactly one
/// guest execute loop can hold the marker per plan root, so a second
/// in-guest `plan execute` is refused (`guest-marker-held`, exit 2)
/// while a run is live. The file body carries pid / hostname /
/// acquired-at as diagnostics only; existence is the lock.
///
/// **Staleness posture**: the marker is removed when the guard drops
/// (clean exit *and* phase-stop returns *and* error unwinds — any exit
/// that runs destructors). A crash that skips destructors leaves the
/// marker behind, and the next acquire refuses with a detail telling
/// the operator to delete the file after confirming no run is live.
/// No pid-liveness probe: WASI gives the guest no process table to
/// check a recorded pid against, so self-healing would be a guess.
///
/// This marker is the only execute-run interlock.
#[derive(Debug)]
pub struct GuestMarker {
    path: PathBuf,
}

impl GuestMarker {
    /// Atomically create the marker, stamping holder diagnostics into
    /// the body.
    ///
    /// # Errors
    ///
    /// - `guest-marker-held` (exit 2) when the marker already exists —
    ///   a live run or a stale crash leftover.
    /// - [`Error::Io`] on directory-create or write failures.
    pub fn acquire(layout: Layout<'_>, now: Timestamp) -> Result<Self, Error> {
        let path = layout.guest_lock_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }
        let mut file = match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(Error::validation_failed(
                    "guest-marker-held",
                    "no other guest execute run holds the marker",
                    format!(
                        "another guest execute run holds {} — if no run is live (a crash left \
                         the marker behind), delete the file and retry",
                        path.display()
                    ),
                ));
            }
            Err(err) => return Err(Error::Io(err)),
        };
        // Diagnostic body only — existence is the lock. Mirrors the
        // native plan-lock body shape.
        let host = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("HOST"))
            .unwrap_or_else(|_| "unknown".to_string());
        write!(file, "pid={}\nhostname={host}\nacquired-at={now}\n", holder_pid())
            .map_err(Error::Io)?;
        Ok(Self { path })
    }
}

/// The marker body's holder pid — diagnostics only.
///
/// `std::process::id()` aborts on `wasm32-wasip2` (WASI models no
/// process table), so the guest records `0`: the staleness posture never
/// probes the recorded pid, and `0` is unambiguous prose for "no pid on
/// this platform".
#[cfg_attr(
    target_arch = "wasm32",
    expect(
        clippy::missing_const_for_fn,
        reason = "const only on wasm32 (the literal-0 arm); the native body calls process::id()"
    )
)]
fn holder_pid() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::process::id()
    }
}

impl Drop for GuestMarker {
    /// Best-effort removal — a failed unlink degrades to the stale
    /// posture (next acquire refuses and names the file).
    fn drop(&mut self) {
        drop(std::fs::remove_file(&self.path));
    }
}
