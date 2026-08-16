//! The drained execute loop behind `emery plan execute`: fan ready
//! builds onto the bounded pool, commit merges serially at the
//! canonical head, repeat until `drained` or a typed stop (RFC-96).

use std::ops::ControlFlow;

use artifacts::leads::Leads;
use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::Resolver;
use project::config::Layout;
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind, claim};
use project::plan::{
    LoopStep, NextActionKind, Plan, StatusBody, StopReason, collect_events, plan_status_body,
    schedule,
};
use project::pool;
use project::seam::{PhaseSource, Source, Target, Workspaces, Worktree};
use tracing::Instrument as _;

use super::converge;

mod marker;
mod publication;

pub use marker::GuestMarker;
use publication::Reconciled;

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

/// Run the drained execute loop: advance → build → merge per entry
/// until `plan status` projects `drained` or a stop. Execute never
/// refines — refinement runs only in `emery plan refine` (RFC-91 D5).
///
/// Re-entry is safe: a build / preflight-merge failure leaves the
/// entry `in-progress`, so the next run resumes (or re-reports the
/// stop); a postflight failure stamps `done` (non-rollback) and
/// projects a sticky stop the next execute acknowledges. The bound
/// target adapter resolves once in loop setup, before the marker.
///
/// # Errors
///
/// Refuses with `guest-marker-held` (exit 2) when another guest execute run holds
///   the marker — or a stale marker survived a crash; the detail
///   says which file to delete.
/// Phase failures do **not** surface here — they return as
///   [`ExecuteOutcome::Stopped`].
pub async fn execute<P: Model, S: Source, T: Target + Workspaces + Worktree, R: Resolver>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp,
) -> Result<ExecuteOutcome, Error> {
    let layout = paths.layout();
    if !paths.is_detached() {
        drop(project::config::ProjectConfig::load(paths.project_root())?);
    }
    let plan = Plan::load(&layout.plan_path())?;
    let _marker = GuestMarker::acquire(layout, now)?;
    if let Some(outcome) = before_epoch(caps.targets, paths, now).await? {
        return Ok(outcome);
    }
    // Digest chain, then `plan.execute.started` with typed coverage.
    super::epoch::open(paths, &plan, now)?;
    let mut phases: Vec<PhaseRun> = Vec::new();

    loop {
        let plan = Plan::load(&layout.plan_path())?;
        let status = plan_status_body(&plan, layout)?;
        // A pending publication member is the loop's own reconcile
        // step, not a dispatched slice phase.
        if status.action == NextActionKind::Materialize {
            if let Some(stopped) = reconcile_publication(caps.targets, paths, now, &phases).await? {
                return Ok(stopped);
            }
            continue;
        }
        // A projected complete-round gap (RFC-96 D8) is the loop's own
        // convergence step: record the missing rounds, then re-project
        // — a durably failed round re-surfaces as the stop.
        if status.stop.as_ref().map(|stop| stop.reason) == Some(StopReason::DomainCompleteFailed) {
            match converge::complete(caps.targets, caps.resolver, paths, now, &plan).await? {
                None => continue,
                Some(failure) => return Ok(domain_stop(&failure, &phases)),
            }
        }
        match dispatch_status(layout, now, status, &phases) {
            ControlFlow::Break(outcome) => return Ok(outcome),
            ControlFlow::Continue(None) => continue, // postflight ack
            ControlFlow::Continue(Some(_)) => {}     // ready work below
        }

        // Ready-set dispatch (RFC-96 D2): merges commit serially at
        // the canonical head (a landed merge requeues stale sibling
        // builds by identity); ready builds fan out on the pool.
        let ready = ready_items(layout, &plan)?;
        if let Some(item) = ready.iter().find(|item| item.phase == LoopStep::Merge) {
            match run_merge(caps, paths, now, &plan, item.slice.as_str(), &mut phases).await? {
                Some(stopped) => return Ok(stopped),
                None => continue,
            }
        }
        let builds: Vec<schedule::WorkItem> =
            ready.into_iter().filter(|item| item.phase == LoopStep::Build).collect();
        if builds.is_empty() {
            // The status projection targeted a phase but the ready set
            // is empty — plan state moved underneath us. Surface it as
            // the stuck stop rather than spinning.
            return Ok(ExecuteOutcome::Stopped {
                reason: StopReason::Stuck,
                detail: None,
                hint: StopReason::Stuck.hint(),
                slice: None,
                phases,
            });
        }
        if let Some(stopped) = run_builds(caps, paths, now, &plan, &builds, &mut phases).await? {
            return Ok(stopped);
        }
    }
}

