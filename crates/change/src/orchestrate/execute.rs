//! The drained execute loop behind `emery plan execute`: advance to
//! the next entry, dispatch its phase (refine / build / merge), repeat
//! until the plan projects `drained` or a stop. Every stop returns as
//! a typed [`ExecuteOutcome::Stopped`] with a [`StopReason`] and hint.
//!
//! Dual-driving is refused by the create-exclusive [`GuestMarker`]
//! (`<plan-root>/.emery/guest.lock`) held for the run. The loop
//! composes the per-phase cadence and, on re-entry after a sticky
//! postflight stop, appends one control-plane
//! `plan.merge-postflight.acknowledged` event before continuing.

use std::ops::ControlFlow;

use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::Resolver;
use project::config::{Layout, ProjectConfig};
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind};
use project::plan::{LoopStep, NextActionKind, Plan, StatusBody, StopReason, plan_status_body};
use project::seam::{Source, Target, Workspaces};
use tracing::Instrument as _;

mod marker;

pub use marker::GuestMarker;

/// One phase the loop completed, in run order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseRun {
    pub slice: String,
    pub step: LoopStep,
}

/// How one [`execute`] run ended.
///
/// Both arms are successful returns — a stop is the loop's typed
/// refusal/halt surface (the skill's stop-conditions contract), not an
/// error; hard failures outside a phase (a corrupt plan, marker I/O)
/// surface as `Err` instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecuteOutcome {
    /// Every entry is `done` — the only clean exit.
    Drained {
        /// Plan name from `plan.yaml.name`.
        plan: String,
        /// Phases completed by this run, in order.
        phases: Vec<PhaseRun>,
    },
    /// The loop halted on a stop condition; re-entry safe. Refine /
    /// build / preflight-merge failures leave the plan entry
    /// `in-progress`; a postflight failure leaves the entry `done`
    /// (non-rollback) and projects a sticky stop until the next
    /// execute acknowledges.
    Stopped {
        /// Why the loop halted.
        reason: StopReason,
        /// Failure detail (the failing phase's error, or the status
        /// projection's journal overlay detail).
        detail: Option<String>,
        /// One-line operator hint for this stop.
        hint: &'static str,
        /// Slice the stop is parked on, when one is targeted.
        slice: Option<String>,
        /// Phases completed before the stop, in order.
        phases: Vec<PhaseRun>,
    },
}

