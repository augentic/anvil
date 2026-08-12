//! Authorization-epoch open at `plan execute` start: assembles typed
//! `closed-plan` coverage (per-leaf refinement digests) and appends
//! `plan.execute.started` (RFC-86/86a/91).

use std::collections::BTreeMap;

use error::Error;
use jiff::Timestamp;
use project::build_record::BuildRecord;
use project::config::Layout;
use project::journal::{self, ClosedPlanCoverage, Event, EventKind};
use project::plan::{Plan, Status, collect_events, in_scope, project_ladders};
use project::slice::SliceMetadata;
use slice::refinement::{self, Freshness, Live};

use crate::plan::wire::load_discovery;

/// Append `plan.execute.started` with typed `closed-plan` coverage.
///
/// # Errors
///
/// Propagates coverage-assembly and journal append failures. A missing
/// or stale refinement manifest fails typed as
/// `plan-refinement-required` before any epoch append — execute never
/// refines (RFC-91 D5).
pub(super) fn append_started(layout: Layout<'_>, plan: &Plan, now: Timestamp) -> Result<(), Error> {
    let coverage = assemble_coverage(layout, plan)?;
    let event = Event::new(
        now,
        EventKind::PlanExecuteStarted {
            coverage,
            discovery_digest: None,
        },
    );
    journal::append_one(layout, &event)
}

/// Build `closed-plan` coverage over in-scope leaves (RFC-91 D5).
///
/// Every in-scope leaf **execute may still build** must project a
/// fresh refinement manifest; a missing or stale one fails typed
/// before any epoch append — execute never refines. Leaves past their
/// build are not re-litigated: a merged leaf (projected `done`)
/// contributes nothing, and a built leaf parked at merge carries the
/// manifest digest its wave bound at build time (resume path).
fn assemble_coverage(layout: Layout<'_>, plan: &Plan) -> Result<ClosedPlanCoverage, Error> {
    let plan_digest = Plan::file_digest(layout)?;

    let discovery = load_discovery(layout)?;
    let inventory = discovery.as_ref().map_or(&[][..], |d| d.leads());
    let events = collect_events(layout)?;
    let ladders = project_ladders(plan, &events);

    // One shared freshness cache across the leaves — the baseline and
    // source trees do not move while coverage assembles.
    let mut live = Live::new();
    let mut refinements = BTreeMap::new();
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
        if BuildRecord::present(&slice_dir)
            && let Some(digest) = refinement::file_digest(&slice_dir)?
        {
            // Built, awaiting merge: build promotion may legitimately
            // drift the bundle inputs (`writable-artifacts[]`), so the
            // covered digest is the unchanged manifest the wave bound.
            refinements.insert(name.to_string(), digest);
            continue;
        }
        match refinement::freshness_with(layout, plan, entry, inventory, &mut live)? {
            Freshness::Fresh { digest } => {
                refinements.insert(name.to_string(), digest);
            }
            Freshness::Missing => {
                return Err(refinement_required(format!(
                    "slice `{name}` has no refinement manifest — run `emery plan refine` \
                     before `emery plan execute`"
                )));
            }
            Freshness::Stale { reasons } => {
                let first = reasons.first().map_or("", String::as_str);
                return Err(refinement_required(format!(
                    "slice `{name}` refinement is stale ({first}) — re-run `emery plan refine` \
                     before `emery plan execute`"
                )));
            }
        }
    }

    Ok(ClosedPlanCoverage::ClosedPlan {
        plan_digest,
        refinements,
    })
}

const fn refinement_required(detail: String) -> Error {
    Error::Diag {
        code: "plan-refinement-required",
        detail,
    }
}
