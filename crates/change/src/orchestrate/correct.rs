//! `plan correct` — durable operator correction, phase-split: fact-only
//! on a parked author (honored by `plan author` re-entry), fact plus an
//! inert [`Proposal::Boundary`] on an authored plan (applied by amend).

use std::collections::BTreeMap;

use artifacts::leads::Leads;
use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::Resolver;
use project::config::Layout;
use project::handler::ExecutionPaths;
use project::journal::{self, CorrectionConstraint, Event, EventKind};
use project::plan::correction::Correction;
use project::plan::decomposition::Decomposition;
use project::plan::{BoundaryProposal, Frontiers, Plan, ProfileRef, Proposal};
use project::profile::{Assessment, Profiles};
use project::seam::{Source, Workspaces};
use project::snapshot::SnapshotId;

use super::decompose;

/// One operator correction as received from the CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionInput {
    /// Domain id (or leaf slice name on an authored plan). Absent
    /// resolves the sole parked domain.
    pub domain: Option<String>,
    /// Closed structural constraint the deterministic tail enforces.
    pub constraint: Option<CorrectionConstraint>,
    /// Child domain ids a `split` constraint requires.
    pub children: Vec<String>,
    /// Operator intent, verbatim.
    pub intent: String,
}

/// How the correction settled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorrectOutcome {
    /// Bound-not-authored: the fact alone; `plan author` re-entry
    /// honors it.
    Recorded {
        /// Corrected domain id.
        domain: String,
    },
    /// Authored: the fact plus an inert boundary proposal for
    /// `plan amend --proposal`.
    Proposed {
        /// Corrected domain id.
        domain: String,
        /// Retained proposal digest.
        proposal: SnapshotId,
    },
}

/// Record one operator correction.
///
/// # Errors
///
/// Missing or ambiguous domain, an incoherent constraint, judgment
/// failures on the authored path, and journal/persist failures.
pub async fn correct<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, input: CorrectionInput,
) -> Result<CorrectOutcome, Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    check_shape(&input)?;
    let layout = paths.layout();
    let plan = Plan::load(&layout.plan_path())?;
    let events = journal::read_union(layout)?;
    let authored = events.iter().any(|event| {
        matches!(
            &event.kind,
            EventKind::PlanReconcileCompleted { plan_name, .. } if plan_name == &plan.name
        )
    });
    if authored {
        propose(provider, paths, now, layout, &plan, input).await
    } else {
        record(layout, now, &events, input)
    }
}

/// Constraint coherence: named children require a `split` constraint;
/// intent is never empty.
fn check_shape(input: &CorrectionInput) -> Result<(), Error> {
    if input.intent.trim().is_empty() {
        return Err(Error::Argument {
            flag: "intent",
            detail: "plan correct requires a non-empty --intent".into(),
        });
    }
    if !input.children.is_empty() && input.constraint != Some(CorrectionConstraint::Split) {
        return Err(Error::Argument {
            flag: "child",
            detail: "--child names required split children; it needs --constraint split".into(),
        });
    }
    Ok(())
}

/// The parked-author path: journal the fact, nothing else. Always
/// works regardless of how many domains are open, and spends no
/// judgment budget.
fn record(
    layout: Layout<'_>, now: Timestamp, events: &[Event], input: CorrectionInput,
) -> Result<CorrectOutcome, Error> {
    let domain = resolve_parked(layout, events, input.domain.as_deref())?;
    journal::append_one(
        layout,
        &Event::new(
            now,
            EventKind::PlanCorrectionRecorded {
                domain: domain.clone(),
                intent: input.intent,
                constraint: input.constraint,
                children: input.children,
                proposal: None,
            },
        ),
    )?;
    Ok(CorrectOutcome::Recorded { domain })
}

/// The corrected domain on the parked-author path: the explicit
/// `--domain` (validated against the persisted tree when one exists),
/// else the sole parked domain.
fn resolve_parked(
    layout: Layout<'_>, events: &[Event], domain: Option<&str>,
) -> Result<String, Error> {
    let tree = Decomposition::load_opt(&layout.decomposition_path())?;
    if let Some(domain) = domain {
        if let Some(tree) = &tree
            && !tree.nodes.contains_key(domain)
        {
            return Err(Error::Diag {
                code: "decomposition-node-unknown",
                detail: format!("no decomposition node `{domain}`"),
            });
        }
        return Ok(domain.to_string());
    }
    let parked = pending_parks(events);
    match parked.as_slice() {
        [sole] => Ok(sole.clone()),
        [] => Err(Error::validation_failed(
            "plan-correct-domain-required",
            "an implicit correction targets the sole parked domain",
            "no domain is parked; name one with --domain <id>",
        )),
        many => Err(Error::validation_failed(
            "plan-correct-domain-ambiguous",
            "an implicit correction targets the sole parked domain",
            format!(
                "{} domains are parked ({}); name one with --domain <id>",
                many.len(),
                many.join(", ")
            ),
        )),
    }
}

/// Parked domains not yet resolved: park facts appended after the
/// latest reconcile, deduplicated in park order.
fn pending_parks(events: &[Event]) -> Vec<String> {
    let mut parks: Vec<String> = Vec::new();
    for event in events {
        match &event.kind {
            EventKind::PlanReconcileCompleted { .. } => parks.clear(),
            EventKind::PlanAuthorParked { domain, .. } if !parks.contains(domain) => {
                parks.push(domain.clone());
            }
            _ => {}
        }
    }
    parks
}

