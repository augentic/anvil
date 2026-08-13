//! Fail-closed resolution of the current reviewed handoff for one wave.

use std::io::ErrorKind;
use std::path::Path;

use error::Error;

use super::{Handoff, Home};
use crate::journal::{Event, EventKind};
use crate::snapshot::SnapshotId;

/// Current reviewed handoff projection for one wave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reviewed {
    /// Validated handoff body.
    pub handoff: Handoff,
    /// Canonical handoff digest (`sha256:<64 hex>`).
    pub digest: SnapshotId,
    /// Matching `system.wave.reviewed` envelope (first in union order).
    pub review: Event,
    /// Canonical digest of [`Self::review`].
    pub event_digest: SnapshotId,
}

/// Resolve the single current handoff for `wave` under `root` and
/// verify its `system.wave.reviewed` fact.
///
/// Multiple matching projections fail closed rather than selecting by
/// timestamp. The review identity is `(writer, sequence, event-digest)`.
///
/// # Errors
///
/// `definition-handoff-missing` / `-ambiguous` / `-mismatch` /
/// `-malformed`; `definition-review-missing`; `definition-event-malformed`.
pub fn resolve(root: &Path, wave: &str) -> Result<Reviewed, Error> {
    let home = Home::new(root);
    let mut found = Vec::new();
    for path in yaml_files(&home.handoffs_dir())? {
        let handoff = Handoff::load(&path)?;
        if handoff.wave.id == wave {
            found.push((path, handoff));
        }
    }
    let (path, handoff) = match found.len() {
        0 => return Err(missing(wave)),
        1 => found.remove(0),
        n => {
            return Err(Error::Diag {
                code: "definition-handoff-ambiguous",
                detail: format!(
                    "definition home has {n} current handoff projections for wave `{wave}`"
                ),
            });
        }
    };
    let digest = handoff.digest()?;
    let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
    if stem != digest.digest() {
        return Err(Error::Diag {
            code: "definition-handoff-mismatch",
            detail: format!(
                "handoff file `{}` does not match canonical digest `{digest}`",
                path.display()
            ),
        });
    }
    let review = matching_review(&home, &digest)?;
    let event_digest = review.digest()?;
    Ok(Reviewed {
        handoff,
        digest,
        review,
        event_digest,
    })
}

fn missing(wave: &str) -> Error {
    Error::Diag {
        code: "definition-handoff-missing",
        detail: format!("no current handoff projection for wave `{wave}`"),
    }
}

fn yaml_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, Error> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(Error::Filesystem {
                op: "readdir",
                path: dir.to_path_buf(),
                source: err,
            });
        }
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Filesystem {
            op: "readdir",
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }
        paths.push(path);
    }
    paths.sort();
    Ok(paths)
}

fn matching_review(home: &Home<'_>, digest: &SnapshotId) -> Result<Event, Error> {
    let events = read_events(&home.events_dir())?;
    events
        .into_iter()
        .find(|event| {
            matches!(
                &event.kind,
                EventKind::SystemWaveReviewed { handoff_digest } if handoff_digest == digest
            )
        })
        .ok_or_else(|| Error::Diag {
            code: "definition-review-missing",
            detail: format!("no system.wave.reviewed fact names handoff `{digest}`"),
        })
}

/// Read a definition-home event root. Unparseable lines fail closed.
fn read_events(dir: &Path) -> Result<Vec<Event>, Error> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(Error::Filesystem {
                op: "readdir",
                path: dir.to_path_buf(),
                source: err,
            });
        }
    };
    let mut events = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Filesystem {
            op: "readdir",
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        events.extend(read_file(&path)?);
    }
    events.sort_by(|left, right| {
        (left.timestamp, left.writer.as_str(), left.sequence).cmp(&(
            right.timestamp,
            right.writer.as_str(),
            right.sequence,
        ))
    });
    Ok(events)
}

fn read_file(path: &Path) -> Result<Vec<Event>, Error> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(Error::Filesystem {
                op: "read",
                path: path.to_path_buf(),
                source: err,
            });
        }
    };
    let mut events = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Event>(line) {
            Ok(event) => events.push(event),
            Err(err) => {
                return Err(Error::Diag {
                    code: "definition-event-malformed",
                    detail: format!("{}:{}: {err}", path.display(), index + 1),
                });
            }
        }
    }
    Ok(events)
}
