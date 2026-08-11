//! Gap policy + epoch freshness before build: conflicts always block;
//! unknowns block unless waived on the covering `plan.execute.started`
//! epoch; divergence is allowed. Drift vs that epoch is `plan-epoch-stale`.

use std::fmt::Write as _;

use artifacts::spec::provenance::RequirementStatus;
use error::Error;
use project::config::Layout;
use project::handler::Render;
use project::journal::{self, ClosedPlanCoverage, EventKind};
use project::plan::{GapsBody, Plan, plan_gaps_body};

/// Enforce authorization-epoch freshness and the typed gap policy for
/// `slice` before build.
///
/// # Errors
///
/// - `plan-epoch-stale` — no covering `plan.execute.started`, plan /
///   covered-refinement digest drift, or `slice` absent from coverage.
/// - `plan-gaps-unresolved` — in-scope `[conflict]` on `slice`, or
///   `[unknown]` without a matching waiver on the covering epoch.
///   Detail includes the rendered gap inventory.
pub fn enforce_before_build(layout: Layout<'_>, plan: &Plan, slice: &str) -> Result<(), Error> {
    let coverage = newest_coverage(layout)?;
    check_epoch_fresh(layout, slice, &coverage)?;
    check_gap_policy(layout, plan, slice, &coverage)
}

/// Newest `closed-plan` coverage from the fact union.
fn newest_coverage(layout: Layout<'_>) -> Result<ClosedPlanCoverage, Error> {
    let events = journal::read_union(layout)?;
    let Some(event) = events
        .iter()
        .rev()
        .find(|event| matches!(event.kind, EventKind::PlanExecuteStarted { .. }))
    else {
        return Err(epoch_stale(
            "no covering `plan.execute.started` — run `emery plan execute` to open an \
             authorization epoch before build",
        ));
    };
    match &event.kind {
        EventKind::PlanExecuteStarted { coverage, .. } => Ok(coverage.clone()),
        _ => unreachable!("filter matched PlanExecuteStarted"),
    }
}

fn check_epoch_fresh(
    layout: Layout<'_>, slice: &str, coverage: &ClosedPlanCoverage,
) -> Result<(), Error> {
    let ClosedPlanCoverage::ClosedPlan {
        plan_digest,
        refinements,
        ..
    } = coverage;

    let live_plan = Plan::file_digest(layout)?;
    if live_plan != *plan_digest {
        return Err(epoch_stale(format!(
            "`plan.yaml` digest drifted (epoch {plan_digest}, live {live_plan}) — re-run \
             `emery plan execute`"
        )));
    }

    // The gate guards *this* build: only the claimed slice's covered
    // digest is re-checked. Other covered leaves may legitimately have
    // moved on (a merged predecessor's slice tree is archived).
    let Some(covered) = refinements.get(slice) else {
        return Err(epoch_stale(format!(
            "slice `{slice}` is not in the covering epoch's per-leaf refinement coverage — \
             re-run `emery plan execute`"
        )));
    };
    match slice::refinement::file_digest(&layout.slice_dir(slice))? {
        Some(live) if live == *covered => Ok(()),
        Some(live) => Err(epoch_stale(format!(
            "covered refinement digest for `{slice}` drifted (epoch {covered}, live {live}) — \
             re-run `emery plan refine`, then `emery plan execute`"
        ))),
        None => Err(epoch_stale(format!(
            "covered refinement manifest for `{slice}` is missing — re-run `emery plan refine`, \
             then `emery plan execute`"
        ))),
    }
}

fn check_gap_policy(
    layout: Layout<'_>, plan: &Plan, slice: &str, coverage: &ClosedPlanCoverage,
) -> Result<(), Error> {
    let ClosedPlanCoverage::ClosedPlan { unknown_waivers, .. } = coverage;
    let gaps = plan_gaps_body(plan, layout)?;
    let leaf_rows: Vec<_> = gaps.rows.iter().filter(|row| row.slice == slice).collect();

    let mut blockers = Vec::new();
    let mut divergences = Vec::new();
    for row in &leaf_rows {
        match row.status {
            RequirementStatus::Conflict => {
                blockers.push(format!(
                    "{}/{} [conflict] {} — not waiveable; resolve inputs and re-refine",
                    row.slice, row.req, row.summary
                ));
            }
            RequirementStatus::Unknown => {
                let waived =
                    unknown_waivers.iter().any(|w| w.slice == row.slice && w.req == row.req);
                if !waived {
                    blockers.push(format!(
                        "{}/{} [unknown] {} — close the gap or `emery plan execute --waive {}/{} \
                         --reason …`",
                        row.slice, row.req, row.summary, row.slice, row.req
                    ));
                }
            }
            RequirementStatus::Divergence => {
                divergences.push(format!("{}/{} [divergence] {}", row.slice, row.req, row.summary));
            }
            RequirementStatus::Agreed => {
                // Gap inventory omits agreed rows; keep the match closed.
            }
        }
    }

    if blockers.is_empty() {
        return Ok(());
    }

    let mut detail = String::new();
    detail.push_str("gap policy refused build for `");
    detail.push_str(slice);
    detail.push_str("`:\n");
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
    detail.push_str(&render_inventory(&gaps));
    Err(gaps_unresolved(detail))
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
        "resolve or waive typed gaps before build",
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
