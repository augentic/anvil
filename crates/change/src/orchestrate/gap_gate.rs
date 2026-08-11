//! Gap policy + epoch freshness before build: conflicts always block;
//! unknowns block unless waived on the covering `plan.execute.started`
//! epoch; divergence is allowed. Drift vs that epoch is `plan-epoch-stale`.

use std::fmt::Write as _;
use std::path::Path;

use artifacts::spec::provenance::RequirementStatus;
use diagnostics::digest::sha256_hex;
use error::Error;
use project::config::Layout;
use project::handler::Render;
use project::journal::{self, ClosedPlanCoverage, EventKind};
use project::plan::{GapsBody, Plan, in_scope, plan_gaps_body};
use project::slice::SliceMetadata;

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
    check_epoch_fresh(layout, plan, slice, &coverage)?;
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
    layout: Layout<'_>, plan: &Plan, slice: &str, coverage: &ClosedPlanCoverage,
) -> Result<(), Error> {
    let ClosedPlanCoverage::ClosedPlan {
        plan_digest,
        refinements,
        ..
    } = coverage;

    let live_plan = live_plan_digest(layout)?;
    if live_plan != *plan_digest {
        return Err(epoch_stale(format!(
            "`plan.yaml` digest drifted (epoch {plan_digest}, live {live_plan}) — re-run \
             `emery plan execute`"
        )));
    }

    if !refinements.contains_key(slice) {
        return Err(epoch_stale(format!(
            "slice `{slice}` is not in the covering epoch's per-leaf refinement coverage — \
             re-run `emery plan execute`"
        )));
    }

    for (name, covered) in refinements {
        let Some(entry) = plan.entries.iter().find(|e| e.name.as_str() == name) else {
            continue;
        };
        let slice_dir = layout.slice_dir(name.as_str());
        let meta = load_meta(&slice_dir)?;
        if !in_scope(plan, entry, meta.as_ref()) {
            continue;
        }
        match slice::refinement::file_digest(&slice_dir)? {
            Some(live) if live.to_string() == *covered => {}
            Some(live) => {
                return Err(epoch_stale(format!(
                    "covered refinement digest for `{name}` drifted (epoch {covered}, live \
                     {live}) — re-run `emery plan refine`, then `emery plan execute`"
                )));
            }
            None => {
                return Err(epoch_stale(format!(
                    "covered refinement manifest for `{name}` is missing — re-run `emery plan \
                     refine`, then `emery plan execute`"
                )));
            }
        }
    }
    Ok(())
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

fn live_plan_digest(layout: Layout<'_>) -> Result<String, Error> {
    let bytes = std::fs::read(layout.plan_path())?;
    Ok(format!("sha256:{}", sha256_hex(&bytes)))
}

fn load_meta(slice_dir: &Path) -> Result<Option<SliceMetadata>, Error> {
    match SliceMetadata::load(slice_dir) {
        Ok(meta) => Ok(Some(meta)),
        Err(
            Error::ArtifactNotFound { .. }
            | Error::Diag {
                code: "slice-not-found",
                ..
            },
        ) => Ok(None),
        Err(err) => Err(err),
    }
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
