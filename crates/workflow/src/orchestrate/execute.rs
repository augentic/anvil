//! The drained execute loop: the retired `/spec:execute` skill's body
//! as guest control flow behind the guest-routed `specify plan execute`.
//!
//! [`execute`] takes the [`GuestMarker`] (D1), then loops
//! [`plan_status_body`] → claim via the core `plan next` projection →
//! dispatch the projected phase ([`super::refine`], [`super::build`],
//! [`super::merge`]) until the plan projects `drained` or a stop. Every
//! stop — Gate 1 unstamped, a failed phase, a stuck queue — returns as
//! a typed [`ExecuteOutcome::Stopped`] carrying the status
//! projection's closed [`StopReason`] plus its operator hint, so the
//! shim renders the same stop-conditions surface the skill does.
//!
//! **Locking posture.** The loop claims entries through
//! [`plan_next_body`] directly — the lock-free core the native handler
//! wraps — because the native `require_held` flock gate fences
//! separate OS processes racing the plan, and the guest collapses
//! every breakout in-process (the loop is a structural single driver;
//! the native gate itself is untouched). Guest-vs-guest dual-driving
//! is refused by the D1 marker instead: a create-exclusive
//! `<plan-root>/.specify/guest.lock` held for the run. **No
//! cross-stack interlock is claimed** — a native skill-driven session
//! does not see the marker and the guest does not probe the native
//! flock; non-concurrent stack use is the documented coexistence rule.
//!
//! **Journal posture.** No `plan.execute.*` events exist — the loop
//! composes the per-phase cadence its verbs already emit
//! (`plan.entry.advanced` on each fresh claim, the `slice.synthesize.*`
//! / `slice.build.*` / `slice.merge.*` brackets, the transitions), so
//! a journal reader cannot tell one drained execute run from the same
//! phases driven one breakout at a time.

use std::io::Write as _;
use std::path::PathBuf;

use jiff::Timestamp;
use specify_error::Error;
use specify_guest_model::Model;

use crate::adapter::BuildInputDeclaration;
use crate::change::{LoopStep, NextActionKind, Plan, StopReason, plan_next_body, plan_status_body};
use crate::config::{Layout, ProjectConfig, with_state};
use crate::journal::{self, Event, EventKind};
use crate::seam::{SourceSeam, TargetSeam, WorkingTree};

