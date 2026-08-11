//! The live deferred set for one slice (RFC-86a D4).
//!
//! Projected from the disposition join at request time — never stored.

use error::Result;
use project::config::Layout;
use project::plan::{Disposition, Plan, collect_events, plan_gaps_body};
use project::seam::wire::DeferredRequirement;

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
    let plan_path = layout.plan_path();
    if !plan_path.is_file() {
        return Ok(Vec::new());
    }
    let plan = Plan::load(&plan_path)?;
    let events = collect_events(layout)?;
    let gaps = plan_gaps_body(&plan, layout, &events)?;
    Ok(gaps
        .rows
        .into_iter()
        .filter(|row| row.slice == slice && row.disposition == Some(Disposition::Deferred))
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
