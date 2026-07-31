//! The drained execute loop behind `emery plan execute`: claim the
//! next entry, dispatch its phase (refine / build / merge), repeat
//! until the plan projects `drained` or a stop. Every stop returns as
//! a typed [`ExecuteOutcome::Stopped`] with a [`StopReason`] and hint.
//!
//! Dual-driving is refused by the create-exclusive [`GuestMarker`]
//! (`<plan-root>/.emery/guest.lock`) held for the run. The loop
//! composes the per-phase cadence and, on re-entry after a sticky
//! postflight stop, appends one control-plane
//! `plan.merge-postflight.acknowledged` event before continuing.

use std::io::Write as _;
use std::ops::ControlFlow;
use std::path::PathBuf;

use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::Resolver;
use project::config::{Layout, ProjectConfig};
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind};
use project::plan::{LoopStep, NextActionKind, Plan, StatusBody, StopReason, plan_status_body};
use project::seam::{Source, Target, WorkingTree};
use tracing::Instrument as _;

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

/// Run the drained execute loop: claim → refine → build → merge per
/// entry until `plan status` projects `drained` or a stop.
///
/// Invoking execute is the Gate 1 approval act: a `pending` plan is
/// stamped `approved` (one `plan.transition.approved` journal event
/// carrying `actor`) before the first status projection; an already
/// `approved` plan stamps nothing. Re-entry is safe — a refine /
/// build / preflight-merge failure leaves the entry `in-progress` and
/// the journal terminal event in place, so the next run's status
/// projection resumes (or re-reports the stop) from the same point.
/// A postflight failure stamps the entry `done` (non-rollback) and
/// projects a sticky `merge-postflight-failed` stop; re-running
/// execute emits `plan.merge-postflight.acknowledged` and continues.
///
/// The bound target adapter resolves once, inside the loop's own
/// setup (after the workspace refusal, before the marker) — its
/// declared inputs and its name feed every [`slice::orchestrate::build`] dispatch,
/// so the declared inputs and the seam routing come from one identity.
/// `tree` names the snapshot builds apply against (today's deployments
/// share one live tree).
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
pub async fn execute<P: Model, S: Source, T: Target, R: Resolver>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp,
    tree: &WorkingTree, actor: journal::Actor,
) -> Result<ExecuteOutcome, Error> {
    let layout = Layout::new(paths.project_root());
    refuse_workspace_routing(layout)?;
    let config = ProjectConfig::load(layout.project_dir())?;
    let adapter = project::target_policy::project_adapter(caps.resolver, &config, paths)?;
    let _marker = GuestMarker::acquire(layout, now)?;
    // Gate 1: invoking execute is the approval act. Stamp before the
    // first status projection; no-op when already approved.
    project::plan::stamp_approved(layout, now, actor)?;
    let mut phases: Vec<PhaseRun> = Vec::new();

    loop {
        let plan = Plan::load(&layout.plan_path())?;
        let status = plan_status_body(&plan, layout)?;
        let step = match dispatch_status(layout, now, status, &phases) {
            ControlFlow::Break(outcome) => return Ok(outcome),
            ControlFlow::Continue(None) => continue, // postflight ack
            ControlFlow::Continue(Some(step)) => step,
        };

        // Claim: `plan next` before every phase, exactly as the skill
        // drives it (returns the active entry unchanged mid-slice).
        let Some(claim) = claim_next(caps.resolver, paths, now)? else {
            // The status projection targeted a phase but the claim
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
        let slice = claim.slice.clone();

        let span = tracing::info_span!("plan.execute.entry", slice = %slice, phase = %step);
        let result: Result<(), Error> = match step {
            LoopStep::Refine => {
                let target = claim.target.clone().ok_or_else(|| Error::Diag {
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
            LoopStep::Build => slice::orchestrate::build(
                caps.targets,
                layout,
                now,
                &slice,
                &adapter.manifest,
                tree.clone(),
            )
            .instrument(span)
            .await
            .map(drop),
            LoopStep::Merge => slice::orchestrate::merge(caps.targets, layout, now, &slice, false)
                .instrument(span)
                .await
                .map(drop),
        };

        match result {
            Ok(()) => phases.push(PhaseRun { slice, step }),
            Err(err) => {
                // The phase already journalled its failure terminal, so
                // a re-entrant run's status projection reports the same
                // stop this return carries. Refine / build / preflight
                // leave the entry `in-progress`; postflight already
                // stamped `done` (non-rollback) before failing.
                let reason = phase_stop_reason(step, &err);
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
/// run hand-driven instead: `emery plan next`, then the
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
             drive workspace plans hand-driven (`emery plan next`, then the \
             /emery:refine → /emery:build → /emery:merge breakouts)"
        ),
    ))
}

/// One claimed entry: the slice to run and its best-effort resolved
/// target (`name[@vN]`).
struct Claim {
    slice: String,
    target: Option<String>,
}

/// Claim the next entry through the shared
/// [`project::plan::claim_next`] kernel (see the module docs for why
/// `require_held` does not apply in-loop). Returns `None` when
/// nothing is runnable (drained / stuck — the status projection
/// decides which).
fn claim_next(
    resolver: &impl Resolver, paths: &ExecutionPaths, now: Timestamp,
) -> Result<Option<Claim>, Error> {
    let layout = Layout::new(paths.project_root());
    let config = ProjectConfig::load(layout.project_dir())?;
    let body = project::plan::claim_next(resolver, paths, now, &config)?;
    // A fresh advance carries the resolved target; the active-entry
    // return does not, so re-resolve lazily from the slice's own
    // metadata at the phase (refine reads it from the claim, and only
    // refine needs it — build/merge read `metadata.yaml` themselves).
    Ok(match (body.next, body.active) {
        (Some(slice), _) => Some(Claim {
            slice,
            target: body.target,
        }),
        (None, Some(slice)) => {
            let target = project::target_policy::resumed(layout, &slice).ok();
            Some(Claim { slice, target })
        }
        (None, None) => None,
    })
}

/// The D1 create-exclusive advisory marker at
/// `<plan-root>/.emery/guest.lock`, held for one guest execute run.
///
/// `OpenOptions::create_new` makes acquisition atomic — exactly one
/// guest execute loop can hold the marker per plan root, so a second
/// in-guest `plan execute` is refused (`guest-marker-held`, exit 2)
/// while a run is live. The file body carries pid / hostname /
/// acquired-at as diagnostics only; existence is the lock.
///
/// **Staleness posture**: the marker is removed when the guard drops
/// (clean exit *and* phase-stop returns *and* error unwinds — any exit
/// that runs destructors). A crash that skips destructors leaves the
/// marker behind, and the next acquire refuses with a detail telling
/// the operator to delete the file after confirming no run is live.
/// No pid-liveness probe: WASI gives the guest no process table to
/// check a recorded pid against, so self-healing would be a guess.
///
/// This marker is the only execute-run interlock.
#[derive(Debug)]
pub struct GuestMarker {
    path: PathBuf,
}

impl GuestMarker {
    /// Atomically create the marker, stamping holder diagnostics into
    /// the body.
    ///
    /// # Errors
    ///
    /// - `guest-marker-held` (exit 2) when the marker already exists —
    ///   a live run or a stale crash leftover.
    /// - [`Error::Io`] on directory-create or write failures.
    pub fn acquire(layout: Layout<'_>, now: Timestamp) -> Result<Self, Error> {
        let path = layout.guest_lock_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }
        let mut file = match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(Error::validation_failed(
                    "guest-marker-held",
                    "no other guest execute run holds the marker",
                    format!(
                        "another guest execute run holds {} — if no run is live (a crash left \
                         the marker behind), delete the file and retry",
                        path.display()
                    ),
                ));
            }
            Err(err) => return Err(Error::Io(err)),
        };
        // Diagnostic body only — existence is the lock. Mirrors the
        // native plan-lock body shape.
        let host = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("HOST"))
            .unwrap_or_else(|_| "unknown".to_string());
        write!(file, "pid={}\nhostname={host}\nacquired-at={now}\n", holder_pid())
            .map_err(Error::Io)?;
        Ok(Self { path })
    }
}

/// The marker body's holder pid — diagnostics only.
///
/// `std::process::id()` aborts on `wasm32-wasip2` (WASI models no
/// process table), so the guest records `0`: the staleness posture never
/// probes the recorded pid, and `0` is unambiguous prose for "no pid on
/// this platform".
#[cfg_attr(
    target_arch = "wasm32",
    expect(
        clippy::missing_const_for_fn,
        reason = "const only on wasm32 (the literal-0 arm); the native body calls process::id()"
    )
)]
fn holder_pid() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::process::id()
    }
}

impl Drop for GuestMarker {
    /// Best-effort removal — a failed unlink degrades to the stale
    /// posture (next acquire refuses and names the file).
    fn drop(&mut self) {
        drop(std::fs::remove_file(&self.path));
    }
}