/// Run the drained execute loop: advance → refine → build → merge
/// per entry until `plan status` projects `drained` or a stop.
///
/// Re-entry is safe — a refine / build / preflight-merge failure
/// leaves the entry `in-progress` and the journal terminal event in
/// place, so the next run's status projection resumes (or re-reports
/// the stop) from the same point. A postflight failure stamps the
/// entry `done` (non-rollback) and projects a sticky
/// `merge-postflight-failed` stop; re-running execute emits
/// `plan.merge-postflight.acknowledged` and continues.
///
/// The bound target adapter resolves once, inside the loop's own
/// setup (after the workspace refusal, before the marker) — its
/// declared inputs and its name feed every [`slice::orchestrate::build`] dispatch,
/// so the declared inputs and the seam routing come from one identity.
/// Each build and merge phase manages its own private workspace
/// through the target seam's `Workspaces` capability (RFC-87).
///
/// # Errors
///
/// Refuses with `plan-execute-workspace-unsupported` (exit 2) when the plan root
///   is a workspace or any entry is `project`-scoped — the skill's
///   workspace routing (slot sync + chdir) has no in-guest counterpart
///   yet, so the loop refuses rather than writing to the wrong tree.
///   Classified **before** the adapter lookup, so a workspace root
///   surfaces this refusal rather than `workspace-no-adapter`.
/// Refuses with `guest-marker-held` (exit 2) when another guest execute run holds
///   the D1 marker — or a stale marker survived a crash; the detail
///   says which file to delete.
/// Phase failures do **not** surface here — they return as
///   [`ExecuteOutcome::Stopped`].
pub async fn execute<P: Model, S: Source, T: Target + Workspaces, R: Resolver>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp,
) -> Result<ExecuteOutcome, Error> {
    let layout = Layout::new(paths.project_root());
    refuse_workspace_routing(layout)?;
    let config = ProjectConfig::load(layout.project_dir())?;
    let adapter = project::target_policy::project_adapter(caps.resolver, &config, paths)?;
    let _marker = GuestMarker::acquire(layout, now)?;
    let mut phases: Vec<PhaseRun> = Vec::new();

    loop {
        let plan = Plan::load(&layout.plan_path())?;
        let status = plan_status_body(&plan, layout)?;
        // Progress rendering: the active entry is the (done + 1)-th of the
        // plan's total, carried into the per-phase lines below.
        let counts = status.counts;
        let total = counts.pending + counts.in_progress + counts.done;
        let entry = (counts.done + 1).min(total.max(1));
        // A single execute process still walks entries one-by-one
        // (RFC-86 D23). When status already names an in-progress entry,
        // resume it — do not call advance, which would start a different
        // eligible pending sibling now that plan-wide single-active is
        // retired.
        let resume = status.active.clone();
        let step = match dispatch_status(layout, now, status, &phases) {
            ControlFlow::Break(outcome) => return Ok(outcome),
            ControlFlow::Continue(None) => continue, // postflight ack
            ControlFlow::Continue(Some(step)) => step,
        };

        let Some(advanced) = (match resume {
            Some(slice) => {
                let target = project::target_policy::resumed(layout, &slice).ok();
                Some(Advanced { slice, target })
            }
            None => advance(caps.resolver, paths, now)?,
        }) else {
            // The status projection targeted a phase but the advance
            // found nothing runnable — plan state moved underneath us.
            // Surface it as the stuck stop rather than spinning.
            return Ok(ExecuteOutcome::Stopped {
                reason: StopReason::Stuck,
                detail: None,
                hint: StopReason::Stuck.hint(),
                slice: None,
                phases,
            });
        };
        let slice = advanced.slice.clone();

        tracing::info!("{step} {slice} [entry {entry}/{total}] …");
        let span = tracing::info_span!("plan.execute.entry", slice = %slice, phase = %step);
        let result: Result<(), Error> = match step {
            LoopStep::Refine => {
                let target = advanced.target.clone().ok_or_else(|| Error::Diag {
                    code: "slice-create-target-missing",
                    detail: format!(
                        "no target resolved for slice `{slice}`; declare the project adapter \
                         (or fix the bound project's topology) before executing"
                    ),
                })?;
                slice::orchestrate::refine(caps, paths, now, &slice, &target)
                    .instrument(span)
                    .await
                    .map(drop)
            }
            LoopStep::Build => {
                slice::orchestrate::build(caps.targets, layout, now, &slice, &adapter.manifest)
                    .instrument(span)
                    .await
                    .map(drop)
            }
            LoopStep::Merge => slice::orchestrate::merge(caps.targets, layout, now, &slice, false)
                .instrument(span)
                .await
                .map(drop),
        };

        match result {
            Ok(()) => {
                tracing::info!("{step} {slice} [entry {entry}/{total}] — completed");
                phases.push(PhaseRun { slice, step });
            }
            Err(err) => {
                // The phase already journalled its failure terminal, so
                // a re-entrant run's status projection reports the same
                // stop this return carries. Refine / build / preflight
                // leave the entry `in-progress`; postflight already
                // stamped `done` (non-rollback) before failing.
                let reason = phase_stop_reason(step, &err);
                tracing::info!("{step} {slice} [entry {entry}/{total}] — stopped: {reason}");
                return Ok(ExecuteOutcome::Stopped {
                    reason,
                    detail: Some(err.to_string()),
                    hint: reason.hint(),
                    slice: Some(slice),
                    phases,
                });
            }
        }
    }
}

/// Map a status projection to the next loop step, a terminal outcome,
/// or a postflight-ack continue (`Continue(None)`).
fn dispatch_status(
    layout: Layout<'_>, now: Timestamp, status: StatusBody, phases: &[PhaseRun],
) -> ControlFlow<ExecuteOutcome, Option<LoopStep>> {
    match status.action {
        NextActionKind::Drained => ControlFlow::Break(ExecuteOutcome::Drained {
            plan: status.plan,
            phases: phases.to_vec(),
        }),
        NextActionKind::Stop => {
            // Sticky postflight debt: the first failure already stopped
            // via the phase Err arm. Re-running execute acknowledges
            // and continues — no new CLI verb. The ack is control-plane
            // (clears the sticky stop), so it must not be best-effort:
            // a swallowed append would leave status projecting the same
            // stop and spin while holding `guest.lock`.
            if status.stop.as_ref().map(|s| s.reason) == Some(StopReason::MergePostflightFailed) {
                return acknowledge_postflight(layout, now, status, phases);
            }
            // Torn merge (commit landed, `done` stamp missing): the
            // merge phase's re-entry heals it by stamping the entry,
            // so dispatch merge instead of parking the loop.
            if status.stop.as_ref().map(|s| s.reason) == Some(StopReason::MergeIncomplete) {
                return ControlFlow::Continue(Some(LoopStep::Merge));
            }
            // A stop projection always carries a stop body; a missing
            // one (unreachable by construction) degrades to the
            // generic stuck stop rather than panicking.
            let (reason, detail) =
                status.stop.map_or((StopReason::Stuck, None), |stop| (stop.reason, stop.detail));
            ControlFlow::Break(ExecuteOutcome::Stopped {
                reason,
                detail,
                hint: reason.hint(),
                slice: status.slice,
                phases: phases.to_vec(),
            })
        }
        NextActionKind::Refine => ControlFlow::Continue(Some(LoopStep::Refine)),
        NextActionKind::Build => ControlFlow::Continue(Some(LoopStep::Build)),
        NextActionKind::Merge => ControlFlow::Continue(Some(LoopStep::Merge)),
    }
}

