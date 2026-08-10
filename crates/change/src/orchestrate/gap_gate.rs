//! Gap policy + epoch freshness before build (RFC-86a D1): deferred
//! rows leave build scope; open `[unknown]` / `[conflict]` block;
//! divergence is allowed. Epoch drift is `plan-epoch-stale`.

use std::fmt::Write as _;

use artifacts::spec::provenance::RequirementStatus;
use error::Error;
use project::config::Layout;
use project::handler::Render;
use project::journal::ClosedPlanCoverage;
use project::plan::epoch::EpochFreshness;
use project::plan::{Disposition, GapsBody, Plan, collect_events, plan_gaps_body};

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
/// - `plan-gaps-unresolved` — an in-scope `[unknown]` / `[conflict]`
///   on `slice` whose disposition is `open`. Detail includes the
///   rendered gap inventory.
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
    let gaps = plan_gaps_body(plan, layout, &events)?;
    check_gap_policy(slice, coverage, &gaps)
}

fn check_gap_policy(
    slice: &str, coverage: &ClosedPlanCoverage, gaps: &GapsBody,
) -> Result<(), Error> {
    let ClosedPlanCoverage::ClosedPlan { gap_policy, .. } = coverage;
    let leaf_rows: Vec<_> = gaps.rows.iter().filter(|row| row.slice == slice).collect();

    let mut blockers = Vec::new();
    let mut divergences = Vec::new();
    for row in &leaf_rows {
        // A live deferral takes the row out of build scope (D1); the
        // requirement is conserved as debt, never built over.
        if row.disposition == Some(Disposition::Deferred) {
            continue;
        }
        match row.status {
            // RFC-86a step-6 seam: gate-time `origin: policy` minting
            // for open rows under an effective `defer` policy lands
            // here; until then open rows block under both policies.
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
