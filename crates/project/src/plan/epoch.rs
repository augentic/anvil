//! Authorization-epoch freshness (RFC-86 D22 / S19 / RFC-91 D5): newest
//! `plan.execute.started` coverage vs live `plan.yaml` and covered
//! refinement digests — shared by status Authorized and the gap gate.

use error::Error;

use super::execution::project_ladders;
use super::model::{Plan, Status};
use super::scope::in_scope;
use crate::config::Layout;
use crate::journal::{self, ClosedPlanCoverage, Event, EventKind};
use crate::name::SliceName;
use crate::refinement;
use crate::slice::SliceMetadata;

/// The newest authorization epoch's relationship to the live artifacts.
#[derive(Debug)]
pub enum EpochFreshness<'a> {
    /// No `plan.execute.started` fact in the union.
    Unopened,
    /// The newest epoch no longer covers the live artifacts.
    Stale {
        /// Which covered artifact drifted, with epoch vs live digests.
        detail: String,
    },
    /// The newest epoch covers the live plan and refinement manifests.
    Fresh {
        /// The covering `closed-plan` coverage (leaves + gap policy).
        coverage: &'a ClosedPlanCoverage,
    },
}

/// Project the newest epoch's freshness from the chronologically
/// ordered fact union (see [`super::collect_events`]).
///
/// Fresh means the newest `plan.execute.started` coverage matches the
/// live `plan.yaml` digest, every in-scope entry not yet projected
/// `done` is in the per-leaf refinement coverage, and every such
/// leaf's on-disk refinement digest still matches its covered digest.
/// A `done` leaf is skipped: merge archives the slice tree, so its
/// absence is completion under the epoch, not drift.
///
/// # Errors
///
/// Propagates `plan.yaml` / slice-tree I/O failures and a corrupt
/// `metadata.yaml` ([`Error::YamlDe`]).
pub fn freshness<'a>(
    layout: Layout<'_>, plan: &Plan, events: &'a [Event],
) -> Result<EpochFreshness<'a>, Error> {
    let Some(coverage) = newest_coverage(events) else {
        return Ok(EpochFreshness::Unopened);
    };
    let freshness = staleness(layout, plan, events, coverage)?
        .map_or(EpochFreshness::Fresh { coverage }, |detail| EpochFreshness::Stale { detail });
    Ok(freshness)
}

/// Newest `closed-plan` coverage in the union.
fn newest_coverage(events: &[Event]) -> Option<&ClosedPlanCoverage> {
    events.iter().rev().find_map(|event| match &event.kind {
        EventKind::PlanExecuteStarted { coverage, .. } => Some(coverage),
        _ => None,
    })
}

/// First drift between `coverage` and the live artifacts, or `None`
/// when the epoch is fresh.
fn staleness(
    layout: Layout<'_>, plan: &Plan, events: &[Event], coverage: &ClosedPlanCoverage,
) -> Result<Option<String>, Error> {
    let ClosedPlanCoverage::ClosedPlan {
        plan_digest,
        refinements,
        ..
    } = coverage;

    let live_plan = Plan::file_digest(layout)?;
    if live_plan != *plan_digest {
        return Ok(Some(format!(
            "`plan.yaml` digest drifted (epoch {plan_digest}, live {live_plan}) — re-run \
             `emery plan execute`"
        )));
    }

    let ladders = project_ladders(plan, events);
    for entry in &plan.entries {
        let slice_dir = layout.slice_dir(entry.name.as_str());
        let meta = SliceMetadata::load_optional(&slice_dir)?;
        if !in_scope(plan, entry, meta.as_ref()) {
            continue;
        }
        if ladders.get(&entry.name).copied() == Some(Status::Done) {
            continue;
        }
        let name = entry.name.as_str();
        let Some(covered) = refinements.get(name) else {
            return Ok(Some(format!(
                "slice `{name}` is not in the covering epoch's per-leaf refinement coverage — \
                 re-run `emery plan execute`"
            )));
        };
        match refinement::file_digest(&slice_dir)? {
            Some(live) if live == *covered => {}
            Some(live) => {
                return Ok(Some(format!(
                    "covered refinement digest for `{name}` drifted (epoch {covered}, live \
                     {live}) — re-run `emery plan refine`, then `emery plan execute`"
                )));
            }
            None => {
                return Ok(Some(format!(
                    "covered refinement manifest for `{name}` is missing — re-run `emery plan \
                     refine`, then `emery plan execute`"
                )));
            }
        }
    }
    Ok(None)
}

/// A live claim without `plan.execute.started` cannot build or merge.
///
/// # Errors
///
/// `plan-epoch-required` when `slice` is claimed and no epoch exists.
pub fn require_for_claim(layout: Layout<'_>, slice: &str) -> Result<(), Error> {
    let events = journal::read_union(layout)?;
    let name = SliceName::from(slice);
    let claimed = journal::claim::project(&events).owner(&name).is_some();
    let epoch =
        events.iter().any(|event| matches!(event.kind, EventKind::PlanExecuteStarted { .. }));
    if claimed && !epoch {
        return Err(Error::Diag {
            code: "plan-epoch-required",
            detail: format!(
                "slice `{slice}` is claimed without a plan.execute.started epoch — run \
                 `emery plan execute`"
            ),
        });
    }
    Ok(())
}