/// Append `plan.merge-postflight.acknowledged` and continue the loop.
/// On append failure, re-surface the sticky stop (no spin).
fn acknowledge_postflight(
    layout: Layout<'_>, now: Timestamp, status: StatusBody, phases: &[PhaseRun],
) -> ControlFlow<ExecuteOutcome, Option<LoopStep>> {
    let Some(slice) = status.slice.as_deref() else {
        return ControlFlow::Break(ExecuteOutcome::Stopped {
            reason: StopReason::Stuck,
            detail: Some(
                "merge-postflight-failed stop projected without a slice — cannot acknowledge"
                    .into(),
            ),
            hint: StopReason::Stuck.hint(),
            slice: None,
            phases: phases.to_vec(),
        });
    };
    let event = Event::new(
        now,
        EventKind::PlanMergePostflightAcknowledged {
            slice_name: slice.into(),
        },
    );
    if let Err(err) = journal::append_one(layout, &event) {
        return ControlFlow::Break(ExecuteOutcome::Stopped {
            reason: StopReason::MergePostflightFailed,
            detail: Some(format!(
                "failed to journal plan.merge-postflight.acknowledged for `{slice}`: {err}"
            )),
            hint: StopReason::MergePostflightFailed.hint(),
            slice: status.slice,
            phases: phases.to_vec(),
        });
    }
    ControlFlow::Continue(None)
}

/// Classify a phase `Err` into the closed stop reason. Postflight is
/// distinguished from other merge failures by the error discriminant.
fn phase_stop_reason(step: LoopStep, err: &Error) -> StopReason {
    match step {
        LoopStep::Refine => StopReason::RefineFailed,
        LoopStep::Build => StopReason::BuildFailed,
        LoopStep::Merge if err.variant_str() == "target-merge-postflight-failed" => {
            StopReason::MergePostflightFailed
        }
        LoopStep::Merge => StopReason::MergeConflict,
    }
}

/// Refuse workspace-routed plans: a `project`-scoped entry needs a
/// slot sync plus a chdir into `workspace/<project>/`, and the guest
/// loop has no counterpart yet — running anyway would create slices
/// under the workspace root's own `.emery/` tree. Workspace plans
/// run hand-driven instead: `emery plan advance`, then the
/// `/emery:refine` → `/emery:build` → `/emery:merge` breakouts. Uses the
/// shared [`super::routing`] classification with this operation's own
/// refusal code; single-project plans are unaffected.
fn refuse_workspace_routing(layout: Layout<'_>) -> Result<(), Error> {
    let plan = Plan::load(&layout.plan_path())?;
    let Some(subject) = super::routing::classify(layout, Some(&plan))?.refusal_subject() else {
        return Ok(());
    };
    Err(Error::validation_failed(
        "plan-execute-workspace-unsupported",
        "the guest execute loop runs single-project plans only",
        format!(
            "{subject}; workspace routing (slot sync + chdir) has no in-guest counterpart — \
             drive workspace plans hand-driven (`emery plan advance`, then the \
             /emery:refine → /emery:build → /emery:merge breakouts)"
        ),
    ))
}

/// One advanced entry: the slice to run and its best-effort resolved
/// target (`name[@vN]`).
struct Advanced {
    slice: String,
    target: Option<String>,
}

/// Advance the plan through the shared
/// [`project::plan::advance_next`] kernel (see the module docs for
/// why `require_held` does not apply in-loop). Returns `None` when
/// nothing is runnable (drained / stuck — the status projection
/// decides which).
fn advance(
    resolver: &impl Resolver, paths: &ExecutionPaths, now: Timestamp,
) -> Result<Option<Advanced>, Error> {
    let layout = Layout::new(paths.project_root());
    let config = ProjectConfig::load(layout.project_dir())?;
    let body = project::plan::advance_next(resolver, paths, now, &config)?;
    // A fresh advance carries the resolved target; the active-entry
    // return does not, so re-resolve lazily from the slice's own
    // metadata at the phase (refine reads it from the advance, and
    // only refine needs it — build/merge read `metadata.yaml`
    // themselves).
    Ok(match (body.advanced, body.active) {
        (Some(slice), _) => Some(Advanced {
            slice,
            target: body.target,
        }),
        (None, Some(slice)) => {
            let target = project::target_policy::resumed(layout, &slice).ok();
            Some(Advanced { slice, target })
        }
        (None, None) => None,
    })
}
