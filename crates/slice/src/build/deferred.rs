//! The live deferred set for one slice (RFC-86a D4).
//!
//! Projected from the disposition join at request time — never stored.

use error::Result;
use project::config::Layout;
use project::plan::{Disposition, GapRow, Plan, collect_events, plan_gaps_body};
use project::seam::wire::DeferredRequirement;

/// The slice's deferred gap rows plus the owning plan, in inventory
/// (declaration) order — the shared projection behind the build
/// request's exclusion set and the merge fold's carried debt.
///
/// `None` when the project has no `plan.yaml` (breakout builds and
/// standalone merges carry no dispositions).
///
/// # Errors
///
/// Propagates plan / journal / model read failures from the gap
/// projection.
pub(crate) fn deferred_rows(
    layout: Layout<'_>, slice: &str,
) -> Result<Option<(Plan, Vec<GapRow>)>> {
    let plan_path = layout.plan_path();
    if !plan_path.is_file() {
        return Ok(None);
    }
    let plan = Plan::load(&plan_path)?;
    let events = collect_events(layout)?;
    let rows = plan_gaps_body(&plan, layout, &events)?
        .rows
        .into_iter()
        .filter(|row| row.slice == slice && row.disposition == Some(Disposition::Deferred))
        .collect();
    Ok(Some((plan, rows)))
}

/// Every deferred gap row on `slice`, projected from the live model
/// and the deferral fact union — the build request's `deferred[]`
/// exclusion set, in inventory (declaration) order.
///
/// Empty when the project has no `plan.yaml` (breakout builds outside
/// a plan carry no dispositions) or when nothing on the slice is
/// deferred.
///
/// # Errors
///
/// Propagates plan / journal / model read failures from the gap
/// projection.
pub fn live_deferred(layout: Layout<'_>, slice: &str) -> Result<Vec<DeferredRequirement>> {
    let Some((_, rows)) = deferred_rows(layout, slice)? else {
        return Ok(Vec::new());
    };
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            // Deferred rows carry a digest by construction — a fact
            // can only match a digest-bearing row.
            let requirement_digest = row.requirement_digest?;
            Some(DeferredRequirement {
                id: row.req,
                title: row.summary,
                requirement_digest,
            })
        })
        .collect())
}
