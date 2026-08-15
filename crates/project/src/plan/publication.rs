//! Publication-set projection (RFC-95 D11): per-target member state
//! from the fact union — the materialize predicate, the drain gate,
//! and the topology-edit lock all read this one projection.

use std::collections::BTreeSet;

use error::Error;

use super::execution::project_ladders;
use super::model::{Plan, Status};
use super::scope::in_scope;
use crate::binding::{Location, Locator};
use crate::config::Layout;
use crate::journal::{Event, EventKind};
use crate::slice::SliceMetadata;
use crate::snapshot::SnapshotId;
use crate::wave::accepted_cid;

/// One publication member: a target with at least one in-scope entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// Target key under `plan.yaml.targets`.
    pub target: String,
    /// Repository URL from the binding locator (`url@revision` minus
    /// the revision).
    pub repository: String,
    /// The recorded parent Git revision from the binding locator.
    pub parent_revision: String,
    /// Every in-scope entry bound to this target projects `done`.
    pub complete: bool,
    /// An unacknowledged postflight failure names one of this
    /// target's entries — materialize must wait for the ack.
    pub blocked: bool,
    /// Current accepted CID from the committed wave chain.
    pub accepted: Option<SnapshotId>,
    /// The covering `plan.publication.materialized` fact, when one
    /// exists for the current accepted CID.
    pub materialized: Option<Materialized>,
}

mod record;

pub use record::{
    FailureRecord, MemberRecord, Projection, PublicationState, Record, Verification, project, ranks,
};

/// The recorded materialization observation on a member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Materialized {
    /// Node-local worktree path (observation, never portable).
    pub worktree_path: String,
    /// Publication branch (`change/<plan>`).
    pub branch: String,
    /// Covering `plan.yaml` content digest recorded on the fact — the
    /// D3 trailer digest the operator copies into the pull request.
    pub plan_digest: String,
}

impl Member {
    /// The D11 fact predicate: the member awaits materialization —
    /// complete, unblocked, an accepted CID exists, and no covering
    /// fact dedupes it.
    #[must_use]
    pub const fn pending(&self) -> bool {
        self.complete && !self.blocked && self.accepted.is_some() && self.materialized.is_none()
    }
}

/// Project the publication member set: one row per distinct target of
/// the plan's in-scope entries, in first-appearance order.
///
/// # Errors
///
/// Unknown target keys, a non-Git binding locator, accepted-CID chain
/// failures, and slice-metadata I/O failures.
pub fn members(plan: &Plan, layout: Layout<'_>, events: &[Event]) -> Result<Vec<Member>, Error> {
    let current = generation(plan, events);
    let ladders = project_ladders(plan, events);
    let mut order: Vec<String> = Vec::new();
    for entry in &plan.entries {
        let meta = SliceMetadata::load_optional(&layout.slice_dir(entry.name.as_str()))?;
        if in_scope(plan, entry, meta.as_ref()) && !order.contains(&entry.target) {
            order.push(entry.target.clone());
        }
    }
    let mut projected = Vec::with_capacity(order.len());
    for target in order {
        let binding = plan.target(&target)?;
        // Publication requires an exact Git binding (`url@revision`,
        // RFC-88 D5). A path- or value-bound target has no repository
        // to publish into and therefore no publication member.
        let Some((repository, parent_revision)) = git_parts(&binding.locator) else {
            continue;
        };
        let mut complete = true;
        let mut slices: Vec<&str> = Vec::new();
        for entry in plan.entries.iter().filter(|entry| entry.target == target) {
            let meta = SliceMetadata::load_optional(&layout.slice_dir(entry.name.as_str()))?;
            if !in_scope(plan, entry, meta.as_ref()) {
                continue;
            }
            slices.push(entry.name.as_str());
            if ladders.get(&entry.name).copied() != Some(Status::Done) {
                complete = false;
            }
        }
        let accepted = accepted_cid(layout, events, &target)?;
        let materialized = accepted
            .as_ref()
            .and_then(|accepted| materialized_fact(plan, current, &target, accepted));
        projected.push(Member {
            blocked: postflight_blocked(events, &slices),
            target,
            repository,
            parent_revision,
            complete,
            accepted,
            materialized,
        });
    }
    Ok(projected)
}

