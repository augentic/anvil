//! `stamp_outcome` verb: stamp a phase outcome onto `metadata.yaml`.

use std::path::Path;

use jiff::Timestamp;
use specify_error::Error;

use crate::slice::{Outcome, OutcomeKind, SliceMetadata, TargetOperation};

/// Stamp the outcome of a phase run on `<slice_dir>/metadata.yaml`.
///
/// Primary writer of [`SliceMetadata::outcome`] for the define and
/// build phases, and for merge failure/deferred outcomes. The merge
/// success path is handled by `crate::merge::slice::commit`, which
/// stamps the outcome atomically before archiving (the archive move
/// removes `slice_dir` from `.specify/slices/`, so a post-merge call
/// to this function would fail with "not found").
///
/// The whole metadata file is rewritten atomically via
/// [`SliceMetadata::save`] so a concurrent reader never sees a
/// half-written file. A new stamp replaces any previous one — history
/// lives in `.specify/journal.jsonl` (workflow §Observability), not here.
///
/// `now` is plumbed in so tests can pin `at` deterministically; the CLI
/// passes `Timestamp::now()`.
///
/// Returns the updated [`SliceMetadata`].
///
/// # Errors
///
/// Propagates load / save failures from `SliceMetadata`.
pub fn stamp_outcome(
    slice_dir: &Path, phase: TargetOperation, outcome: OutcomeKind, summary: &str,
    context: Option<&str>, now: Timestamp,
) -> Result<SliceMetadata, Error> {
    let mut metadata = SliceMetadata::load(slice_dir)?;
    metadata.outcome = Some(Outcome {
        phase,
        kind: outcome,
        at: now,
        summary: summary.to_string(),
        context: context.map(str::to_string),
    });
    metadata.save(slice_dir)?;
    Ok(metadata)
}