/// The ready set over the live plan, facts, and lead catalog. An
/// absent catalog degrades to empty, matching the freshness posture.
fn ready_items(layout: Layout<'_>, plan: &Plan) -> Result<Vec<schedule::WorkItem>, Error> {
    let events = collect_events(layout)?;
    let leads_path = layout.leads_path();
    let catalog = if leads_path.exists() { Leads::load(&leads_path)? } else { Leads::empty() };
    let mut live = project::refinement::Live::new();
    schedule::ready_set(plan, layout, &events, catalog.leads(), &mut live)
}

/// Claim `slice` for this writer (`slice.claimed` +
/// `plan.entry.advanced`), a no-op when this writer already holds it.
///
/// # Errors
///
/// `slice-claim-conflict` when another writer owns the slice.
fn claim_slice(layout: Layout<'_>, now: Timestamp, plan: &Plan, slice: &str) -> Result<(), Error> {
    let slice_name: project::name::SliceName = slice.to_string().into();
    let events = collect_events(layout)?;
    let ownership = claim::project(&events);
    let writer = journal::writer_id();
    if ownership.owner(&slice_name) == Some(writer.as_str()) {
        return Ok(());
    }
    let claimed = claim::claim(&ownership, slice_name.clone(), &writer)?;
    journal::append_one(layout, &Event::new(now, claimed))?;
    journal::append_one(
        layout,
        &Event::new(
            now,
            EventKind::PlanEntryAdvanced {
                plan_name: plan.name.clone(),
                slice_name,
            },
        ),
    )?;
    Ok(())
}

/// Commit one merge at the canonical head — serially, never on the
/// pool. Deferred-disposition drift (RFC-86a D4) downgrades the merge
/// to a rebuild under this epoch. Returns the stop that ends the run,
/// or `None` to continue the loop.
async fn run_merge<P: Model, S: Source, T: Target + Workspaces + Worktree, R: Resolver>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp, plan: &Plan,
    slice: &str, phases: &mut Vec<PhaseRun>,
) -> Result<Option<ExecuteOutcome>, Error> {
    let layout = paths.layout();
    claim_slice(layout, now, plan, slice)?;
    let step = if slice::dispositions_drifted(layout, &layout.slice_dir(slice), slice)? {
        tracing::info!("deferred dispositions drifted for {slice} — re-building under this epoch");
        LoopStep::Build
    } else {
        LoopStep::Merge
    };
    tracing::info!("{step} {slice} …");
    if step == LoopStep::Build {
        super::enforce_before_build(layout, plan, slice, now)?;
    }
    // The frontier round gates a multi-member wave's commit (RFC-96
    // D8): a failed round parks the wave — no prefix commits.
    if step == LoopStep::Merge
        && let Some(failure) =
            converge::frontier(caps.targets, caps.resolver, paths, now, plan, slice).await?
    {
        return Ok(Some(domain_stop(&failure, phases)));
    }
    match run_phase(caps, paths, now, step, slice).await {
        Ok(verification) => {
            tracing::info!("{step} {slice} — completed");
            phases.push(PhaseRun {
                slice: slice.to_string(),
                step,
                verification,
            });
            // A landed merge may complete its target's in-scope set:
            // record any newly convergeable complete rounds (RFC-96
            // D8), then materialize eagerly (RFC-95 D11).
            if step == LoopStep::Merge {
                if let Some(failure) =
                    converge::complete(caps.targets, caps.resolver, paths, now, plan).await?
                {
                    return Ok(Some(domain_stop(&failure, phases)));
                }
                return reconcile_publication(caps.targets, paths, now, phases).await;
            }
            Ok(None)
        }
        Err(err) => {
            let reason = phase_stop_reason(step, &err);
            tracing::info!("{step} {slice} — stopped: {reason}");
            Ok(Some(ExecuteOutcome::Stopped {
                reason,
                detail: Some(err.to_string()),
                hint: reason.hint(),
                slice: Some(slice.to_string()),
                phases: phases.clone(),
            }))
        }
    }
}