/// Targets whose in-scope entries are locked against topology edits:
/// a `plan.publication.materialized` fact exists for this plan in the
/// current authoring generation (RFC-95 D11 — rejected until archive).
#[must_use]
pub fn locked_targets(plan: &Plan, events: &[Event]) -> BTreeSet<String> {
    generation(plan, events)
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::PublicationMaterialized {
                plan_name, target, ..
            } if *plan_name == plan.name => Some(target.clone()),
            _ => None,
        })
        .collect()
}

/// The current authoring generation: events at or after the latest
/// `plan.reconcile.completed` for this plan. The change events
/// directory outlives archive and plan names recur, so publication
/// facts never join across authoring generations.
fn generation<'e>(plan: &Plan, events: &'e [Event]) -> &'e [Event] {
    events
        .iter()
        .rposition(|event| {
            matches!(&event.kind, EventKind::PlanReconcileCompleted { plan_name, .. }
                if *plan_name == plan.name)
        })
        .map_or(events, |start| &events[start..])
}

/// The typed topology-lock refusal for a mutation touching `target`.
#[must_use]
pub fn locked_err(target: &str) -> Error {
    Error::Diag {
        code: "plan-publication-locked",
        detail: format!(
            "target `{target}` has a materialized publication worktree; topology edits that \
             add, remove, or rebind its in-scope entries are rejected until `emery plan archive`"
        ),
    }
}

/// Split one exact Git binding locator into `(url, revision)`;
/// `None` for any non-Git locator.
fn git_parts(locator: &str) -> Option<(String, String)> {
    match Location::parse(locator, None).ok()?.locator {
        Locator::Git { url, revision } => Some((url, revision)),
        _ => None,
    }
}

/// The covering materialized fact for `(target, accepted)` within the
/// current authoring generation (callers pass the [`generation`]
/// slice — plan names recur across changes).
fn materialized_fact(
    plan: &Plan, events: &[Event], target: &str, accepted: &SnapshotId,
) -> Option<Materialized> {
    events.iter().rev().find_map(|event| match &event.kind {
        EventKind::PublicationMaterialized {
            plan_name,
            plan_digest,
            target: recorded,
            cid,
            worktree_path,
            branch,
            ..
        } if *plan_name == plan.name && recorded == target && cid == accepted => {
            Some(Materialized {
                worktree_path: worktree_path.clone(),
                branch: branch.clone(),
                plan_digest: plan_digest.clone(),
            })
        }
        _ => None,
    })
}

/// The plan's in-scope entries — the graph publication contraction
/// and ordering run over (RFC-95 D1).
///
/// # Errors
///
/// Slice-metadata I/O failures.
pub fn in_scope_entries(
    plan: &Plan, layout: Layout<'_>,
) -> Result<Vec<super::model::Entry>, Error> {
    let mut entries = Vec::new();
    for entry in &plan.entries {
        let meta = SliceMetadata::load_optional(&layout.slice_dir(entry.name.as_str()))?;
        if in_scope(plan, entry, meta.as_ref()) {
            entries.push(entry.clone());
        }
    }
    Ok(entries)
}

/// Whether the chronologically latest postflight event naming one of
/// `slices` is an unacknowledged failure.
fn postflight_blocked(events: &[Event], slices: &[&str]) -> bool {
    for event in events.iter().rev() {
        match &event.kind {
            EventKind::MergeWavePostflightFailed { members, .. }
                if members.iter().any(|m| slices.contains(&m.as_str())) =>
            {
                return true;
            }
            EventKind::PostflightAcknowledged { slice_name }
                if slices.contains(&slice_name.as_str()) =>
            {
                return false;
            }
            _ => {}
        }
    }
    false
}
