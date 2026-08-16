//! Deterministic work-item identity and the ready-set projection
//! (RFC-96 D2): read-only over plan topology, slice artifacts, and
//! the fact union — nothing here dispatches or persists.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use artifacts::leads::Lead;
use diagnostics::digest::sha256_hex;
use error::Error;

use super::decomposition::Decomposition;
use super::execution::{parked_refinement, project_ladders};
use super::model::{Entry, Plan, Status};
use super::projection::{Projections, contributing_leads};
use super::scope::in_scope;
use super::status::LoopStep;
use crate::build_record::BuildRecord;
use crate::config::Layout;
use crate::journal::{ClosedPlanCoverage, Event, EventKind};
use crate::name::SliceName;
use crate::refinement::{
    Freshness, Live, empty_digest, file_digest, freshness_with, live_profile, predecessor_digest,
};
use crate::slice::SliceMetadata;
use crate::snapshot::SnapshotId;
use crate::wave::{Wave, wave_base};

/// One schedulable unit of work: a slice phase under an exact input
/// identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItem {
    /// Owning slice.
    pub slice: SliceName,
    /// The entry's bound target key.
    pub target: String,
    /// The phase this item dispatches.
    pub phase: LoopStep,
    /// Digest over the phase's canonical inputs — the identity leg of
    /// the `(slice, phase, input-digest)` key.
    pub digest: SnapshotId,
    /// Topological layer over `depends_on` (roots are layer 0).
    pub layer: usize,
    /// Position of the entry in `plan.entries` (the plan-order
    /// tiebreak).
    pub plan_index: usize,
}

impl WorkItem {
    /// The canonical sort key: target, topological layer, plan order,
    /// slice name, then phase — one deterministic dispatch order for
    /// every scheduler regardless of completion timing.
    #[must_use]
    pub fn canonical_key(&self) -> (String, usize, usize, String, u8) {
        (
            self.target.clone(),
            self.layer,
            self.plan_index,
            self.slice.to_string(),
            phase_rank(self.phase),
        )
    }
}

const fn phase_rank(phase: LoopStep) -> u8 {
    match phase {
        LoopStep::Refine => 0,
        LoopStep::Build => 1,
        LoopStep::Merge => 2,
    }
}

/// Topological layer per entry over `depends_on`.
///
/// An entry with no in-plan predecessors is layer 0; otherwise one
/// past its deepest predecessor. Computed over the whole plan (scope
/// filters do not move an entry between layers). The plan is
/// validated acyclic; a malformed residue converges to the fixpoint
/// reached after `n` passes.
#[must_use]
pub fn layers(plan: &Plan) -> HashMap<SliceName, usize> {
    let mut layer: HashMap<SliceName, usize> =
        plan.entries.iter().map(|entry| (entry.name.clone(), 0)).collect();
    for _pass in 0..plan.entries.len() {
        let mut settled = true;
        for entry in &plan.entries {
            let above = entry
                .depends_on
                .iter()
                .filter_map(|dep| layer.get(dep).copied())
                .max()
                .map_or(0, |deepest| deepest + 1);
            if layer.get(&entry.name).copied() != Some(above) {
                layer.insert(entry.name.clone(), above);
                settled = false;
            }
        }
        if settled {
            break;
        }
    }
    layer
}