/// Fan the ready builds onto the bounded pool: same-target groups
/// freeze one multi-member wave before claims and builds (RFC-96 D7,
/// membership capped at the pool cap), claims and the gap gate run
/// serially per admitted item, dispatches overlap up to the cap, and
/// outcomes join in canonical order — never completion order. The
/// first failure (in canonical order) drains in-flight siblings to
/// their terminal reports and stops the run; completed siblings keep
/// their `PhaseRun` rows.
async fn run_builds<P: Model, S: Source, T: Target + Workspaces + Worktree, R: Resolver>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp, plan: &Plan,
    builds: &[schedule::WorkItem], phases: &mut Vec<PhaseRun>,
) -> Result<Option<ExecuteOutcome>, Error> {
    let layout = paths.layout();
    let builds = capped_groups(builds);
    for (target, group) in &builds {
        if group.len() > 1 {
            let slices: Vec<String> = group.iter().map(|item| item.slice.to_string()).collect();
            slice::orchestrate::open_wave_group(caps.targets, layout, now, target, &slices).await?;
        }
    }
    let builds: Vec<&schedule::WorkItem> =
        builds.iter().flat_map(|(_, group)| group.iter().copied()).collect();
    for item in &builds {
        claim_slice(layout, now, plan, item.slice.as_str())?;
        // Epoch freshness gates build before the target orchestration
        // (`plan-epoch-stale`); open gaps are dispositioned at the
        // gate itself (gate-time deferrals) and never block.
        super::enforce_before_build(layout, plan, item.slice.as_str(), now)?;
        tracing::info!("build {} …", item.slice);
    }
    let claims = pool::Claims::default();
    let jobs: Vec<pool::Job<'_, Option<PhaseSource>, Error>> = builds
        .iter()
        .map(|item| pool::Job {
            claim: pool::Claim {
                item: format!("{}:{}", item.slice, item.digest),
                operation: "build".to_string(),
                attempt: 1,
            },
            budget: pool::budget::BUILD,
            future: Box::pin(run_phase(caps, paths, now, LoopStep::Build, item.slice.as_str())),
        })
        .collect();
    let outcomes = pool::run(pool::cap(), &claims, pool::OnFailure::Drain, jobs).await;

    let mut stop: Option<(StopReason, String, String)> = None;
    for (item, outcome) in builds.iter().copied().zip(outcomes) {
        let slice = item.slice.as_str();
        match settle_build(outcome, slice) {
            Ok(verification) => {
                tracing::info!("build {slice} — completed");
                phases.push(PhaseRun {
                    slice: slice.to_string(),
                    step: LoopStep::Build,
                    verification,
                });
            }
            Err(err) if stop.is_none() => {
                let reason = phase_stop_reason(LoopStep::Build, &err);
                tracing::info!("build {slice} — stopped: {reason}");
                stop = Some((reason, err.to_string(), slice.to_string()));
            }
            // Later failures re-surface on the re-entrant run; the
            // first (in canonical order) is this run's stop.
            Err(_) => {}
        }
    }
    Ok(stop.map(|(reason, detail, slice)| ExecuteOutcome::Stopped {
        reason,
        detail: Some(detail),
        hint: reason.hint(),
        slice: Some(slice),
        phases: phases.clone(),
    }))
}

