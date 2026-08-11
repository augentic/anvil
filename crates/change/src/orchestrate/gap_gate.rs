//! Epoch freshness + gate-time gap deferral before build (RFC-86a
//! D1): deferred rows leave build scope; open rows are dispositioned
//! at the gate itself; drift is `plan-epoch-stale`.

use error::Error;
use jiff::Timestamp;
use project::config::Layout;
use project::journal::{self, DeferralOrigin, Event, EventKind};
use project::plan::epoch::EpochFreshness;
use project::plan::{Disposition, GapRow, Plan, collect_events, plan_gaps_body};

/// Enforce authorization-epoch freshness and disposition open gaps
/// for `slice` before build.
///
/// Freshness is the shared [`project::plan::epoch::freshness`]
/// predicate — the same rule `plan status` projects as Authorized.
/// The gate mints one `gap.deferred` fact per open row (`origin:
/// policy`, synthesized reason) and build proceeds — minting is
/// gate-time because `refine-under-epoch` rows do not exist earlier.
/// A digest-less legacy row has no match key and mints nothing.
///
/// # Errors
///
/// - `plan-epoch-stale` — no covering `plan.execute.started`, plan /
///   covered-spec digest drift, or an in-scope leaf absent from
///   coverage.
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
    match project::plan::epoch::freshness(layout, plan, &events)? {
        EpochFreshness::Unopened => {
            return Err(epoch_stale(
                "no covering `plan.execute.started` — run `emery plan execute` to open an \
                 authorization epoch before build",
            ));
        }
        EpochFreshness::Stale { detail } => return Err(epoch_stale(detail)),
        EpochFreshness::Fresh { .. } => {}
    }
    let gaps = plan_gaps_body(plan, layout, &events)?;

    // A live deferral takes a row out of build scope (D1); the
    // requirement is conserved as debt, never built over. Open
    // dispositions exist only on `[unknown]` / `[conflict]` rows.
    let open: Vec<&GapRow> = gaps
        .rows
        .iter()
        .filter(|row| row.slice == slice && row.disposition == Some(Disposition::Open))
        .collect();
    if open.is_empty() {
        return Ok(());
    }

    let facts = policy_deferrals(&open, now, epoch);
    if !facts.is_empty() {
        journal::append_batch(layout, &facts)?;
        tracing::info!(
            "dispositioned {} open gap row(s) on `{slice}` at the build gate",
            facts.len()
        );
    }
    Ok(())
}

/// One `gap.deferred` fact per open row (`origin: policy`, the
/// synthesized epoch reason). A digest-less legacy row contributes no
/// fact — it has no match key for any fact to cover.
fn policy_deferrals(open: &[&GapRow], now: Timestamp, epoch: Timestamp) -> Vec<Event> {
    let reason = format!("deferred by gap-policy under epoch {epoch}");
    open.iter()
        .filter_map(|row| {
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

fn epoch_stale(detail: impl Into<String>) -> Error {
    Error::validation_failed(
        "plan-epoch-stale",
        "covered artifacts changed — re-run emery plan execute",
        detail,
    )
}
