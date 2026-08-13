//! Inert boundary-proposal persist after a refinement escalation.

use artifacts::leads::Leads;
use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::Resolver;
use project::config::Layout;
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind, ParkReason, claim};
use project::plan::{BoundaryProposal, FocusParent, Frontiers, Plan, Proposal, collect_events};
use project::profile::Profiles;
use project::seam::{Source, Workspaces};
use project::snapshot::SnapshotId;
use project::wave::accepted_cid;
use slice::orchestrate::RefineOutcome as SliceRefine;

use super::decompose;
use super::survey::focused_leads;

/// Run focused resurvey + nearest-domain re-decomposition and persist
/// one inert boundary proposal. Live planning artifacts are unchanged.
///
/// # Errors
///
/// Survey, re-decomposition, and persist failures. Budget exhaustion
/// is remapped to `plan-refine-budget-exhausted`.
pub async fn persist<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, escalation: SliceRefine,
) -> Result<SnapshotId, Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let SliceRefine::Escalated {
        slice,
        assessment,
        affected,
        rationale,
        profile,
        ..
    } = escalation
    else {
        return Err(Error::Diag {
            code: "slice-synthesize-escalation-incomplete",
            detail: "persist expected a boundary-escalation outcome".into(),
        });
    };

    let layout = paths.layout();
    let live_leads = Leads::load(&layout.leads_path())?;
    let live_leads_bytes = std::fs::read(layout.leads_path())?;
    let live_decomp_bytes = std::fs::read(layout.decomposition_path()).ok();
    let live_plan_bytes = std::fs::read(layout.plan_path())?;

    let mut catalog = live_leads.clone();
    for parent in &affected {
        focus_into(provider, paths, now, plan, &mut catalog, parent).await?;
    }

    let candidate = decompose::nearest(provider, paths, now, plan, &slice, &mut catalog)
        .await
        .map_err(remap_budget)?;

    // Restore live planning artifacts if a persist-mode path leaked.
    restore(layout, &live_leads_bytes, live_decomp_bytes.as_deref(), &live_plan_bytes)?;

    let expected = frontiers(layout, plan)?;
    let proposal = Proposal::Boundary(BoundaryProposal {
        version: project::plan::PROPOSAL_VERSION,
        failed_leaf: slice.clone().into(),
        assessment,
        profile,
        rationale,
        affected,
        candidate_leads: catalog.into_leads(),
        candidate_decomposition: candidate.tree,
        expected,
    });
    let digest = proposal.save(layout)?;
    journal::append_one(
        layout,
        &Event::new(
            now,
            EventKind::SliceRefinementParked {
                slice_name: slice.into(),
                reason: ParkReason::BoundaryEscalation,
                proposal: Some(digest.clone()),
            },
        ),
    )?;
    Ok(digest)
}

async fn focus_into<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, catalog: &mut Leads,
    parent: &FocusParent,
) -> Result<(), Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let Some(binding) = plan.sources.get(&parent.source) else {
        return Err(Error::Diag {
            code: "plan-source-unknown",
            detail: format!("focus source `{}` is not a plan binding", parent.source),
        });
    };
    if binding.locator.is_none() {
        return Ok(());
    }
    let children = focused_leads(
        provider,
        provider,
        provider,
        paths,
        now,
        &parent.source,
        binding,
        catalog,
        &parent.lead,
    )
    .await?;
    catalog.merge_leads(&parent.source, children);
    Ok(())
}

fn restore(
    layout: Layout<'_>, leads: &[u8], decomp: Option<&[u8]>, plan: &[u8],
) -> Result<(), Error> {
    std::fs::write(layout.leads_path(), leads)?;
    if let Some(bytes) = decomp {
        std::fs::write(layout.decomposition_path(), bytes)?;
    }
    std::fs::write(layout.plan_path(), plan)?;
    Ok(())
}

fn frontiers(layout: Layout<'_>, plan: &Plan) -> Result<Frontiers, Error> {
    let events = collect_events(layout)?;
    let mut accepted = std::collections::BTreeMap::new();
    for (id, row) in &plan.targets {
        let cid = accepted_cid(layout, &events, id)?.unwrap_or_else(|| row.cid.clone());
        accepted.insert(id.clone(), cid);
    }
    let claims: Vec<_> = claim::project(&events).iter().map(|(slice, _)| slice.clone()).collect();
    Ok(Frontiers {
        leads_digest: plan.leads_digest.clone().ok_or_else(|| Error::Diag {
            code: "plan-leads-digest-missing",
            detail: "plan.yaml has no leads-digest".into(),
        })?,
        decomposition_digest: plan.decomposition_digest.clone().ok_or_else(|| Error::Diag {
            code: "plan-decomposition-digest-missing",
            detail: "plan.yaml has no decomposition-digest".into(),
        })?,
        discovery_digest: plan.discovery_digest.clone(),
        plan_digest: SnapshotId::parse(&Plan::file_digest(layout)?)?,
        accepted,
        committed: std::collections::BTreeMap::new(),
        claims,
    })
}

fn remap_budget(err: Error) -> Error {
    match err {
        Error::Validation { code, detail } if code.as_ref() == "plan-author-budget-exhausted" => {
            Error::validation_failed(
                "plan-refine-budget-exhausted",
                "focused resurvey and re-decomposition stay within the compiled judgment budget",
                detail,
            )
        }
        other => other,
    }
}
