//! Gap policy + epoch freshness before build: conflicts always block;
//! unknowns block unless waived on the covering `plan.execute.started`
//! epoch; divergence is allowed. Drift vs that epoch is `plan-epoch-stale`.

use std::fmt::Write as _;

use artifacts::spec::provenance::RequirementStatus;
use error::Error;
use project::config::Layout;
use project::handler::Render;
use project::journal::ClosedPlanCoverage;
use project::plan::epoch::EpochFreshness;
use project::plan::{GapsBody, Plan, collect_events, plan_gaps_body};

/// Enforce authorization-epoch freshness and the typed gap policy for
/// `slice` before build.
///
/// Freshness is the shared [`project::plan::epoch::freshness`]
/// predicate — the same rule `plan status` projects as Authorized.
///
/// # Errors
///
/// - `plan-epoch-stale` — no covering `plan.execute.started`, plan /
///   covered-spec digest drift, or an in-scope leaf absent from
///   coverage.
/// - `plan-gaps-unresolved` — in-scope `[conflict]` on `slice`, or
///   `[unknown]` without a matching waiver on the covering epoch.
///   Detail includes the rendered gap inventory.
pub fn enforce_before_build(layout: Layout<'_>, plan: &Plan, slice: &str) -> Result<(), Error> {
    let events = collect_events(layout)?;
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
    check_gap_policy(layout, plan, slice, coverage)
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
