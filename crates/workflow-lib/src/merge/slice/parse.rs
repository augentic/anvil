//! Time conversions shared by [`super::commit`] and
//! [`super::conflict_check`] — lifts `std::fs::Metadata::modified()`
//! into `jiff::Timestamp` for direct comparison.

use std::time::SystemTime;

use jiff::Timestamp;
use specify_error::Error;

/// Convert a [`SystemTime`] (as returned by `fs::metadata().modified()`)
/// into UTC. Returns a typed `Error::Diag` rather than panicking when
/// the host clock reports a baseline mtime that falls outside jiff's
/// representable range (pre-epoch, overflow, or otherwise unsupported).
pub(super) fn system_time_to_utc(t: SystemTime) -> Result<Timestamp, Error> {
    Timestamp::try_from(t).map_err(|err| Error::Diag {
        code: "merge-mtime-out-of-range",
        detail: format!("baseline mtime out of range: {err}"),
    })
}
