//! Compare-and-set application of a retained amendment proposal.

use std::collections::{BTreeMap, BTreeSet};

use artifacts::leads::Leads;
use error::Error;
use jiff::Timestamp;

use super::{Boundary, Frontiers, Ownership, Proposal, Repair};
use crate::config::Layout;
use crate::journal::{self, Event, EventKind, claim};
use crate::name::SliceName;
use crate::plan::decomposition::{self, Decomposition, Kind, Node, slices};
use crate::plan::execution::collect_events;
use crate::plan::leads as leads_retain;
use crate::plan::model::{Entry, Plan};
use crate::snapshot::SnapshotId;

/// Result of a successful `plan amend --proposal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// Applied proposal digest.
    pub digest: SnapshotId,
    /// Projected slice names after reprojection.
    pub slices: Vec<String>,
    /// New `leads-digest`.
    pub leads_digest: SnapshotId,
    /// New `decomposition-digest`.
    pub decomposition_digest: SnapshotId,
}

/// Validate, compare-and-set, and apply `digest`.
///
/// Envelope and definition-revision documents refuse. Live affected
/// claims and open waves refuse. Committed leaves keep identity,
/// binding, target, and dependencies. Boundary application activates
/// the candidate catalog and tree, then reprojects `plan.yaml`.
///
/// # Errors
///
/// Typed refusals for stale, live, malformed, cyclic, preserving, or
/// non-amendment proposals. Filesystem failures.
pub fn apply(layout: Layout<'_>, now: Timestamp, digest: &SnapshotId) -> Result<Applied, Error> {
    let proposal = Proposal::load(layout, digest)?;
    let plan_path = layout.plan_path();
    let plan = Plan::load(&plan_path)?;
    match proposal {
        Proposal::Envelope(_) | Proposal::Revision(_) => Err(Error::validation_failed(
            "plan-proposal-kind",
            "amend --proposal applies ownership and boundary amendments",
            "envelope and definition-revision documents are not amendments",
        )),
        Proposal::Boundary(body) => apply_boundary(layout, now, digest, plan, body),
        Proposal::Ownership(body) => apply_ownership(layout, now, digest, plan, &body),
    }
}

fn apply_boundary(
    layout: Layout<'_>, now: Timestamp, digest: &SnapshotId, plan: Plan, body: Boundary,
) -> Result<Applied, Error> {
    let live = Frontiers::live(layout, &plan)?;
    body.expected.compare(&live)?;
    refuse_live(layout, &affected_boundary(&plan, &body))?;
    body.candidate_decomposition.check().map_err(map_cycle)?;
    let projected = slices(&body.candidate_decomposition)?;
    let entries = overlay(&plan.entries, projected);
    preserve(&plan.entries, &entries, &body.expected.committed)?;
    let mut trial = plan.clone();
    trial.entries.clone_from(&entries);
    decomposition::matches_plan(&body.candidate_decomposition, &trial)?;

    let catalog = Leads::from_leads(body.candidate_leads);
    catalog.write_atomic(&layout.leads_path())?;
    let mut tree = body.candidate_decomposition;
    tree.leads_digest = SnapshotId::from_digest(&catalog.digest_hex()?);
    tree.save(&layout.decomposition_path())?;
    finish(layout, now, digest, plan, entries)
}

fn apply_ownership(
    layout: Layout<'_>, now: Timestamp, digest: &SnapshotId, plan: Plan, body: &Ownership,
) -> Result<Applied, Error> {
    let live = Frontiers::live(layout, &plan)?;
    body.expected.compare(&live)?;
    refuse_live(layout, &affected_ownership(body))?;
    let mut tree = Decomposition::load(&layout.decomposition_path())?;
    repair_tree(&mut tree, body)?;
    tree.check().map_err(map_cycle)?;
    let projected = slices(&tree)?;
    let entries = overlay(&plan.entries, projected);
    preserve(&plan.entries, &entries, &body.expected.committed)?;
    let mut trial = plan.clone();
    trial.entries.clone_from(&entries);
    decomposition::matches_plan(&tree, &trial)?;
    tree.save(&layout.decomposition_path())?;
    finish(layout, now, digest, plan, entries)
}

fn finish(
    layout: Layout<'_>, now: Timestamp, digest: &SnapshotId, mut plan: Plan, entries: Vec<Entry>,
) -> Result<Applied, Error> {
    let (leads_digest, decomposition_digest) = commit_tree(layout, &mut plan, entries)?;
    journal::append_one(
        layout,
        &Event::new(
            now,
            EventKind::PlanAmendApplied {
                digest: digest.clone(),
            },
        ),
    )?;
    Ok(Applied {
        digest: digest.clone(),
        slices: plan.entries.iter().map(|entry| entry.name.to_string()).collect(),
        leads_digest,
        decomposition_digest,
    })
}

/// Write retained revisions and reproject `plan.yaml`.
///
/// # Errors
///
/// Retention, projection-match, or filesystem failures.
pub(super) fn commit_tree(
    layout: Layout<'_>, plan: &mut Plan, entries: Vec<Entry>,
) -> Result<(SnapshotId, SnapshotId), Error> {
    let leads_digest = leads_retain::retain(layout)?;
    let decomposition_digest = decomposition::retain(layout)?;
    plan.leads_digest = Some(leads_digest.clone());
    plan.decomposition_digest = Some(decomposition_digest.clone());
    plan.entries = entries;
    let tree = Decomposition::load(&layout.decomposition_path())?;
    decomposition::matches_plan(&tree, plan)?;
    plan.save(&layout.plan_path())?;
    Ok((leads_digest, decomposition_digest))
}