/// The authored path: snapshot live bytes, re-decompose the corrected
/// domain into an inert candidate with the correction in the request,
/// restore, save the boundary proposal, and journal the fact.
async fn propose<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, layout: Layout<'_>, plan: &Plan,
    input: CorrectionInput,
) -> Result<CorrectOutcome, Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let Some(named) = input.domain.clone() else {
        return Err(Error::validation_failed(
            "plan-correct-domain-required",
            "a correction on an authored plan names its domain",
            "no domain is parked on an authored plan; name one with --domain <id>",
        ));
    };
    let tree = Decomposition::load(&layout.decomposition_path())?;
    let domain = resolve_domain(&tree, &named)?;
    let profile = domain_profile(&tree, plan, &domain)?;

    let live_leads = Leads::load(&layout.leads_path())?;
    let live_leads_bytes = std::fs::read(layout.leads_path())?;
    let live_decomp_bytes = std::fs::read(layout.decomposition_path())?;
    let live_plan_bytes = std::fs::read(layout.plan_path())?;

    let corrections = BTreeMap::from([(
        domain.clone(),
        vec![Correction {
            intent: input.intent.clone(),
            constraint: input.constraint,
            children: input.children.clone(),
        }],
    )]);
    let mut catalog = live_leads;
    let candidate =
        decompose::candidate(provider, paths, now, plan, &domain, &mut catalog, &corrections)
            .await
            .map_err(map_non_reducing);

    // Restore live planning artifacts if a persist-mode path leaked.
    std::fs::write(layout.leads_path(), &live_leads_bytes)?;
    std::fs::write(layout.decomposition_path(), &live_decomp_bytes)?;
    std::fs::write(layout.plan_path(), &live_plan_bytes)?;
    let candidate = candidate?;

    let expected = Frontiers::live(layout, plan)?;
    let proposal = Proposal::Boundary(BoundaryProposal {
        version: project::plan::PROPOSAL_VERSION,
        failed_leaf: named.into(),
        // Operator-directed: no refinement judgment scored this
        // domain, so the assessment is the neutral floor.
        assessment: NEUTRAL,
        profile,
        rationale: input.intent.clone(),
        affected: Vec::new(),
        candidate_leads: catalog.into_leads(),
        candidate_decomposition: candidate.tree,
        expected,
    });
    let digest = proposal.save(layout)?;
    journal::append_one(
        layout,
        &Event::new(
            now,
            EventKind::PlanCorrectionRecorded {
                domain: domain.clone(),
                intent: input.intent,
                constraint: input.constraint,
                children: input.children,
                proposal: Some(digest.clone()),
            },
        ),
    )?;
    Ok(CorrectOutcome::Proposed {
        domain,
        proposal: digest,
    })
}

/// Neutral floor assessment for an operator-directed re-cut.
const NEUTRAL: Assessment = Assessment {
    behavioural_breadth: 1,
    coupling: 1,
    uncertainty: 1,
    context_volume: 1,
    verification_surface: 1,
};

/// A `--domain` on an authored plan: a node id verbatim, or a leaf
/// slice name resolving to its nearest domain (the parent).
fn resolve_domain(tree: &Decomposition, named: &str) -> Result<String, Error> {
    if tree.nodes.contains_key(named) {
        return Ok(named.to_string());
    }
    if let Ok(leaf_id) = tree.leaf_id(named) {
        return Ok(tree
            .nodes
            .get(leaf_id)
            .and_then(|node| node.parent.clone())
            .unwrap_or_else(|| tree.root.clone()));
    }
    Err(Error::Diag {
        code: "decomposition-node-unknown",
        detail: format!("no decomposition node or leaf slice `{named}`"),
    })
}

/// The corrected domain's bound profile reference (its sole target,
/// else the plan's first target).
fn domain_profile(tree: &Decomposition, plan: &Plan, domain: &str) -> Result<ProfileRef, Error> {
    let node = tree.node(domain)?;
    let target = node
        .target_set()
        .into_iter()
        .next()
        .map(str::to_string)
        .or_else(|| plan.targets.keys().next().cloned())
        .ok_or_else(|| Error::Diag {
            code: "plan-profile-missing",
            detail: format!("domain `{domain}` binds no target and the plan has none"),
        })?;
    let bound = tree.profiles.get(&target).ok_or_else(|| Error::Diag {
        code: "plan-profile-missing",
        detail: format!("target `{target}` has no recorded profile"),
    })?;
    Ok(ProfileRef {
        id: bound.id.clone(),
        digest: bound.digest.clone(),
    })
}

/// A correction-driven re-cut must still reduce; an uncovering cut
/// refuses without mutating the live tree.
fn map_non_reducing(err: Error) -> Error {
    match err {
        Error::Validation { code, detail }
            if code.as_ref() == "decomposition-non-reducing"
                || code.as_ref() == "decomposition-lead-uncovered" =>
        {
            Error::validation_failed(
                "plan-correction-non-reducing",
                "a corrected cut still strictly reduces its parent and covers every lead",
                detail,
            )
        }
        other => other,
    }
}
