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

/// How a built slice's wave-authorized record went stale.
enum Staleness {
    /// The newest opened wave has no build record (the re-build it
    /// authorized failed) while an earlier record still projects the
    /// slice as built — merge would refuse `slice-build-record-missing`.
    OrphanWave { wave: String },
    /// The record's consumed deferred set disagrees with the live
    /// disposition projection.
    Drifted(Drift),
}

/// Compare the wave-authorized build record's consumed deferred set
/// against the live disposition projection. `None` when the slice has
/// nothing built to be stale (no opened wave, or no record at all),
/// or when the sets agree.
fn staleness(layout: Layout<'_>, slice_dir: &Path, name: &str) -> Result<Option<Staleness>> {
    let events = collect_events(layout)?;
    let Some(digest) = events.iter().rev().find_map(|event| match &event.kind {
        EventKind::TargetWaveOpened { digest, members, .. }
            if members.iter().any(|member| member.as_str() == name) =>
        {
            Some(digest.clone())
        }
        _ => None,
    }) else {
        return Ok(None);
    };
    let wave = SnapshotId::parse(&digest)?;
    let Some(record) = BuildRecord::find_for_wave(slice_dir, &wave)? else {
        // A wave with no record and no earlier record is a plain
        // failed first build — the build-failed stop already carries
        // it, and status re-projects Build without this probe.
        if !BuildRecord::present(slice_dir) {
            return Ok(None);
        }
        return Ok(Some(Staleness::OrphanWave { wave: digest }));
    };
    let mut live: Vec<String> = crate::build::deferred::live_deferred(layout, name)?
        .into_iter()
        .map(|req| req.requirement_digest)
        .collect();
    live.sort_unstable();
    // Identical requirement bodies legally share one digest (RFC-86a
    // D2), so the set comparison is over unique digests — mirroring
    // the record side (`BuildRecord::from_capture`).
    live.dedup();
    if live == record.deferred {
        return Ok(None);
    }
    Ok(Some(Staleness::Drifted(Drift {
        recorded: record.deferred,
        live,
    })))
}

/// The execute loop's re-build staleness probe (RFC-86a D4).
///
/// True when the built slice's wave-authorized record is stale — its
/// recorded deferred set no longer matches the live dispositions, or
/// the newest wave's re-build failed and left no record. False when
/// nothing is built.
///
/// # Errors
///
/// Propagates plan / journal / record read failures;
/// `slice-build-record-ambiguous` on duplicate records for the wave.
pub fn dispositions_drifted(layout: Layout<'_>, slice_dir: &Path, name: &str) -> Result<bool> {
    Ok(staleness(layout, slice_dir, name)?.is_some())
}

/// Emit the disposition-drift (or orphan-wave) review finding for one
/// slice.
///
/// No-ops when the slice has nothing built to be stale.
/// Recomputes the live projection; rewrites nothing.
///
/// # Errors
///
/// Propagates plan / journal / record read failures.
pub(super) fn findings(
    layout: Layout<'_>, slice_dir: &Path, name: &str,
) -> Result<Vec<Diagnostic>> {
    Ok(staleness(layout, slice_dir, name)?
        .map(|staleness| match staleness {
            Staleness::Drifted(drift) => Diagnostic::finding(
                "slice-disposition-drifted",
                "the deferred set recorded on a built slice's build record matches the live \
                 disposition projection",
                format!(
                    "slice `{name}` was built under deferred requirement digest(s) [{}] but \
                     the live projection defers [{}] — a deferral lapsed or was \
                     added after the build; re-running `emery plan execute` re-builds this \
                     slice under the current dispositions",
                    digest_set(&drift.recorded),
                    digest_set(&drift.live)
                ),
                Severity::Suggestion,
                DiagnosticKind::Review,
                DiagnosticSource::Deterministic,
                Artifact::Specs,
                None,
            ),
            Staleness::OrphanWave { wave } => Diagnostic::finding(
                "slice-wave-record-missing",
                "a built slice's newest `target.wave.opened` fact has a matching build record",
                format!(
                    "slice `{name}` opened wave `{wave}` but no build record consumes it — the \
                     re-build that wave authorized failed after re-opening the wave; \
                     re-running `emery plan execute` re-builds this slice before merge"
                ),
                Severity::Suggestion,
                DiagnosticKind::Review,
                DiagnosticSource::Deterministic,
                Artifact::Specs,
                None,
            ),
        })
        .into_iter()
        .collect())
}

/// Comma-joined digest set for the drift finding detail, `none` when
/// empty — mirroring pin drift's pinned-vs-live digests.
fn digest_set(digests: &[String]) -> String {
    if digests.is_empty() {
        return "none".to_string();
    }
    digests.join(", ")
}