pub(super) fn overlay(old: &[Entry], mut projected: Vec<Entry>) -> Vec<Entry> {
    let prior: BTreeMap<&str, &Entry> =
        old.iter().map(|entry| (entry.name.as_str(), entry)).collect();
    for entry in &mut projected {
        if let Some(was) = prior.get(entry.name.as_str()) {
            entry.description.clone_from(&was.description);
            entry.context.clone_from(&was.context);
            entry.divergence = was.divergence;
            entry.disagreements.clone_from(&was.disagreements);
            entry.authority_override.clone_from(&was.authority_override);
            entry.allow_composition_replace = was.allow_composition_replace;
        }
    }
    projected
}

fn preserve(
    old: &[Entry], next: &[Entry], committed: &BTreeMap<SliceName, SnapshotId>,
) -> Result<(), Error> {
    let next_by: BTreeMap<&str, &Entry> =
        next.iter().map(|entry| (entry.name.as_str(), entry)).collect();
    for name in committed.keys() {
        let Some(was) = old.iter().find(|entry| entry.name == *name) else {
            continue;
        };
        let Some(got) = next_by.get(name.as_str()) else {
            return Err(preserve_err(name, "committed leaf cannot be removed"));
        };
        if got.target != was.target {
            return Err(preserve_err(name, "committed leaf cannot be rebound"));
        }
        if got.sources != was.sources {
            return Err(preserve_err(name, "committed leaf cannot change source binding"));
        }
        if got.depends_on != was.depends_on {
            return Err(preserve_err(name, "committed leaf cannot be reordered behind new work"));
        }
    }
    Ok(())
}

fn preserve_err(name: &SliceName, detail: &str) -> Error {
    Error::validation_failed(
        "plan-proposal-preserve",
        "application preserves every committed leaf",
        format!("slice `{name}`: {detail}"),
    )
}

/// Refuse live affected claims. An affected *open wave* no longer
/// refuses: the applied amendment retracts the whole uncommitted wave
/// through identity — re-refined members stale the frozen manifest
/// bindings, so the scheduler requeues builds instead of merging the
/// retracted wave (RFC-96 D7).
fn refuse_live(layout: Layout<'_>, affected: &BTreeSet<SliceName>) -> Result<(), Error> {
    let events = collect_events(layout)?;
    let live_claims: BTreeSet<SliceName> =
        claim::project(&events).iter().map(|(slice, _)| slice.clone()).collect();
    let claimed: Vec<_> = live_claims.intersection(affected).map(SliceName::as_str).collect();
    if !claimed.is_empty() {
        return Err(Error::validation_failed(
            "plan-proposal-live",
            "amend --proposal refuses live affected claims",
            format!("claimed: {}", claimed.join(", ")),
        ));
    }
    Ok(())
}

fn affected_boundary(plan: &Plan, body: &Boundary) -> BTreeSet<SliceName> {
    let mut out: BTreeSet<SliceName> =
        plan.entries.iter().map(|entry| entry.name.clone()).collect();
    out.insert(body.failed_leaf.clone());
    if let Ok(projected) = slices(&body.candidate_decomposition) {
        out.extend(projected.into_iter().map(|entry| entry.name));
    }
    out
}

fn affected_ownership(body: &Ownership) -> BTreeSet<SliceName> {
    match &body.repair {
        Repair::DependsOn {
            predecessor,
            successor,
        } => BTreeSet::from([predecessor.clone(), successor.clone()]),
        Repair::FanIn { slice, children, .. } => {
            let mut out: BTreeSet<SliceName> = children.iter().cloned().collect();
            out.insert(slice.clone());
            out
        }
    }
}

fn repair_tree(tree: &mut Decomposition, body: &Ownership) -> Result<(), Error> {
    tree.node(&body.nearest)?;
    match &body.repair {
        Repair::DependsOn {
            predecessor,
            successor,
        } => {
            let pred = tree.leaf_id(predecessor.as_str())?.to_string();
            let succ = tree.leaf_id(successor.as_str())?.to_string();
            let node = tree.node_mut(&succ)?;
            if !node.depends_on.iter().any(|dep| dep == &pred) {
                node.depends_on.push(pred);
            }
        }
        Repair::FanIn {
            id,
            slice,
            target,
            children,
        } => {
            if tree.nodes.contains_key(id) {
                return Err(Error::validation_failed(
                    "plan-proposal-malformed",
                    "a fan-in repair names a new node",
                    format!("node `{id}` already exists"),
                ));
            }
            let mut leaf = Node::leaf(target, slice.clone());
            leaf.parent = Some(body.nearest.clone());
            leaf.kind = Some(Kind::Leaf);
            leaf.ownership = vec![format!("{slice}/**")];
            leaf.acceptance = Some(format!("fan-in {slice}"));
            if let Ok(parent) = tree.node(&body.nearest) {
                leaf.sources.clone_from(&parent.sources);
            }
            tree.nodes.insert(id.clone(), leaf);
            tree.node_mut(&body.nearest)?.children.push(id.clone());
            for child in children {
                let child_id = tree.leaf_id(child.as_str())?.to_string();
                let node = tree.node_mut(&child_id)?;
                if !node.depends_on.iter().any(|dep| dep == id) {
                    node.depends_on.push(id.clone());
                }
            }
        }
    }
    Ok(())
}

fn map_cycle(err: Error) -> Error {
    match err {
        Error::Validation { code, detail }
            if code.as_ref() == "cycle-in-depends-on"
                || code.as_ref() == "publication-target-cycle"
                || code.as_ref().starts_with("decomposition-") =>
        {
            Error::validation_failed(
                "plan-proposal-cycle",
                "applied tree must stay acyclic",
                detail,
            )
        }
        other => other,
    }
}