/// One phase the loop completed, in run order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseRun {
    /// Slice the phase ran for.
    pub slice: String,
    /// Which phase completed.
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
    /// The loop halted on a stop condition; re-entry safe (the plan
    /// entry stays `in-progress` on phase failures).
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
/// Gate 1 is enforced by the first status projection: an unapproved
/// plan projects `stop plan-not-approved` and the loop returns it
/// before touching plan state. Re-entry is safe — a phase failure
/// leaves the entry `in-progress` and the journal terminal event in
/// place, so the next run's status projection resumes (or re-reports
/// the stop) from the same point.
///
/// `manifest_inputs` and `tree` are the caller-resolved build
/// parameters [`super::build`] takes (the shim resolves the bound
/// target's declared inputs once; today's deployments share one live
/// tree).
///
/// # Errors
///
/// - `plan-execute-workspace-unsupported` (exit 2) when the plan root
///   is a workspace or any entry is `project`-scoped — the skill's
///   workspace routing (slot sync + chdir) has no in-guest counterpart
///   yet, so the loop refuses rather than writing to the wrong tree.
/// - `guest-marker-held` (exit 2) when another guest execute run holds
///   the D1 marker — or a stale marker survived a crash; the detail
///   says which file to delete.
/// - propagates plan load/validate failures and marker I/O failures.
/// - phase failures do **not** surface here — they return as
///   [`ExecuteOutcome::Stopped`].
pub async fn execute<P: Model, S: SourceSeam, T: TargetSeam>(
    model: &P, sources: &S, targets: &T, layout: Layout<'_>, now: Timestamp,
    manifest_inputs: &[BuildInputDeclaration], tree: &WorkingTree,
) -> Result<ExecuteOutcome, Error> {
    refuse_workspace_routing(layout)?;
    let _marker = GuestMarker::acquire(layout, now)?;
    let mut phases: Vec<PhaseRun> = Vec::new();

    loop {
        let plan = Plan::load(&layout.plan_path())?;
        let status = plan_status_body(&plan, layout)?;
        let step = match status.action {
            NextActionKind::Drained => return Ok(ExecuteOutcome::Drained { phases }),
            NextActionKind::Stop => {
                // A stop projection always carries a stop body; a
                // missing one (unreachable by construction) degrades
                // to the generic stuck stop rather than panicking.
                let (reason, detail) = status
                    .stop
                    .map_or((StopReason::Stuck, None), |stop| (stop.reason, stop.detail));
                return Ok(ExecuteOutcome::Stopped {
                    reason,
                    detail,
                    hint: reason.hint(),
                    slice: status.slice,
                    phases,
                });
            }
            NextActionKind::Refine => LoopStep::Refine,
            NextActionKind::Build => LoopStep::Build,
            NextActionKind::Merge => LoopStep::Merge,
        };

        // Claim: `plan next` before every phase, exactly as the skill
        // drives it (returns the active entry unchanged mid-slice).
        let Some(claim) = claim_next(layout, now)? else {
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

        let result: Result<(), Error> = match step {
            LoopStep::Refine => {
                let target = claim.target.clone().ok_or_else(|| Error::Diag {
                    code: "slice-create-target-missing",
                    detail: format!(
                        "no target resolved for slice `{slice}`; declare the project adapter \
                         (or fix the bound project's topology) before executing"
                    ),
                })?;
                super::refine(model, sources, targets, layout, now, &slice, &target).await.map(drop)
            }
            LoopStep::Build => {
                super::build(targets, layout, now, &slice, manifest_inputs, tree.clone())
                    .await
                    .map(drop)
            }
            LoopStep::Merge => {
                let classes = crate::merge::artifact_classes(
                    layout.project_dir(),
                    &layout.slices_dir().join(&slice),
                );
                super::merge(layout, now, &slice, &classes, false).map(drop)
            }
        };

        match result {
            Ok(()) => phases.push(PhaseRun { slice, step }),
            Err(err) => {
                // The phase already journalled its failure terminal, so
                // a re-entrant run's status projection reports the same
                // stop this return carries. The entry stays
                // `in-progress` — merge is the only `done` writer.
                let reason = match step {
                    LoopStep::Refine => StopReason::RefineFailed,
                    LoopStep::Build => StopReason::BuildFailed,
                    LoopStep::Merge => StopReason::MergeConflict,
                };
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

/// Refuse workspace-routed plans: the `/spec:execute` skill syncs the
/// slot and chdirs into `workspace/<project>/` for a `project`-scoped
/// entry, and the guest loop has no counterpart yet — running anyway
/// would create slices under the workspace root's own `.specify/`
/// tree. Single-project plans (no `workspace: true`, no `project:`
/// keys) are unaffected.
fn refuse_workspace_routing(layout: Layout<'_>) -> Result<(), Error> {
    let config = ProjectConfig::load(layout.project_dir())?;
    let plan = Plan::load(&layout.plan_path())?;
    let scoped_entry = plan.entries.iter().find_map(|entry| entry.project.as_deref());
    if config.workspace || scoped_entry.is_some() {
        let detail = scoped_entry.map_or_else(
            || "the plan root is a workspace (`workspace: true` in project.yaml)".to_string(),
            |project| format!("plan entry scoped to project `{project}`"),
        );
        return Err(Error::validation_failed(
            "plan-execute-workspace-unsupported",
            "the guest execute loop runs single-project plans only",
            format!(
                "{detail}; workspace routing (slot sync + chdir) has no in-guest counterpart — \
                 drive workspace plans hand-driven (`specify plan next`, then the \
                 /spec:refine → /spec:build → /spec:merge breakouts)"
            ),
        ));
    }
    Ok(())
}

/// One claimed entry: the slice to run and its best-effort resolved
/// target (`name@vN`).
struct Claim {
    slice: String,
    target: Option<String>,
}

/// Claim the next entry through the lock-free `plan next` core (see
/// the module docs for why `require_held` does not apply in-loop),
/// journalling `plan.entry.advanced` on a fresh advance exactly as the
/// native handler does. Returns `None` when nothing is runnable
/// (drained / stuck — the status projection decides which).
fn claim_next(layout: Layout<'_>, now: Timestamp) -> Result<Option<Claim>, Error> {
    let config = ProjectConfig::load(layout.project_dir())?;
    let slices_dir = layout.slices_dir();
    let project_dir = layout.project_dir().to_path_buf();
    let (body, plan_name) = with_state::<Plan, _, _>(layout, "plan.yaml", move |plan| {
        let body = plan_next_body(plan, &slices_dir, &config, &project_dir)?;
        Ok((body, plan.name.clone()))
    })?;
    if let Some(advanced) = &body.next {
        let event = Event::new(
            now,
            EventKind::PlanEntryAdvanced {
                plan_name,
                slice_name: advanced.clone().into(),
            },
        );
        journal::append_batch(layout, std::slice::from_ref(&event))?;
    }
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
            let target = crate::slice::SliceMetadata::load(&layout.slices_dir().join(&slice))
                .ok()
                .map(|metadata| metadata.target);
            Some(Claim { slice, target })
        }
        (None, None) => None,
    })
}

/// The D1 create-exclusive advisory marker at
/// `<plan-root>/.specify/guest.lock`, held for one guest execute run.
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
/// This marker is the only execute-run interlock: the retired native
/// stack's flock-based `plan lock` wrapper is gone with the old stack.
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
