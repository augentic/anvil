//! The merge orchestrator: target merge gates around the deterministic
//! core merge, with RFC-86 D9 one-member wave commit + identity maps.

use std::path::{Path, PathBuf};

use error::Error;
use jiff::Timestamp;
use project::build_record::BuildRecord;
use project::config::{Layout, ProjectConfig};
use project::journal::{self, EventKind, FactEpochRef, IdentityMap};
use project::plan::{Plan, Status, collect_events, project_ladders};
use project::seam::{self, MergePhase, Target, Workspaces};
use project::snapshot::CodePatch;
use project::wave::Wave;

use crate::merge::{MergeCommit, PreviewEntry, artifact_classes, identity, slice as slice_merge};

mod gate;

use gate::{enforce_gate, fetch_gate_report, persist_gate_report, run_gate};

/// The result of a completed guest [`merge`].
#[derive(Debug, Clone)]
pub struct MergeOutcome {
    /// The 3-way merged spec/composition entries.
    pub merged: Vec<PreviewEntry>,
    /// `DEC-NNNN` ids promoted into the Decision Record catalogue.
    pub decisions: Vec<String>,
    /// Where the slice's working directory was archived.
    pub archive_path: PathBuf,
}

/// Context loaded once for wave commit + gates.
struct WaveCommit {
    /// Loaded and revalidated wave manifest.
    wave: Wave,
    /// Content digest of the wave (`sha256:…`).
    digest: String,
    /// Code patch projected from the build record.
    patch: CodePatch,
}

/// Merge one built slice (the execute loop's merge phase).
///
/// Runs preflight gate → identity finalization → deterministic commit
/// → wave-committed fact → postflight gate. A preflight failure
/// aborts with the slice still built; a postflight failure is terminal
/// but non-rollback — the merge stands once `target.merge.wave-committed`
/// is appended, and a parseable failed report is persisted to the
/// archive first. Re-entry heals a torn merge without a second commit.
///
/// # Errors
///
/// Completion-gate and preflight failures (slice not built / not
/// claimed in-progress, `target-merge-preflight-failed`), the terminal
/// `target-merge-postflight-failed`, plus commit and archive I/O
/// failures. Failures before `target.merge.wave-committed` leave no
/// merged projection.
#[tracing::instrument(
    name = "slice.merge",
    skip_all,
    fields(slice = %slice, target = tracing::field::Empty)
)]
pub async fn merge<T: Target + Workspaces>(
    targets: &T, layout: Layout<'_>, now: Timestamp, slice: &str, allow_composition_replace: bool,
) -> Result<MergeOutcome, Error> {
    tracing::info!("merge started");
    preflight_completion(layout, slice)?;
    if let Some(outcome) = heal_torn_merge(layout, slice) {
        tracing::info!("merge completed: torn merge healed (archive already present)");
        return Ok(outcome);
    }
    let slice_dir = layout.slice_dir(slice);
    // The recorded slice target keeps its `name@version` pin: the
    // routed id dispatches the exact identity, never a reduced bare
    // name.
    let target = project::target_policy::resumed(layout, slice)?;
    tracing::Span::current().record("target", target.as_str());
    let id =
        project::adapter::RoutedId::recorded(project::adapter::Axis::Target, &target).to_string();
    let commit = load_wave_commit(layout, slice, &slice_dir)?;

    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceMergeStarted {
            slice_name: slice.into(),
        },
        "slice.merge",
    );

    // One read-only view of the built result snapshot serves both
    // gates; discarded on every exit (best-effort — a leaked view is
    // GC territory, never a merge failure).
    let view =
        journal_on_failure(layout, now, slice, prepare_view(targets, slice, &commit.patch).await)?;
    let run = gated(
        targets,
        layout,
        now,
        slice,
        &slice_dir,
        &id,
        allow_composition_replace,
        &view,
        &commit,
    )
    .await;
    if let Err(err) = targets.discard(view.id.clone()).await {
        tracing::warn!(workspace = %view.id, "merge view discard failed: {err}");
    }
    let outcome = run?;

    // Interim code delivery (deleted by RFC-88): the postflight gate
    // passed, so materialize the accepted result snapshot onto the
    // product tree and journal the apply.
    journal_on_failure(layout, now, slice, apply_result(targets, slice, &commit.patch).await)?;
    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceCodeApplied {
            slice_name: slice.into(),
            snapshot: commit.patch.result.to_string(),
        },
        "slice.merge",
    );

    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceMergeSucceeded {
            slice_name: slice.into(),
        },
        "slice.merge",
    );
    tracing::info!(decisions = outcome.decisions.len(), "merge completed");
    Ok(outcome)
}

