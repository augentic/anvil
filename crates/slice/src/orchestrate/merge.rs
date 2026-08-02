//! The merge orchestrator: target merge gates around the deterministic
//! core merge.

use std::path::{Path, PathBuf};

use error::Error;
use jiff::Timestamp;
use project::config::{Layout, Mutation, with_state};
use project::journal::{self, EventKind};
use project::plan::{Plan, Status};
use project::seam::{MergePhase, Target};

use crate::merge::{MergeCommit, PreviewEntry, artifact_classes, slice as slice_merge};

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

/// Merge one built slice (`emery slice merge`).
///
/// Runs target preflight gate → deterministic `slice_merge::commit`
/// → plan entry `done` → target postflight gate, with each gate's
/// report schema-gated and persisted.
///
/// A preflight failure aborts with the slice still `built`; a
/// postflight failure (`target-merge-postflight-failed`) is terminal
/// but non-rollback — the merge stands. A parseable postflight report
/// is persisted to the archive (including `status: failure`) before
/// the terminal error returns.
///
/// Re-entry heals a torn merge: when the deterministic commit already
/// landed (the slice tree is archived at lifecycle `merged`) but the
/// per-entry `done` stamp is missing, the run stamps the entry and
/// returns without a second baseline merge or gate dispatch.
///
/// # Errors
///
/// Completion-gate and preflight failures (slice not `built`,
/// `target-merge-preflight-failed`), the terminal
/// `target-merge-postflight-failed`, plus commit, plan-stamp, and
/// archive I/O failures.
#[tracing::instrument(
    name = "slice.merge",
    skip_all,
    fields(slice = %slice, target = tracing::field::Empty)
)]
pub async fn merge<T: Target>(
    targets: &T, layout: Layout<'_>, now: Timestamp, slice: &str, allow_composition_replace: bool,
) -> Result<MergeOutcome, Error> {
    tracing::info!("merge started");
    preflight_completion(layout, slice)?;
    if let Some(outcome) = heal_torn_merge(layout, slice) {
        stamp_plan_entry_done(layout, slice)?;
        tracing::info!("merge completed: torn merge healed, entry stamped done");
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

    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceMergeStarted {
            slice_name: slice.into(),
        },
        "slice.merge",
    );

    // Target preflight: a failure aborts with the slice still `built`.
    let preflight = run_gate(targets, &id, slice, MergePhase::Preflight).await;
    let preflight = journal_on_failure(layout, now, slice, preflight)?;
    persist_gate_report(&slice_dir.join("merge"), MergePhase::Preflight, &preflight)?;

    // The deterministic core: validators, spec fold, Decision Record
    // promotion, lifecycle, archive, and the plan entry's `done` stamp.
    let outcome = journal_on_failure(
        layout,
        now,
        slice,
        commit_run(layout, now, slice, allow_composition_replace),
    )?;

    // Target postflight: the slice is already merged and archived, so a
    // failure is a terminal diagnostic — never a rollback. Persist any
    // parseable report (including `status: failure`) before enforcing.
    // Every post-commit error routes through `postflight_terminal` so
    // execute classifies sticky `merge-postflight-failed` debt — a bare
    // `?` on persist would otherwise surface as `merge-conflict`.
    let archive_merge = outcome.archive_path.join("merge");
    match fetch_gate_report(targets, &id, slice, MergePhase::Postflight).await {
        Ok(report) => {
            let persist_err =
                persist_gate_report(&archive_merge, MergePhase::Postflight, &report).err();
            if let Err(err) = enforce_gate(&report, MergePhase::Postflight, slice) {
                return postflight_terminal(layout, now, slice, &err);
            }
            if let Some(err) = persist_err {
                return postflight_terminal(layout, now, slice, &err);
            }
        }
        Err(err) => {
            // Seam / dispatch / slice-mismatch — no report to persist.
            return postflight_terminal(layout, now, slice, &err);
        }
    }

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

/// Journal `slice.merge.postflight-failed` and return the terminal
/// non-rollback diagnostic.
///
/// The journal event is control-plane for sticky plan status (not
/// lifecycle observability), so the append is strict. A journal I/O
/// failure still returns `target-merge-postflight-failed` so execute
/// classifies correctly; the detail names the journal error too.
fn postflight_terminal(
    layout: Layout<'_>, now: Timestamp, slice: &str, err: &Error,
) -> Result<MergeOutcome, Error> {
    let detail = format!(
        "target postflight merge gate failed for slice `{slice}` after the merge \
         committed — the baseline, archive, and plan entry `done` stamp stand \
         (non-rollback); inspect the archive `merge/postflight.yaml` when present \
         and land a follow-up slice: {err}"
    );
    let event = journal::Event::new(
        now,
        EventKind::SliceMergePostflightFailed {
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

/// Detect a torn merge left by a crash between the deterministic
/// commit and the per-entry `done` stamp: the slice tree is gone from
/// `.emery/slices/` and its newest archive reads lifecycle `merged`.
/// Returns the outcome to hand back after the caller re-stamps the
/// entry; the baseline fold and archive stand, so no merge work (and
/// no gate dispatch) re-runs. Detection is best-effort read-only —
/// any unreadable archive falls through to the normal merge path and
/// its errors.
fn heal_torn_merge(layout: Layout<'_>, slice: &str) -> Option<MergeOutcome> {
    if layout.slice_dir(slice).exists() {
        return None;
    }
    let archive_path = latest_archive(&layout.archive_dir(), slice)?;
    let metadata = project::slice::SliceMetadata::load(&archive_path).ok()?;
    if metadata.status != project::slice::LifecycleStatus::Merged {
        return None;
    }
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
/// bracket and any baseline write: a plan-owned merge must be able to
/// stamp its entry `done` (`in-progress → done` is the only legal
/// edge), so an absent or not-yet-advanced entry refuses here instead
/// of failing after the baseline and archive have already been mutated.
/// Standalone merges (no `plan.yaml`) skip the gate entirely.
fn preflight_completion(layout: Layout<'_>, slice: &str) -> Result<(), Error> {
    if !layout.plan_path().exists() {
        return Ok(());
    }
    let plan = Plan::load(&layout.plan_path())?;
    let Some(entry) = plan.entries.iter().find(|e| e.name == slice) else {
        return Err(plan.entry_not_found(slice));
    };
    if entry.status != Status::InProgress {
        return Err(Error::validation_failed(
            "slice-merge-entry-not-in-progress",
            "a plan-owned merge stamps its entry `done` from `in-progress`",
            format!(
                "plan entry `{slice}` is `{}`; advance it with `emery plan advance` before merging",
                entry.status
            ),
        ));
    }
    Ok(())
}

/// Validator + apply core: commit the deltas, journal the skipped git
/// leg, append the outcome-ledger entry, and stamp the plan entry
/// `done`.
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

    stamp_plan_entry_done(layout, slice)?;

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

/// workflow §Workflow: the merge step is the sole writer of per-entry
/// `done`. Standalone merges without `plan.yaml` skip this step
/// silently, matching the native verb. [`preflight_completion`]
/// guarantees the entry exists and is `in-progress` before any merge
/// write; `Plan::transition` re-checks the edge on the re-read state.
fn stamp_plan_entry_done(layout: Layout<'_>, slice: &str) -> Result<(), Error> {
    if !layout.plan_path().exists() {
        return Ok(());
    }
    with_state::<Plan, _, _>(layout, "plan.yaml", move |plan| {
        plan.transition(slice, Status::Done)?;
        Ok(Mutation::changed(()))
    })?;
    Ok(())
}
