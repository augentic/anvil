//! Authorization-epoch open at `plan execute` start.
//!
//! Verifies the closed-plan digest chain, assembles typed coverage,
//! and appends `plan.execute.started`.

use std::collections::BTreeMap;

use error::Error;
use jiff::Timestamp;
use project::build_record::BuildRecord;
use project::config::Layout;
use project::handler::ExecutionPaths;
use project::journal::{self, ClosedPlanCoverage, Event, EventKind};
use project::plan::{Plan, Status, collect_events, in_scope, project_ladders};
use project::refinement;
use project::slice::SliceMetadata;
use project::snapshot::SnapshotId;
use slice::refinement::{self as slice_refinement, Freshness, Live};

use crate::plan::wire::load_leads;

/// Verify the digest chain, then append `plan.execute.started`.
///
/// # Errors
///
/// Closed-plan verification, coverage-assembly, and journal append
/// failures. A missing or stale refinement manifest fails typed as
/// `plan-refinement-required` before any epoch append.
pub(super) fn open(paths: &ExecutionPaths, plan: &Plan, now: Timestamp) -> Result<(), Error> {
    project::plan::closed_plan(paths, plan)?;
    super::author::current_definition(paths, plan)?;
    append_started(paths.layout(), plan, now)
}

/// Append `plan.execute.started` with typed `closed-plan` coverage.
///
/// # Errors
///
/// Propagates coverage-assembly and journal append failures. A missing
/// or stale refinement manifest fails typed as
/// `plan-refinement-required` before any epoch append — execute never
/// refines (RFC-91 D5).
pub(super) fn append_started(layout: Layout<'_>, plan: &Plan, now: Timestamp) -> Result<(), Error> {
    let (coverage, discovery_digest) = assemble_coverage(layout, plan)?;
    let event = Event::new(
        now,
        EventKind::PlanExecuteStarted {
            coverage,
            discovery_digest: Some(discovery_digest.to_string()),
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
fn assemble_coverage(
    layout: Layout<'_>, plan: &Plan,
) -> Result<(ClosedPlanCoverage, SnapshotId), Error> {
    let plan_digest = Plan::file_digest(layout)?;
    let discovery_digest = discovery_digest(layout, plan)?;

    let catalog = load_leads(layout)?;
    let inventory = catalog.as_ref().map_or(&[][..], |d| d.leads());
    let events = collect_events(layout)?;
    let ladders = project_ladders(plan, &events);

    // One shared freshness cache across the leaves — the baseline and
    // source trees do not move while coverage assembles.
    let mut live = Live::new();
    let mut refinements = BTreeMap::new();
    for entry in &plan.entries {
        let slice_dir = layout.slice_dir(entry.name.as_str());
        let meta = SliceMetadata::load_optional(&slice_dir)?;
        if !in_scope(plan, entry, meta.as_ref(), &events) {
            continue;
        }
        if ladders.get(&entry.name).copied() == Some(Status::Done) {
            continue;
        }
        let name = entry.name.as_str();
        if BuildRecord::present(&slice_dir)
            && let Some(digest) = slice_refinement::file_digest(&slice_dir)?
        {
            // Built, awaiting merge: build promotion may legitimately
            // drift the bundle inputs (`writable-artifacts[]`), so the
            // covered digest is the unchanged manifest the wave bound.
            refinements.insert(name.to_string(), digest);
            continue;
        }
        match slice_refinement::freshness_with(layout, plan, entry, inventory, &mut live)? {
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

    Ok((
        ClosedPlanCoverage::ClosedPlan {
            plan_digest,
            refinements,
        },
        discovery_digest,
    ))
}

fn discovery_digest(layout: Layout<'_>, plan: &Plan) -> Result<SnapshotId, Error> {
    if let Some(recorded) = &plan.discovery_digest {
        return Ok(recorded.clone());
    }
    let path = layout.discovery_yaml_path();
    if path.is_file() {
        return project::plan::Discovery::load(&path)?.digest();
    }
    Ok(refinement::empty_digest())
}

const fn refinement_required(detail: String) -> Error {
    Error::Diag {
        code: "plan-refinement-required",
        detail,
    }
}