/// Project the ready set: at most one work item per in-scope,
/// not-yet-done entry whose next phase is dispatchable now, in the
/// canonical order ([`WorkItem::canonical_key`]).
///
/// A work item is keyed `(slice, phase, input-digest)`: the digest
/// covers the phase's canonical inputs, so changed coverage mints a
/// new identity and a completed item never re-runs.
///
/// # Readiness per phase
///
/// - **refine** — the manifest is missing or stale, the leaf is not
///   parked (boundary proposal / budget exhaustion), and every direct
///   predecessor holds a fresh or archived manifest (RFC-91 D3).
/// - **build** — the manifest is fresh and every predecessor is
///   projected `done`. Epoch admission stays the execute drain's gate
///   (the gap gate defers, never blocks), but the item's *identity*
///   covers the newest epoch so re-authorization mints new items.
/// - **merge** — a build record exists and its recorded base is the
///   target's current accepted frontier. A record whose base moved
///   cannot merge: the projection emits a **build** item under the
///   new frontier instead (identity-level stale-base requeue).
///
/// # Errors
///
/// Plan / slice-tree / journal I/O failures from the digest recomputes.
pub fn ready_set(
    plan: &Plan, layout: Layout<'_>, events: &[Event], inventory: &[Lead], live: &mut Live,
) -> Result<Vec<WorkItem>, Error> {
    let ladders = project_ladders(plan, events);
    let layer_map = layers(plan);
    let epoch = epoch_digest(events);
    let mut items = Vec::new();

    for (plan_index, entry) in plan.entries.iter().enumerate() {
        let slice_dir = layout.slice_dir(entry.name.as_str());
        let meta = SliceMetadata::load_optional(&slice_dir)?;
        if !in_scope(plan, entry, meta.as_ref()) {
            continue;
        }
        if ladders.get(&entry.name).copied() == Some(Status::Done) {
            continue;
        }
        let layer = layer_map.get(&entry.name).copied().unwrap_or_default();
        let at = |phase: LoopStep, digest: SnapshotId| WorkItem {
            slice: entry.name.clone(),
            target: entry.target.clone(),
            phase,
            digest,
            layer,
            plan_index,
        };

        if BuildRecord::present(&slice_dir) {
            let record = BuildRecord::load_latest(&slice_dir)?;
            let frontier = wave_base(layout, events, plan, &entry.target)?;
            if record.base == frontier && wave_fresh(layout, entry, &record)? {
                // A wave commits only after every frozen member holds
                // a record (RFC-96 D7): a member awaiting its
                // siblings contributes no item this round.
                let wave = Wave::load(layout, &entry.target, record.wave.as_str())?;
                if wave.records_complete(layout)? {
                    items.push(at(LoopStep::Merge, merge_digest(&record, &frontier)?));
                }
            } else {
                // A moved frontier or a retracted wave changed the
                // input digest: the scheduler emits a new build item
                // — never a retry (RFC-96 D2/D7).
                items.push(at(
                    LoopStep::Build,
                    build_digest(layout, entry, &slice_dir, epoch.as_ref(), &frontier)?,
                ));
            }
            continue;
        }

        match freshness_with(layout, plan, entry, inventory, live)? {
            Freshness::Fresh { .. } => {
                let accepted = entry
                    .depends_on
                    .iter()
                    .all(|dep| ladders.get(dep).copied() == Some(Status::Done));
                if accepted {
                    let frontier = wave_base(layout, events, plan, &entry.target)?;
                    items.push(at(
                        LoopStep::Build,
                        build_digest(layout, entry, &slice_dir, epoch.as_ref(), &frontier)?,
                    ));
                }
            }
            Freshness::Missing | Freshness::Stale { .. } => {
                if parked_refinement(layout, events, entry.name.as_str())?.is_some() {
                    continue;
                }
                if predecessors_refined(layout, plan, entry, inventory, live)? {
                    items
                        .push(at(LoopStep::Refine, refine_digest(layout, plan, entry, inventory)?));
                }
            }
        }
    }

    items.sort_by_key(WorkItem::canonical_key);
    Ok(items)
}

/// Whether the record's frozen wave is still live: the manifest loads
/// and every member's refinement matches its frozen binding. A
/// missing or unreadable manifest retracts the wave (RFC-96 D7).
fn wave_fresh(layout: Layout<'_>, entry: &Entry, record: &BuildRecord) -> Result<bool, Error> {
    Wave::load(layout, &entry.target, record.wave.as_str())
        .map_or(Ok(false), |wave| wave.members_fresh(layout))
}

/// Every direct predecessor holds a fresh manifest or an archived one
/// (an accepted predecessor satisfies "predecessor refined" a
/// fortiori — RFC-91 D3).
fn predecessors_refined(
    layout: Layout<'_>, plan: &Plan, entry: &Entry, inventory: &[Lead], live: &mut Live,
) -> Result<bool, Error> {
    for dep in &entry.depends_on {
        let Some(dep_entry) = plan.entries.iter().find(|e| e.name == *dep) else {
            if predecessor_digest(layout, dep.as_str())?.is_none() {
                return Ok(false);
            }
            continue;
        };
        match freshness_with(layout, plan, dep_entry, inventory, live)? {
            Freshness::Fresh { .. } => {}
            Freshness::Missing => {
                if predecessor_digest(layout, dep.as_str())?.is_none() {
                    return Ok(false);
                }
            }
            Freshness::Stale { .. } => return Ok(false),
        }
    }
    Ok(true)
}

