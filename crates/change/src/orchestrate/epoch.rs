//! Authorization-epoch open at `plan execute` start: assembles typed
//! `closed-plan` coverage carrying the effective gap policy and
//! appends `plan.execute.started` (RFC-86 D6 / RFC-86a D3).

use std::collections::BTreeMap;

use diagnostics::digest::sha256_hex;
use error::Error;
use jiff::Timestamp;
use project::GapPolicy;
use project::config::Layout;
use project::journal::{self, ClosedPlanCoverage, Event, EventKind, LeafSpecCoverage};
use project::plan::{Plan, dir_cid, in_scope};
use project::slice::SliceMetadata;

/// Append `plan.execute.started` carrying the effective `gap_policy`.
///
/// # Errors
///
/// Propagates coverage-assembly and journal append failures.
pub(super) fn append_started(
    layout: Layout<'_>, plan: &Plan, now: Timestamp, gap_policy: GapPolicy,
) -> Result<(), Error> {
    let coverage = assemble_coverage(layout, plan, gap_policy)?;
    let event = Event::new(
        now,
        EventKind::PlanExecuteStarted {
            coverage,
            discovery_digest: None,
        },
    );
    journal::append_one(layout, &event)
}

/// Build `closed-plan` coverage over in-scope leaves.
///
/// A leaf covers as `existing` only when its specs are present **and**
/// its recorded `base.yaml` pins still match the live trees; a
/// pin-drifted refined slice re-enters as `refine-under-epoch`, so the
/// loop re-refines exactly the affected slices under this epoch.
fn assemble_coverage(
    layout: Layout<'_>, plan: &Plan, gap_policy: GapPolicy,
) -> Result<ClosedPlanCoverage, Error> {
    let plan_bytes = std::fs::read(layout.plan_path())?;
    let plan_digest = format!("sha256:{}", sha256_hex(&plan_bytes));

    let mut specs = BTreeMap::new();
    for entry in &plan.entries {
        let slice_dir = layout.slice_dir(entry.name.as_str());
        let meta = SliceMetadata::load_optional(&slice_dir)?;
        if !in_scope(plan, entry, meta.as_ref()) {
            continue;
        }
        let leaf = if project::slice::has_spec_artifacts(&slice_dir)
            && !slice::pins_drifted(layout, &slice_dir, entry.name.as_str())?
        {
            LeafSpecCoverage::Existing {
                digest: dir_cid(&slice_dir.join("specs"))?.to_string(),
            }
        } else {
            LeafSpecCoverage::RefineUnderEpoch
        };
        specs.insert(entry.name.as_str().to_string(), leaf);
    }

    Ok(ClosedPlanCoverage::ClosedPlan {
        plan_digest,
        specs,
        gap_policy,
    })
}
