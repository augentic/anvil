//! Gap policy + epoch freshness before build (RFC-86a D1/D3 / RFC-91
//! D5): deferred rows leave build scope; open rows block under
//! `strict` and are dispositioned at the gate under `defer`; covered
//! refinement digest drift is `plan-epoch-stale`.

use std::fmt::Write as _;

use artifacts::spec::provenance::RequirementStatus;
use error::Error;
use jiff::Timestamp;
use project::GapPolicy;
use project::config::Layout;
use project::handler::Render;
use project::journal::{self, ClosedPlanCoverage, DeferralOrigin, Event, EventKind};
use project::plan::epoch::EpochFreshness;
use project::plan::{Disposition, GapRow, GapsBody, Plan, collect_events, plan_gaps_body};

/// Enforce authorization-epoch freshness and the typed gap policy for
/// `slice` before build.
///
/// Freshness is the shared [`project::plan::epoch::freshness`]
/// predicate — the same rule `plan status` projects as Authorized.
/// Under an effective `defer` policy the gate dispositions open rows
/// itself: one `gap.deferred` fact per requirement (`origin: policy`,
/// synthesized reason), then build proceeds (RFC-86a D3/D6).
///
/// # Errors
///
/// - `plan-epoch-stale` — no covering `plan.execute.started`, plan /
///   covered-refinement digest drift, or an in-scope leaf absent from
///   coverage.
/// - `plan-gaps-unresolved` — an in-scope `[unknown]` / `[conflict]`
///   on `slice` whose disposition is `open`, under an effective
///   `strict` policy (or a digest-less legacy row no fact can cover).
///   Detail includes the rendered gap inventory.
pub fn enforce_before_build(
    layout: Layout<'_>, plan: &Plan, slice: &str, now: Timestamp,
) -> Result<(), Error> {
    let events = collect_events(layout)?;
    // Fresh coverage implies a `plan.execute.started` fact in the
    // union; the `now` fallback is unreachable by construction.
    let epoch = events
        .iter()
        .rev()
        .find(|event| matches!(event.kind, EventKind::PlanExecuteStarted { .. }))
        .map_or(now, |event| event.timestamp);
    let coverage = match project::plan::epoch::freshness(layout, plan, &events)? {
        EpochFreshness::Unopened => {
            return Err(epoch_stale(
                "no covering `plan.execute.started` — run `emery plan execute` to open an \
                 authorization epoch before build",
            ));
        }
        EpochFreshness::Stale { detail } => return Err(epoch_stale(detail)),
        EpochFreshness::Fresh { coverage } => coverage,
    };
    let gaps = plan_gaps_body(plan, layout, &events)?;
    check_gap_policy(layout, slice, coverage, &gaps, now, epoch)
}

fn check_gap_policy(
    layout: Layout<'_>, slice: &str, coverage: &ClosedPlanCoverage, gaps: &GapsBody,
    now: Timestamp, epoch: Timestamp,
) -> Result<(), Error> {
    let ClosedPlanCoverage::ClosedPlan { gap_policy, .. } = coverage;
    let leaf_rows: Vec<_> = gaps.rows.iter().filter(|row| row.slice == slice).collect();

    // A live deferral takes a row out of build scope (D1); the
    // requirement is conserved as debt, never built over. Open
    // dispositions exist only on `[unknown]` / `[conflict]` rows.
    let open: Vec<&GapRow> = leaf_rows
        .iter()
        .filter(|row| row.disposition == Some(Disposition::Open))
        .copied()
        .collect();
    if open.is_empty() {
        return Ok(());
    }

    // Gate-time `origin: policy` minting (D3/D6): `defer` dispositions
    // open rows — unknown and conflict alike — and build proceeds.
    // Batch-or-nothing: a digest-less legacy row falls through to block.
    if *gap_policy == GapPolicy::Defer
        && let Some(facts) = policy_deferrals(&open, now, epoch)
    {
        journal::append_batch(layout, &facts)?;
        tracing::info!(
            "gap-policy defer: dispositioned {} open gap row(s) on `{slice}` at the build gate",
            facts.len()
        );
        return Ok(());
    }

    let mut blockers = Vec::new();
    for row in &open {
        match row.status {
            RequirementStatus::Conflict => {
                blockers.push(format!(
                    "{}/{} [conflict] {} — resolve inputs and re-refine, or defer it: `emery \
                     plan defer {}/{} --reason …`",
                    row.slice, row.req, row.summary, row.slice, row.req
                ));
            }
            RequirementStatus::Unknown => {
                blockers.push(format!(
                    "{}/{} [unknown] {} — close the gap or defer it: `emery plan defer {}/{} \
                     --reason …`",
                    row.slice, row.req, row.summary, row.slice, row.req
                ));
            }
            RequirementStatus::Divergence | RequirementStatus::Agreed => {
                // Open dispositions never land on these; keep the
                // match closed.
            }
        }
    }
    let divergences: Vec<String> = leaf_rows
        .iter()
        .filter(|row| row.status == RequirementStatus::Divergence)
        .map(|row| format!("{}/{} [divergence] {}", row.slice, row.req, row.summary))
        .collect();

    let mut detail = String::new();
    let _ = writeln!(detail, "gap policy ({gap_policy}) refused build for `{slice}`:");
    for line in &blockers {
        let _ = writeln!(detail, "  - {line}");
    }
    if !divergences.is_empty() {
        detail.push_str("listed (allowed) divergences:\n");
        for line in &divergences {
            let _ = writeln!(detail, "  - {line}");
        }
    }
    detail.push('\n');
    detail.push_str(&render_inventory(gaps));
    Err(gaps_unresolved(detail))
}

/// One `gap.deferred` fact per open row (`origin: policy`, the
/// synthesized epoch reason), or `None` when any row carries no
/// digest — nothing is appended for a partially-coverable set.
fn policy_deferrals(open: &[&GapRow], now: Timestamp, epoch: Timestamp) -> Option<Vec<Event>> {
    let reason = format!("deferred by gap-policy under epoch {epoch}");
    open.iter()
        .map(|row| {
            let digest = row.requirement_digest.as_ref()?;
            Some(Event::new(
                now,
                EventKind::GapDeferred {
                    slice: row.slice.as_str().into(),
                    req: row.req.clone(),
                    requirement_digest: digest.clone(),
                    reason: reason.clone(),
                    origin: DeferralOrigin::Policy,
                },
            ))
        })
        .collect()
}

fn render_inventory(gaps: &GapsBody) -> String {
    let mut buf = Vec::new();
    if gaps.render(&mut buf).is_err() {
        return String::new();
    }
    String::from_utf8(buf).unwrap_or_default()
}

fn gaps_unresolved(detail: impl Into<String>) -> Error {
    Error::validation_failed(
        "plan-gaps-unresolved",
        "resolve or defer typed gaps before build",
        detail,
    )
}

fn epoch_stale(detail: impl Into<String>) -> Error {
    Error::validation_failed(
        "plan-epoch-stale",
        "covered artifacts changed — re-run emery plan execute",
        detail,
    )
}
