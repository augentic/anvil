//! The drained execute loop behind `emery plan execute`: advance,
//! dispatch the entry's phase (build / merge), repeat until `drained`
//! or a typed stop. Execute never refines (RFC-91 D5).

use std::ops::ControlFlow;

use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::Resolver;
use project::config::Layout;
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind};
use project::plan::{LoopStep, NextActionKind, Plan, StatusBody, StopReason, plan_status_body};
use project::seam::{PhaseSource, Source, Target, Workspaces};
use tracing::Instrument as _;

mod marker;

pub use marker::GuestMarker;

/// One phase the loop completed, in run order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseRun {
    pub slice: String,
    pub step: LoopStep,
    /// The terminal verification report's assurance source — carried
    /// for build steps even on a clean pass (RFC-90 D3).
    pub verification: Option<PhaseSource>,
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
/// Re-entry is safe: a refine / build / preflight-merge failure leaves
/// the entry `in-progress`, so the next run resumes (or re-reports the
/// stop); a postflight failure stamps `done` (non-rollback) and
/// projects a sticky stop the next execute acknowledges. The bound
/// target adapter resolves once in loop setup (before the marker),
/// giving every dispatch one identity.
///
/// # Errors
///
/// Refuses with `guest-marker-held` (exit 2) when another guest execute run holds
///   the marker — or a stale marker survived a crash; the detail
///   says which file to delete.
/// Phase failures do **not** surface here — they return as
///   [`ExecuteOutcome::Stopped`].
pub async fn execute<P: Model, S: Source, T: Target + Workspaces, R: Resolver>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp,
) -> Result<ExecuteOutcome, Error> {
    let layout = paths.layout();
    if !paths.is_detached() {
        drop(project::config::ProjectConfig::load(paths.project_root())?);
    }
    let plan = Plan::load(&layout.plan_path())?;
    let _marker = GuestMarker::acquire(layout, now)?;
    // A drained plan is a read-only no-op: opening a fresh
    // authorization epoch would journal coverage nothing runs under.
    let status = plan_status_body(&plan, layout)?;
    if status.action == NextActionKind::Drained {
        return Ok(ExecuteOutcome::Drained {
            plan: status.plan,
            phases: Vec::new(),
        });
    }
    // Digest chain, then `plan.execute.started` with typed coverage.
    super::epoch::open(paths, &plan, now)?;
    let mut phases: Vec<PhaseRun> = Vec::new();

    loop {
        let plan = Plan::load(&layout.plan_path())?;
        let status = plan_status_body(&plan, layout)?;
        // Progress rendering: the active entry is the (done + 1)-th of the
        // plan's total, carried into the per-phase lines below.
        let counts = status.counts;
        let total = counts.pending + counts.in_progress + counts.done;
        let entry = (counts.done + 1).min(total.max(1));
        // A single execute process walks entries one-by-one. When status
        // already names an in-progress entry, resume it — advance would
        // start a different eligible pending sibling instead.
        let resume = status.active.clone();
        let step = match dispatch_status(layout, now, status, &phases) {
            ControlFlow::Break(outcome) => return Ok(outcome),
            ControlFlow::Continue(None) => continue, // postflight ack
            ControlFlow::Continue(Some(step)) => step,
        };

        let Some(slice) = (match resume {
            Some(slice) => Some(slice),
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
        // Build staleness (RFC-86a D4): rebuild when deferred set drifted
        // from the build record or the newest wave's rebuild failed.
        // Pin/source drift is refinement's (RFC-91); execute never re-refines.
        let step = if step == LoopStep::Merge
            && slice::dispositions_drifted(layout, &layout.slice_dir(&slice), &slice)?
        {
            tracing::info!(
                "deferred dispositions drifted for {slice} — re-building under this epoch"
            );
            LoopStep::Build
        } else {
            step
        };

        tracing::info!("{step} {slice} [entry {entry}/{total}] …");
        // Epoch freshness gates build before the target orchestration
        // (`plan-epoch-stale`); open gaps are dispositioned at the
        // gate itself (gate-time deferrals) and never block.
        if step == LoopStep::Build {
            let plan = Plan::load(&layout.plan_path())?;
            super::enforce_before_build(layout, &plan, &slice, now)?;
        }
        let result = run_phase(caps, paths, now, step, &slice).await;

        match result {
            Ok(verification) => {
                tracing::info!("{step} {slice} [entry {entry}/{total}] — completed");
                phases.push(PhaseRun {
                    slice,
                    step,
                    verification,
                });
            }
            Err(err) => {
                // The phase already journalled its failure terminal, so a
                // re-entrant run reports this same stop. Refine / build /
                // preflight leave `in-progress`; postflight stamped `done`.
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

/// Dispatch one loop phase for `slice` under the entry's tracing
/// span; a completed build step returns its terminal verification
/// source.
async fn run_phase<P: Model, S: Source, T: Target + Workspaces, R: Resolver>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp,
    step: LoopStep, slice: &str,
) -> Result<Option<PhaseSource>, Error> {
    let layout = paths.layout();
    let span = tracing::info_span!("plan.execute.entry", slice = %slice, phase = %step);
    match step {
        LoopStep::Refine => {
            unreachable!("dispatch_status never yields Refine — execute never refines (RFC-91 D5)")
        }
        LoopStep::Build => {
            let adapter = entry_adapter(caps.resolver, paths, slice)?;
            Box::pin(
                slice::orchestrate::build(caps.targets, layout, now, slice, &adapter.manifest)
                    .instrument(span),
            )
            .await
            .map(|outcome| outcome.verification)
        }
        LoopStep::Merge => {
            // The composition-replace override lives on the plan
            // entry (`emery plan amend --allow-composition-replace`),
            // read fresh so a mid-run amend takes effect.
            let plan = Plan::load(&layout.plan_path())?;
            let allow_replace = plan
                .entries
                .iter()
                .find(|entry| entry.name == slice)
                .is_some_and(|entry| entry.allow_composition_replace);
            slice::orchestrate::merge(caps.targets, layout, now, slice, allow_replace)
                .instrument(span)
                .await
                .map(|_| None)
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
            // Sticky postflight debt: re-running execute acknowledges and
            // continues. The ack must not be best-effort — a swallowed
            // append would re-project the stop and spin under `guest.lock`.
            if status.stop.as_ref().map(|s| s.reason) == Some(StopReason::MergePostflightFailed) {
                return acknowledge_postflight(layout, now, status, phases);
            }
            // Journalled phase failures are retryable — re-dispatch the
            // parked phase (this run's failures exit via the `Err`
            // arm). A parked refine is not ours: execute never refines.
            match status.stop.as_ref().map(|s| s.reason) {
                Some(StopReason::RefineFailed) => {
                    return ControlFlow::Break(refinement_required(status, phases));
                }
                Some(StopReason::BuildFailed) => {
                    return ControlFlow::Continue(Some(LoopStep::Build));
                }
                // Torn merge (commit landed, `done` stamp missing) heals
                // on merge re-entry; a preflight conflict retries after
                // the operator fixed inputs.
                Some(StopReason::MergeConflict | StopReason::MergeIncomplete) => {
                    return ControlFlow::Continue(Some(LoopStep::Merge));
                }
                _ => {}
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
        // Execute never refines (RFC-91 D5): a projected refine action
        // is the typed refinement-required stop pointing at the drain.
        NextActionKind::Refine => ControlFlow::Break(refinement_required(status, phases)),
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
        EventKind::PostflightAcknowledged {
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
        LoopStep::Refine => StopReason::RefinementRequired,
        LoopStep::Build => StopReason::BuildFailed,
        LoopStep::Merge if err.variant_str() == "target-merge-postflight-failed" => {
            StopReason::MergePostflightFailed
        }
        LoopStep::Merge => StopReason::MergeConflict,
    }
}

/// The typed refinement-required stop: execute reached an entry
/// without a fresh refinement manifest and never refines (RFC-91 D5).
fn refinement_required(status: StatusBody, phases: &[PhaseRun]) -> ExecuteOutcome {
    let detail = status.stop.and_then(|stop| stop.detail).or_else(|| {
        status
            .slice
            .as_ref()
            .map(|slice| format!("slice `{slice}` has no fresh refinement manifest"))
    });
    ExecuteOutcome::Stopped {
        reason: StopReason::RefinementRequired,
        detail,
        hint: StopReason::RefinementRequired.hint(),
        slice: status.slice,
        phases: phases.to_vec(),
    }
}

/// Advance the plan through the shared
/// [`project::plan::advance_next`] kernel (see the module docs for
/// why `require_held` does not apply in-loop). Returns the claimed or
/// active slice, or `None` when nothing is runnable (drained / stuck —
/// the status projection decides which).
fn advance(
    resolver: &impl Resolver, paths: &ExecutionPaths, now: Timestamp,
) -> Result<Option<String>, Error> {
    let body = project::plan::advance_next(resolver, paths, now)?;
    Ok(body.advanced.or(body.active))
}

fn entry_adapter(
    resolver: &impl Resolver, paths: &ExecutionPaths, slice: &str,
) -> Result<project::adapter::ResolvedTarget, Error> {
    let layout = paths.layout();
    let plan = Plan::load(&layout.plan_path())?;
    let entry = plan
        .entries
        .iter()
        .find(|entry| entry.name.as_str() == slice)
        .ok_or_else(|| plan.entry_not_found(slice))?;
    let binding = plan.target(&entry.target)?;
    resolver.resolve_target(&binding.adapter.selector(), paths)
}
