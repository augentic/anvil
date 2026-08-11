//! Epoch freshness + gate-time gap deferral before build (RFC-86a
//! D1): deferred rows leave build scope; open rows are dispositioned
//! at the gate itself; drift is `plan-epoch-stale`.

use error::Error;
use jiff::Timestamp;
use project::config::Layout;
use project::journal::{self, Event, EventKind};
use project::plan::epoch::EpochFreshness;
use project::plan::{Disposition, GapRow, Plan, collect_events, plan_gaps_body};

/// Enforce authorization-epoch freshness and disposition open gaps
/// for `slice` before build.
///
/// Freshness is the shared [`project::plan::epoch::freshness`]
/// predicate — the same rule `plan status` projects as Authorized.
/// The gate mints one `gap.deferred` fact per open row (synthesized
/// reason) and build proceeds — minting is gate-time because
/// `refine-under-epoch` rows do not exist earlier. A digest-less open
/// row (legacy `spec.md` fallback) has no match key: the gate refuses.
///
/// # Errors
///
/// - `plan-epoch-stale` — no covering `plan.execute.started`, plan /
///   covered-spec digest drift, or an in-scope leaf absent from
///   coverage.
/// - `plan-gap-digest-missing` — an open row on `slice` carries no
///   `requirement-digest`, so no deferral fact can cover it.
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
    let digest_less: Vec<&GapRow> =
        open.iter().filter(|row| row.requirement_digest.is_none()).copied().collect();
    if !digest_less.is_empty() {
        return Err(digest_missing(&digest_less));
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

/// One `gap.deferred` fact per open row (the synthesized epoch
/// reason). Digest-less rows never reach here — the gate refuses
/// them before minting.
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
                },
            ))
        })
        .collect()
}

/// Refusal for open rows with no `requirement-digest` (a legacy
/// `spec.md`-fallback inventory): the detail names every affected
/// row and the recovery path.
fn digest_missing(rows: &[&GapRow]) -> Error {
    let named: Vec<String> = rows.iter().map(|row| format!("{}/{}", row.slice, row.req)).collect();
    Error::validation_failed(
        "plan-gap-digest-missing",
        "open gap rows carry no requirement digest — no deferral fact can cover them",
        format!(
            "{}; re-run `emery plan execute` — the refine phase rewrites `model.yaml` and mints \
             the digests deferrals match on",
            named.join(", ")
        ),
    )
}

fn epoch_stale(detail: impl Into<String>) -> Error {
    Error::validation_failed(
        "plan-epoch-stale",
        "covered artifacts changed — re-run emery plan execute",
        detail,
    )
}
