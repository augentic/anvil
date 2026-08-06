//! The merge orchestrator: target merge gates around the deterministic
//! core merge.

use std::path::{Path, PathBuf};

use error::Error;
use jiff::Timestamp;
use project::config::Layout;
use project::journal::{self, EventKind};
use project::plan::{Plan, Status, collect_events, project_ladders};
use project::seam::{self, MergePhase, Target, Workspaces};
use project::snapshot::CodePatch;

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
/// (archive fact projects `done`) → target postflight gate, with each
/// gate's report schema-gated and persisted. Both gates read the built
/// result code through one read-only private-workspace view of the
/// slice's captured result snapshot (`build/patch.yaml`); after a
/// successful postflight, the interim apply writes the patch's
/// touched paths onto the product tree (journal-visible; deleted when
/// RFC-89 publication sets own the final seal).
///
/// A preflight failure aborts with the slice still built; a
/// postflight failure (`target-merge-postflight-failed`) is terminal
/// but non-rollback — the merge stands. A parseable postflight report
/// is persisted to the archive (including `status: failure`) before
/// the terminal error returns.
///
/// Re-entry heals a torn merge: when the deterministic commit already
/// landed (the slice tree is archived with `merged_at`) the run
/// returns without a second baseline merge or gate dispatch.
///
/// # Errors
///
/// Completion-gate and preflight failures (slice not built / not
/// claimed in-progress, `target-merge-preflight-failed`), the terminal
/// `target-merge-postflight-failed`, plus commit and archive I/O
/// failures.
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
    // The captured code patch, read before the deterministic commit
    // moves the slice tree into the archive.
    let patch = load_patch(&slice_dir, slice)?;

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
    let view = journal_on_failure(layout, now, slice, prepare_view(targets, slice, &patch).await)?;
    let run =
        gated(targets, layout, now, slice, &slice_dir, &id, allow_composition_replace, &view).await;
    if let Err(err) = targets.discard(view.id.clone()).await {
        tracing::warn!(workspace = %view.id, "merge view discard failed: {err}");
    }
    let outcome = run?;

    // Interim code delivery (deleted by RFC-89): the postflight gate
    // passed, so materialize the accepted result snapshot onto the
    // product tree and journal the apply.
    journal_on_failure(layout, now, slice, apply_result(targets, slice, &patch).await)?;
    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceCodeApplied {
            slice_name: slice.into(),
            snapshot: patch.result.to_string(),
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

/// The gate-bracketed core: preflight → deterministic commit →
/// postflight, all over the shared read-only `view`. Split from
/// [`merge`] so the caller can discard the view on every exit.
#[expect(
    clippy::too_many_arguments,
    reason = "internal merge kernel bracketed by the view lifecycle; callers use `merge`"
)]
async fn gated<T: Target>(
    targets: &T, layout: Layout<'_>, now: Timestamp, slice: &str, slice_dir: &Path, id: &str,
    allow_composition_replace: bool, view: &seam::Workspace,
) -> Result<MergeOutcome, Error> {
    // Target preflight: a failure aborts with the slice still `built`.
    let preflight = run_gate(targets, id, slice, MergePhase::Preflight, view).await;
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
    match fetch_gate_report(targets, id, slice, MergePhase::Postflight, view).await {
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
    Ok(outcome)
}

/// Load the code patch `slice build` captured beside its report.
fn load_patch(slice_dir: &Path, slice: &str) -> Result<CodePatch, Error> {
    let path = slice_dir.join("build").join("patch.yaml");
    if !path.is_file() {
        return Err(Error::validation_failed(
            "slice-merge-patch-missing",
            "a built slice carries its captured code patch",
            format!(
                "slice `{slice}` has no `build/patch.yaml`; re-run `emery slice build {slice}` \
                 before merging"
            ),
        ));
    }
    Ok(serde_saphyr::from_str(&project::fs::read_text(&path)?)?)
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

/// Interim apply (deleted by RFC-89): write the accepted patch's
/// touched paths onto the product tree — never a full-tree sync, so
/// the deterministic commit's own baseline fold stands.
async fn apply_result(
    workspaces: &impl Workspaces, slice: &str, patch: &CodePatch,
) -> Result<(), Error> {
    workspaces.apply(patch.clone()).await.map_err(|err| Error::Diag {
        code: "slice-merge-apply-failed",
        detail: format!(
            "applying result snapshot `{}` for merged slice `{slice}` failed after the \
             commit (the baseline, archive, and plan stamp stand): {err}",
            patch.result
        ),
    })
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
    let events = collect_events(&plan, layout)?;
    let ladders = project_ladders(&plan, &events);
    let status = ladders.get(&entry.name).copied().unwrap_or(Status::Pending);
    if status != Status::InProgress {
        return Err(Error::validation_failed(
            "slice-merge-entry-not-in-progress",
            "a plan-owned merge requires a projected `in-progress` entry",
            format!(
                "plan entry `{slice}` projects `{status}`; advance it with `emery plan advance` \
                 before merging"
            ),
        ));
    }
    Ok(())
}

/// Validator + apply core: commit the deltas, journal the skipped git
/// leg, and append the outcome-ledger entry (`slice.archive.created`
/// projects plan-entry `done`).
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