/// The gate-bracketed core: preflight → identity + commit →
/// wave-committed → postflight, all over the shared read-only `view`.
#[expect(
    clippy::too_many_arguments,
    reason = "internal merge kernel bracketed by the view lifecycle; callers use `merge`"
)]
async fn gated<T: Target>(
    targets: &T, layout: Layout<'_>, now: Timestamp, slice: &str, slice_dir: &Path, id: &str,
    allow_composition_replace: bool, view: &seam::Workspace, commit: &WaveCommit,
) -> Result<MergeOutcome, Error> {
    // Target preflight: a failure aborts with the slice still `built`.
    let preflight = run_gate(targets, id, slice, MergePhase::Preflight, view).await;
    let preflight = journal_on_failure(layout, now, slice, preflight)?;
    persist_gate_report(&slice_dir.join("merge"), MergePhase::Preflight, &preflight)?;

    // Identity finalization before the deterministic fold — rewrites
    // slice-local ids to baseline numbers; drifted MODIFIED aborts
    // before any baseline write or wave-committed fact.
    let identity_maps =
        journal_on_failure(layout, now, slice, identity::finalize(&layout.specs_dir(), slice_dir))?;

    // The deterministic core: validators, spec fold, Decision Record
    // promotion, lifecycle, and archive.
    let outcome = journal_on_failure(
        layout,
        now,
        slice,
        commit_run(layout, now, slice, allow_composition_replace),
    )?;

    // Wave commit fact — merge authority (RFC-86 D9 / D27). Strict
    // append: failures before this fact must not project merged.
    emit_wave_committed(layout, now, slice, commit, &identity_maps)?;

    // The slice is already merged and archived, so postflight failure
    // is terminal, never a rollback; persist any parseable report first
    // and route every post-commit error via `postflight_terminal`.
    let archive_merge = outcome.archive_path.join("merge");
    match fetch_gate_report(targets, id, slice, MergePhase::Postflight, view).await {
        Ok(report) => {
            let persist_err =
                persist_gate_report(&archive_merge, MergePhase::Postflight, &report).err();
            if let Err(err) = enforce_gate(&report, MergePhase::Postflight, slice) {
                return postflight_terminal(layout, now, slice, commit, &err);
            }
            if let Some(err) = persist_err {
                return postflight_terminal(layout, now, slice, commit, &err);
            }
        }
        Err(err) => {
            // Seam / dispatch / slice-mismatch — no report to persist.
            return postflight_terminal(layout, now, slice, commit, &err);
        }
    }
    journal::emit_best_effort(
        layout,
        now,
        EventKind::TargetMergeWaveSucceeded {
            target: commit.wave.target.clone(),
            digest: commit.digest.clone(),
            slice_name: slice.into(),
        },
        "slice.merge",
    );
    Ok(outcome)
}

/// Load the newest build record, revalidate its one-member wave, and
/// project the code patch merge still applies.
fn load_wave_commit(
    layout: Layout<'_>, slice: &str, slice_dir: &Path,
) -> Result<WaveCommit, Error> {
    let record = BuildRecord::load_latest(slice_dir)?;
    let config = ProjectConfig::load(layout.project_dir())?;
    let wave = Wave::load_for_merge(layout, &config.name, slice, &record)?;
    let digest = wave.digest()?.as_str().to_string();
    Ok(WaveCommit {
        wave,
        digest,
        patch: record.to_patch(),
    })
}

/// Prepare the read-only workspace view of the slice's result snapshot.
async fn prepare_view(
    workspaces: &impl Workspaces, slice: &str, patch: &CodePatch,
) -> Result<seam::Workspace, Error> {
    workspaces.prepare(patch.result.clone(), false).await.map_err(|err| Error::Diag {
        code: "target-merge-workspace-failed",
        detail: format!(
            "preparing the read-only result view for slice `{slice}` failed \
             (result `{}`): {err}",
            patch.result
        ),
    })
}