/// The refine item's input digest: the freshness recompute set —
/// planning projections, bound profile, live baseline, per-source
/// CIDs, and ordered predecessor pins. Target guidance stays
/// recorded-only (never recomputed statically), matching the
/// freshness posture. A planning projection that no longer computes
/// folds a marker instead, so shape drift still moves the identity.
fn refine_digest(
    layout: Layout<'_>, plan: &Plan, entry: &Entry, inventory: &[Lead],
) -> Result<SnapshotId, Error> {
    let mut fold = String::from("work-item:refine\n");
    let tree = Decomposition::load_opt(&layout.decomposition_path()).ok().flatten();
    let declared = crate::config::ProjectConfig::load(layout.project_dir())
        .ok()
        .and_then(|config| config.adapter);
    let planning = contributing_leads(entry, inventory).and_then(|contributing| {
        Projections::compute_with(plan, entry, &contributing, declared.as_deref(), tree.as_ref())
    });
    match planning {
        Ok(planning) => {
            let _ = writeln!(
                fold,
                "entry={}\nleads={}\ndecomposition={}",
                planning.entry, planning.leads, planning.decomposition
            );
        }
        Err(err) => {
            let _ = writeln!(fold, "planning-unavailable={err}");
        }
    }
    let _ = writeln!(fold, "profile={}", live_profile(plan, entry));
    let _ = writeln!(fold, "baseline={}", super::dir_cid(&layout.specs_dir())?);
    let mut sources: BTreeMap<&str, SnapshotId> = BTreeMap::new();
    for binding in &entry.sources {
        let key = binding.source();
        if let Some(plan_binding) = plan.sources.get(key)
            && plan_binding.value.is_none()
        {
            sources.insert(key, super::source_cid(key, plan_binding, layout.project_dir())?);
        }
    }
    for (key, cid) in &sources {
        let _ = writeln!(fold, "source:{key}={cid}");
    }
    fold_dependencies(&mut fold, layout, entry)?;
    Ok(SnapshotId::from_digest(&sha256_hex(fold.as_bytes())))
}

/// The build item's input digest: the fresh refinement manifest, the
/// newest authorization epoch's coverage, the target's wave base, and
/// the accepted predecessor identities.
fn build_digest(
    layout: Layout<'_>, entry: &Entry, slice_dir: &std::path::Path, epoch: Option<&SnapshotId>,
    frontier: &SnapshotId,
) -> Result<SnapshotId, Error> {
    let mut fold = String::from("work-item:build\n");
    let refinement = file_digest(slice_dir)?.unwrap_or_else(empty_digest);
    let _ = writeln!(fold, "refinement={refinement}");
    let _ = writeln!(fold, "epoch={}", epoch.cloned().unwrap_or_else(empty_digest));
    let _ = writeln!(fold, "base={frontier}");
    fold_dependencies(&mut fold, layout, entry)?;
    Ok(SnapshotId::from_digest(&sha256_hex(fold.as_bytes())))
}

/// The merge item's input digest: the successful build record and the
/// current accepted frontier.
fn merge_digest(record: &BuildRecord, frontier: &SnapshotId) -> Result<SnapshotId, Error> {
    let fold = format!("work-item:merge\nrecord={}\nfrontier={frontier}\n", record.digest()?);
    Ok(SnapshotId::from_digest(&sha256_hex(fold.as_bytes())))
}

/// Fold the ordered predecessor identities (live or archived
/// manifest digests; a missing one folds the canonical empty digest).
fn fold_dependencies(fold: &mut String, layout: Layout<'_>, entry: &Entry) -> Result<(), Error> {
    for dep in &entry.depends_on {
        let digest = predecessor_digest(layout, dep.as_str())?.unwrap_or_else(empty_digest);
        let _ = writeln!(fold, "dep:{dep}={digest}");
    }
    Ok(())
}

/// Digest of the newest `plan.execute.started` coverage, or `None`
/// before any epoch — folded into build identities so a
/// re-authorization mints new work items.
fn epoch_digest(events: &[Event]) -> Option<SnapshotId> {
    let coverage = events.iter().rev().find_map(|event| match &event.kind {
        EventKind::PlanExecuteStarted { coverage, .. } => Some(coverage),
        _ => None,
    })?;
    let ClosedPlanCoverage::ClosedPlan {
        plan_digest,
        refinements,
    } = coverage;
    let mut fold = format!("epoch\nplan={plan_digest}\n");
    for (slice, digest) in refinements {
        let _ = writeln!(fold, "refinement:{slice}={digest}");
    }
    Some(SnapshotId::from_digest(&sha256_hex(fold.as_bytes())))
}
