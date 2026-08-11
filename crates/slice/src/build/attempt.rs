//! The RFC-90 D6 attempt store: one attempt directory per build
//! invocation at `<slice_dir>/build/attempts/<4-digit ordinal>/`,
//! holding the request copy, phase reports, and continuation state.

use std::path::{Path, PathBuf};

use artifacts::atomic::{bytes_write, serialise_yaml, yaml_write};
use error::{Error, Result};
use project::seam::wire::{BuildReport, PhaseReport};

use super::gate::PhaseOperation;

/// One allocated build attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    /// Monotonic attempt ordinal (1-based).
    pub id: u32,
    /// The attempt directory,
    /// `<slice_dir>/build/attempts/<id 4-digit>/`.
    pub dir: PathBuf,
}

/// One persisted phase report's location and content digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseRecord {
    /// Absolute path of the written `phases/<ordinal>-<operation>.yaml`.
    pub path: PathBuf,
    /// `sha256:<hex>` over the exact written bytes.
    pub digest: String,
}

/// Allocate the next attempt under `<slice_dir>/build/attempts/`.
///
/// Scans the existing numeric directories, then atomically
/// `create_dir`s the next absent ordinal, looping on `AlreadyExists`
/// so concurrent allocators never share or reuse an id — even an
/// unterminated (abandoned) attempt keeps its ordinal forever.
///
/// # Errors
///
/// Propagates filesystem failures creating or scanning the attempts
/// tree.
pub fn allocate(slice_dir: &Path) -> Result<Attempt> {
    let attempts = slice_dir.join("build").join("attempts");
    std::fs::create_dir_all(&attempts)?;
    let mut next = highest_ordinal(&attempts)?.map_or(1, |highest| highest.saturating_add(1));
    loop {
        let dir = attempts.join(format!("{next:04}"));
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(Attempt { id: next, dir }),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                next = next.saturating_add(1);
            }
            Err(err) => return Err(Error::Io(err)),
        }
    }
}

/// The highest numeric attempt ordinal under `attempts`, if any.
fn highest_ordinal(attempts: &Path) -> Result<Option<u32>> {
    let mut highest = None;
    for entry in std::fs::read_dir(attempts)? {
        let entry = entry?;
        if let Some(ordinal) = entry.file_name().to_str().and_then(|name| name.parse::<u32>().ok())
        {
            highest = Some(highest.map_or(ordinal, |current: u32| current.max(ordinal)));
        }
    }
    Ok(highest)
}

/// Copy the immutable `build/request.yaml` into the attempt directory
/// (atomic write of the read bytes).
///
/// # Errors
///
/// Propagates the read of `<slice_dir>/build/request.yaml` and the
/// atomic write into the attempt.
pub fn copy_request(attempt: &Attempt, slice_dir: &Path) -> Result<()> {
    let bytes = std::fs::read(slice_dir.join("build").join("request.yaml"))?;
    bytes_write(&attempt.dir.join("request.yaml"), &bytes)
}

/// Atomically persist one returned phase report as
/// `phases/<2-digit ordinal>-<operation>.yaml` and return its
/// [`PhaseRecord`].
///
/// The report's `next_continuation` is `#[serde(skip)]`, so the
/// continuation never rides the persisted YAML — it is stored
/// separately via [`store_continuation`]. The digest covers the exact
/// written bytes; engine-measured elapsed time stays outside it.
///
/// # Errors
///
/// Propagates YAML serialization and atomic-write failures.
pub fn write_phase(
    attempt: &Attempt, ordinal: u32, operation: PhaseOperation, report: &PhaseReport,
) -> Result<PhaseRecord> {
    let yaml = serialise_yaml(report)?;
    let path = attempt.dir.join("phases").join(format!("{ordinal:02}-{operation}.yaml"));
    bytes_write(&path, yaml.as_bytes())?;
    let digest = format!("sha256:{}", diagnostics::digest::sha256_hex(yaml.as_bytes()));
    Ok(PhaseRecord { path, digest })
}

/// Atomically persist the attempt's continuation payload at
/// `continuation.bin`. Continuations never cross attempts — there is
/// deliberately no cross-attempt lookup.
///
/// # Errors
///
/// Propagates the atomic write.
pub fn store_continuation(attempt: &Attempt, bytes: &[u8]) -> Result<()> {
    bytes_write(&attempt.dir.join("continuation.bin"), bytes)
}

/// Remove the attempt's continuation payload; absent is already
/// clear.
///
/// # Errors
///
/// Propagates filesystem failures other than the file being absent.
pub fn clear_continuation(attempt: &Attempt) -> Result<()> {
    match std::fs::remove_file(attempt.dir.join("continuation.bin")) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(Error::Io(err)),
    }
}

/// Load the attempt's continuation payload; `None` when absent.
///
/// # Errors
///
/// Propagates filesystem failures other than the file being absent.
pub fn load_continuation(attempt: &Attempt) -> Result<Option<Vec<u8>>> {
    match std::fs::read(attempt.dir.join("continuation.bin")) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(Error::Io(err)),
    }
}

/// Atomically write the attempt's terminal report at `report.yaml`,
/// beside `phases/`. The caller projects the same body to the
/// canonical `build/report.yaml` (RFC-90 D6).
///
/// # Errors
///
/// Propagates YAML serialization and atomic-write failures.
pub fn write_terminal(attempt: &Attempt, report: &BuildReport) -> Result<()> {
    yaml_write(&attempt.dir.join("report.yaml"), report)
}