/// Interim apply (deleted by RFC-88): write the accepted patch's
/// touched paths onto the product tree — never a full-tree sync, so
/// the deterministic commit's own baseline fold stands.
async fn apply_result(
    workspaces: &impl Workspaces, slice: &str, patch: &CodePatch,
) -> Result<(), Error> {
    workspaces.apply(patch.clone()).await.map_err(|err| Error::Diag {
        code: "slice-merge-apply-failed",
        detail: format!(
            "applying result snapshot `{}` for merged slice `{slice}` failed after the \
             commit (the baseline, archive, and wave-committed fact stand): {err}",
            patch.result
        ),
    })
}

/// Append `target.merge.wave-committed` with identity maps.
///
/// Commit-authorization reuses the wave's build-authorization
/// (serial execution normally uses the covering `plan.execute.started`
/// epoch bound at wave open).
fn emit_wave_committed(
    layout: Layout<'_>, now: Timestamp, slice: &str, commit: &WaveCommit, maps: &[IdentityMap],
) -> Result<(), Error> {
    let auth = &commit.wave.build_authorization;
    journal::append_one(
        layout,
        &journal::Event::new(
            now,
            EventKind::TargetMergeWaveCommitted {
                target: commit.wave.target.clone(),
                digest: commit.digest.clone(),
                slice_name: slice.into(),
                commit_authorization: FactEpochRef {
                    writer: auth.writer.clone(),
                    sequence: auth.sequence,
                },
                identity_maps: maps.to_vec(),
            },
        ),
    )
}

/// Journal `target.merge.wave-postflight-failed` and return the terminal
/// non-rollback diagnostic.
///
/// The journal event is control-plane for sticky plan status (not
/// lifecycle observability), so the append is strict. A journal I/O
/// failure still returns `target-merge-postflight-failed` so execute
/// classifies correctly; the detail names the journal error too.
fn postflight_terminal(
    layout: Layout<'_>, now: Timestamp, slice: &str, commit: &WaveCommit, err: &Error,
) -> Result<MergeOutcome, Error> {
    let detail = format!(
        "target postflight merge gate failed for slice `{slice}` after the wave \
         committed — the baseline, archive, and `target.merge.wave-committed` fact stand \
         (non-rollback); inspect the archive `merge/postflight.yaml` when present \
         and land a follow-up slice: {err}"
    );
    let event = journal::Event::new(
        now,
        EventKind::TargetMergeWavePostflightFailed {
            target: commit.wave.target.clone(),
            digest: commit.digest.clone(),
            slice_name: slice.into(),
            reason: err.variant_str().into_owned(),
        },
    );
    if let Err(journal_err) = journal::append_one(layout, &event) {
        return Err(Error::Diag {
            code: "target-merge-postflight-failed",
            detail: format!("{detail}; also failed to journal postflight debt: {journal_err}"),
        });
    }
    Err(Error::Diag {
        code: "target-merge-postflight-failed",
        detail,
    })
}

/// Journal a `slice.merge.failed` terminal on the error arm — the
/// pre-commit phases share one failure event.
fn journal_on_failure<V>(
    layout: Layout<'_>, now: Timestamp, slice: &str, result: Result<V, Error>,
) -> Result<V, Error> {
    if let Err(err) = &result {
        journal::emit_best_effort(
            layout,
            now,
            EventKind::SliceMergeFailed {
                slice_name: slice.into(),
                reason: err.variant_str().into_owned(),
            },
            "slice.merge",
        );
    }
    result
}

/// Detect a torn merge left by a crash after the deterministic commit:
/// the slice tree is gone from `.emery/slices/` and its newest archive
/// carries `merged_at`. Returns the outcome without re-running merge
/// work. Detection is best-effort read-only — any unreadable archive
/// falls through to the normal merge path and its errors.
fn heal_torn_merge(layout: Layout<'_>, slice: &str) -> Option<MergeOutcome> {
    if layout.slice_dir(slice).exists() {
        return None;
    }
    let archive_path = latest_archive(&layout.archive_dir(), slice)?;
    let metadata = project::slice::SliceMetadata::load(&archive_path).ok()?;
    metadata.merged_at?;
    Some(MergeOutcome {
        merged: vec![],
        decisions: vec![],
        archive_path,
    })
}