/// Map one failed domain round onto the typed execute stop (RFC-96
/// D8): `domain-frontier-failed` parks the wave;
/// `domain-complete-failed` blocks dependants, drain, and publication.
fn domain_stop(failure: &converge::Failure, phases: &[PhaseRun]) -> ExecuteOutcome {
    let reason = match failure.kind {
        project::domain::RoundKind::Frontier => StopReason::DomainFrontierFailed,
        project::domain::RoundKind::Complete => StopReason::DomainCompleteFailed,
    };
    tracing::info!("domain {} — stopped: {reason}", failure.domain);
    ExecuteOutcome::Stopped {
        reason,
        detail: Some(failure.detail.clone()),
        hint: reason.hint(),
        slice: None,
        phases: phases.to_vec(),
    }
}

/// Group the canonical-order ready builds by target, truncating each
/// group at the pool cap: frozen wave membership never exceeds the
/// cap, and overflow items wait for a later round (RFC-96 D7).
fn capped_groups(builds: &[schedule::WorkItem]) -> Vec<(String, Vec<&schedule::WorkItem>)> {
    let cap = pool::cap();
    let mut groups: Vec<(String, Vec<&schedule::WorkItem>)> = Vec::new();
    for item in builds {
        match groups.last_mut() {
            Some((target, group)) if *target == item.target => {
                if group.len() < cap {
                    group.push(item);
                }
            }
            _ => groups.push((item.target.clone(), vec![item])),
        }
    }
    groups
}

/// Fold one pool outcome into the drain's per-build surface, in
/// canonical order.
fn settle_build(
    outcome: pool::Outcome<Option<PhaseSource>, Error>, slice: &str,
) -> Result<Option<PhaseSource>, Error> {
    match outcome {
        pool::Outcome::Settled(result) => result,
        pool::Outcome::TimedOut => Err(Error::Diag {
            code: "target-build-timeout",
            detail: format!("build of `{slice}` exceeded its inactivity budget; re-run execute"),
        }),
        pool::Outcome::Rejected | pool::Outcome::Cancelled | pool::Outcome::Skipped => {
            Err(Error::Diag {
                code: "target-build-cancelled",
                detail: format!("build of `{slice}` did not run (a sibling build failed first)"),
            })
        }
    }
}

/// The pre-epoch gate. A drained plan is a read-only no-op: opening a
/// fresh authorization epoch would journal coverage nothing runs
/// under. Pending publication members reconcile first — the fact
/// predicate authorizes the materialize; no new epoch (RFC-95 D11).
async fn before_epoch<W: Worktree>(
    worktree: &W, paths: &ExecutionPaths, now: Timestamp,
) -> Result<Option<ExecuteOutcome>, Error> {
    let layout = paths.layout();
    let status = plan_status_body(&Plan::load(&layout.plan_path())?, layout)?;
    let status = if status.action == NextActionKind::Materialize {
        if let Some(stopped) = reconcile_publication(worktree, paths, now, &Vec::new()).await? {
            return Ok(Some(stopped));
        }
        // Re-project: the reconcile may have completed the drain.
        plan_status_body(&Plan::load(&layout.plan_path())?, layout)?
    } else {
        status
    };
    if status.action == NextActionKind::Drained {
        return Ok(Some(ExecuteOutcome::Drained {
            plan: status.plan,
            phases: Vec::new(),
        }));
    }
    Ok(None)
}

/// Run one publication reconcile pass; a member refusal maps onto the
/// typed publication stop.
async fn reconcile_publication<W: Worktree>(
    worktree: &W, paths: &ExecutionPaths, now: Timestamp, phases: &[PhaseRun],
) -> Result<Option<ExecuteOutcome>, Error> {
    match publication::reconcile(worktree, paths, now).await? {
        Reconciled::Clean => Ok(None),
        Reconciled::Stopped { reason, detail } => Ok(Some(ExecuteOutcome::Stopped {
            reason,
            detail: Some(detail),
            hint: reason.hint(),
            slice: None,
            phases: phases.to_vec(),
        })),
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
        // The loop intercepts materialize before dispatch; looping
        // back re-intercepts if a race re-projects it.
        NextActionKind::Materialize => ControlFlow::Continue(None),
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
