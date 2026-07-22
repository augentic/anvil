//! The merge orchestrator: target merge gates around the deterministic
//! core merge.

use std::path::{Path, PathBuf};

use artifacts::atomic::bytes_write;
use error::Error;
use jiff::Timestamp;
use project::config::{Layout, Mutation, with_state};
use project::journal::{self, EventKind};
use project::plan::{Plan, Status};
use project::seam::{MergePhase, Target, WorkingTree};

use super::seam_failure;
use crate::merge::{MergeCommit, PreviewEntry, artifact_classes, slice as slice_merge};
use crate::{BuildReport, BuildStatus};

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

/// Merge one built slice (`specify slice merge run`).
///
/// Runs target preflight gate → deterministic `slice_merge::commit`
/// → plan entry `done` → target postflight gate, with each gate's
/// report schema-gated and persisted.
///
/// A preflight failure aborts with the slice still `built`; a
/// postflight failure (`target-merge-postflight-failed`) is terminal
/// but non-rollback — the merge stands.
///
/// # Errors
///
/// Completion-gate and preflight failures (slice not `built`,
/// `target-merge-preflight-failed`), the terminal
/// `target-merge-postflight-failed`, plus commit, plan-stamp, and
/// archive I/O failures.
pub async fn merge<T: Target>(
    targets: &T, layout: Layout<'_>, now: Timestamp, slice: &str, allow_composition_replace: bool,
) -> Result<MergeOutcome, Error> {
    preflight_completion(layout, slice)?;
    let slice_dir = layout.slice_dir(slice);
    // The recorded slice target keeps its `name@version` pin: the
    // routed id dispatches the exact identity, never a reduced bare
    // name.
    let target = project::target_policy::resumed(layout, slice)?;
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
    // failure is a terminal diagnostic — never a rollback.
    match run_gate(targets, &id, slice, MergePhase::Postflight).await {
        Ok(report) => {
            persist_gate_report(
                &outcome.archive_path.join("merge"),
                MergePhase::Postflight,
                &report,
            )?;
        }
        Err(err) => {
            journal::emit_best_effort(
                layout,
                now,
                EventKind::SliceMergePostflightFailed {
                    slice_name: slice.into(),
                    reason: err.variant_str().into_owned(),
                },
                "slice.merge",
            );
            return Err(Error::Diag {
                code: "target-merge-postflight-failed",
                detail: format!(
                    "target postflight merge gate failed for slice `{slice}` after the merge \
                     committed — the baseline, archive, and plan entry `done` stamp stand \
                     (non-rollback); inspect the diagnostic and land a follow-up slice: {err}"
                ),
            });
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
    Ok(outcome)
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

/// Dispatch one target merge gate and gate its report: slice-name
/// match, blocking findings, and status.
async fn run_gate<T: Target>(
    targets: &T, id: &str, slice: &str, phase: MergePhase,
) -> Result<BuildReport, Error> {
    let report = targets
        .merge(id.to_string(), slice.to_string(), phase, WorkingTree::live())
        .await
        .map_err(|err| seam_failure("merge", id, &err))?;

    if report.slice != slice {
        return Err(Error::validation_failed(
            "target-merge-report-slice-mismatch",
            "the merge gate report's slice matches the slice being merged",
            format!("report names slice `{}`, but the merge ran for `{slice}`", report.slice),
        ));
    }
    report.enforce_no_blocking()?;
    if report.status == BuildStatus::Failure {
        return Err(Error::Diag {
            code: match phase {
                MergePhase::Preflight => "target-merge-preflight-failed",
                MergePhase::Postflight => "target-merge-postflight-failed",
            },
            detail: format!(
                "target `{}` reported a failed {phase} merge gate for slice `{slice}` ({} \
                 finding(s))",
                report.target,
                report.findings.len()
            ),
        });
    }
    Ok(report)
}

/// Persist one gate's validated report to `<dir>/<phase>.yaml`, so the
/// archived slice carries both gate outcomes.
fn persist_gate_report(dir: &Path, phase: MergePhase, report: &BuildReport) -> Result<(), Error> {
    std::fs::create_dir_all(dir).map_err(Error::Io)?;
    let yaml = project::fs::yaml(report)?;
    bytes_write(&dir.join(format!("{phase}.yaml")), yaml.as_bytes())
}

/// Read-only completion preflight, run before the `slice.merge.*`
/// bracket and any baseline write: a plan-owned merge must be able to
/// stamp its entry `done` (`in-progress → done` is the only legal
/// edge), so an absent or unclaimed entry refuses here instead of
/// failing after the baseline and archive have already been mutated.
/// Standalone merges (no `plan.yaml`) skip the gate entirely.
fn preflight_completion(layout: Layout<'_>, slice: &str) -> Result<(), Error> {
    if !layout.plan_path().exists() {
        return Ok(());
    }
    let plan = Plan::load(&layout.plan_path())?;
    let Some(entry) = plan.entries.iter().find(|e| e.name == slice) else {
        return Err(Error::Diag {
            code: "plan-entry-not-found",
            detail: format!("no slice named '{slice}' in plan"),
        });
    };
    if entry.status != Status::InProgress {
        return Err(Error::validation_failed(
            "slice-merge-entry-not-in-progress",
            "a plan-owned merge stamps its entry `done` from `in-progress`",
            format!(
                "plan entry `{slice}` is `{}`; claim it with `specify plan next` before merging",
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