/// The newest `<YYYY-MM-DD>-<slice>` folder under the archive root,
/// by the date prefix's lexicographic order.
fn latest_archive(archive_dir: &Path, slice: &str) -> Option<PathBuf> {
    const DATE_PREFIX_LEN: usize = "0000-00-00-".len();
    let mut best: Option<String> = None;
    for entry in std::fs::read_dir(archive_dir).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let dated_match = name.len() == DATE_PREFIX_LEN + slice.len()
            && name.ends_with(slice)
            && name.as_bytes().get(DATE_PREFIX_LEN - 1) == Some(&b'-');
        if dated_match && entry.path().is_dir() && best.as_deref() < Some(name.as_str()) {
            best = Some(name);
        }
    }
    best.map(|name| archive_dir.join(name))
}

/// Read-only completion preflight, run before the `slice.merge.*`
/// bracket and any baseline write: a plan-owned merge requires the
/// entry to project `in-progress` from the fact union (claim /
/// advance), so an absent or not-yet-advanced entry refuses here
/// instead of failing after the baseline and archive have already been
/// mutated. Standalone merges (no `plan.yaml`) skip the gate entirely.
fn preflight_completion(layout: Layout<'_>, slice: &str) -> Result<(), Error> {
    if !layout.plan_path().exists() {
        return Ok(());
    }
    let plan = Plan::load(&layout.plan_path())?;
    let Some(entry) = plan.entries.iter().find(|e| e.name == slice) else {
        return Err(plan.entry_not_found(slice));
    };
    let events = collect_events(layout)?;
    let ladders = project_ladders(&plan, &events);
    let status = ladders.get(&entry.name).copied().unwrap_or(Status::Pending);
    if status != Status::InProgress {
        return Err(Error::validation_failed(
            "slice-merge-entry-not-in-progress",
            "a plan-owned merge requires a projected `in-progress` entry",
            format!(
                "plan entry `{slice}` projects `{status}`; re-run `emery plan execute` — the \
                 loop claims the entry before merging"
            ),
        ));
    }
    Ok(())
}

/// Validator + apply core: commit the deltas, journal the skipped git
/// leg, and append the outcome-ledger entry (`slice.archive.created`).
fn commit_run(
    layout: Layout<'_>, now: Timestamp, slice: &str, allow_composition_replace: bool,
) -> Result<MergeOutcome, Error> {
    let slice_dir = layout.slice_dir(slice);
    let archive_dir = layout.archive_dir();
    let classes = artifact_classes(layout.project_dir(), &slice_dir);

    let merged =
        slice_merge::commit(&slice_dir, &classes, &archive_dir, now, allow_composition_replace)?;

    // D2: the guest owns no git surface, so the native workspace-clone
    // auto-commit leg is skipped — explicitly, so a journal reader can
    // tell a guest merge from a native merge that ran outside a clone.
    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceMergeCommitSkipped {
            slice_name: slice.into(),
        },
        "slice.merge",
    );

    emit_archive_created(layout, now, slice, &merged);

    let today = now.strftime("%Y-%m-%d").to_string();
    let archive_path = archive_dir.join(format!("{today}-{slice}"));
    Ok(MergeOutcome {
        merged: merged.specs,
        decisions: merged.decisions,
        archive_path,
    })
}

/// Append the durable `slice.archive.created` outcome-ledger entry.
/// `merge-sha` stays absent — there is no git leg in-guest.
/// Best-effort: a journal-write failure must not undo a committed
/// merge.
fn emit_archive_created(layout: Layout<'_>, now: Timestamp, slice: &str, merged: &MergeCommit) {
    let touched_specs: Vec<String> = merged.specs.iter().map(|e| e.name.clone()).collect();
    let outcome_summary = if merged.specs.is_empty() {
        "no baseline specs touched".to_string()
    } else {
        merged
            .specs
            .iter()
            .map(|e| {
                format!("{}: {}", e.name, crate::merge::summarise_operations(&e.result.operations))
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceArchiveCreated {
            slice_name: slice.into(),
            touched_specs,
            outcome_summary,
            merge_sha: None,
            decisions: merged.decisions.clone(),
        },
        "slice.archive.created",
    );
}
