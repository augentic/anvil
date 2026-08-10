//! Disposition-drift signals over the build record's deferred set
//! (RFC-86a D4) — pin drift's build-scope analog, feeding validate's
//! review findings and the execute loop's [`dispositions_drifted`].

use std::path::Path;

use diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use error::Result;
use project::build_record::BuildRecord;
use project::config::Layout;
use project::journal::EventKind;
use project::plan::collect_events;
use project::snapshot::SnapshotId;

/// The recorded vs live deferred digest sets, when they disagree.
struct Drift {
    recorded: Vec<String>,
    live: Vec<String>,
}

/// Compare the wave-authorized build record's consumed deferred set
/// against the live disposition projection. `None` when the slice has
/// no opened wave or no record for it (nothing built to be stale), or
/// when the sets agree.
fn drift(layout: Layout<'_>, slice_dir: &Path, name: &str) -> Result<Option<Drift>> {
    let Some(record) = wave_record(layout, slice_dir, name)? else {
        return Ok(None);
    };
    let mut live: Vec<String> = crate::build::deferred::live_deferred(layout, name)?
        .into_iter()
        .map(|req| req.requirement_digest)
        .collect();
    live.sort_unstable();
    if live == record.deferred {
        return Ok(None);
    }
    Ok(Some(Drift {
        recorded: record.deferred,
        live,
    }))
}

/// The build record the slice's newest `target.wave.opened` fact
/// authorizes — the record merge would consume. `None` without a wave
/// fact or a matching record.
fn wave_record(layout: Layout<'_>, slice_dir: &Path, name: &str) -> Result<Option<BuildRecord>> {
    let events = collect_events(layout)?;
    let Some(digest) = events.iter().rev().find_map(|event| match &event.kind {
        EventKind::TargetWaveOpened {
            digest, slice_name, ..
        } if slice_name.as_str() == name => Some(digest.clone()),
        _ => None,
    }) else {
        return Ok(None);
    };
    let wave = SnapshotId::parse(&digest)?;
    Ok(BuildRecord::load_all(slice_dir)?.into_iter().find(|record| record.wave == wave))
}

/// True when the built slice's recorded deferred set no longer matches
/// the live dispositions — the execute loop's re-build staleness probe
/// (RFC-86a D4). False when nothing is built.
///
/// # Errors
///
/// Propagates plan / journal / record read failures.
pub fn dispositions_drifted(layout: Layout<'_>, slice_dir: &Path, name: &str) -> Result<bool> {
    Ok(drift(layout, slice_dir, name)?.is_some())
}

/// Emit the disposition-drift review finding for one slice.
///
/// No-ops when the slice has no wave-authorized build record.
/// Recomputes the live projection; rewrites nothing.
///
/// # Errors
///
/// Propagates plan / journal / record read failures.
pub(super) fn findings(
    layout: Layout<'_>, slice_dir: &Path, name: &str,
) -> Result<Vec<Diagnostic>> {
    Ok(drift(layout, slice_dir, name)?
        .map(|drift| {
            Diagnostic::finding(
                "slice-disposition-drifted",
                "the deferred set recorded on a built slice's build record matches the live \
                 disposition projection",
                format!(
                    "slice `{name}` was built under {} deferred requirement(s) but the live \
                     projection defers {} — a deferral was retracted, lapsed, or added after \
                     the build; re-running `emery plan execute` re-builds this slice under \
                     the current dispositions",
                    drift.recorded.len(),
                    drift.live.len()
                ),
                Severity::Suggestion,
                DiagnosticKind::Review,
                DiagnosticSource::Deterministic,
                Artifact::Specs,
                None,
            )
        })
        .into_iter()
        .collect())
}
