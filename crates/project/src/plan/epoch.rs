//! Authorization-epoch freshness (RFC-86 D22 / S19).
//!
//! The newest `plan.execute.started` coverage vs the live `plan.yaml`
//! and covered spec trees — shared by status Authorized and the gap gate.

use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::Error;

use super::execution::project_ladders;
use super::model::{Plan, Status};
use super::pins::dir_cid;
use super::scope::in_scope;
use crate::config::Layout;
use crate::journal::{ClosedPlanCoverage, Event, EventKind, LeafSpecCoverage};
use crate::slice::SliceMetadata;

/// The newest authorization epoch's relationship to the live artifacts.
#[derive(Debug)]
pub enum EpochFreshness<'a> {
    /// No `plan.execute.started` fact in the union.
    Unopened,
    /// The newest epoch no longer covers the live artifacts.
    Stale {
        /// Which covered artifact drifted, with epoch vs live digests.
        detail: String,
    },
    /// The newest epoch covers the live plan and spec trees.
    Fresh {
        /// The covering `closed-plan` coverage (leaves + waivers).
        coverage: &'a ClosedPlanCoverage,
    },
}

/// Project the newest epoch's freshness from the chronologically
/// ordered fact union (see [`super::collect_events`]).
///
/// Fresh means the newest `plan.execute.started` coverage matches the
/// live `plan.yaml` digest, every in-scope entry not yet projected
/// `done` is in the per-leaf coverage, and every such `existing`
/// leaf's specs tree still matches its covered digest. A `done` leaf
/// is skipped: merge archives the slice tree, so its absence is
/// completion under the epoch, not drift.
///
/// # Errors
///
/// Propagates `plan.yaml` / slice-tree I/O failures and a corrupt
/// `metadata.yaml` ([`Error::YamlDe`]).
pub fn freshness<'a>(
    layout: Layout<'_>, plan: &Plan, events: &'a [Event],
) -> Result<EpochFreshness<'a>, Error> {
    let Some(coverage) = newest_coverage(events) else {
        return Ok(EpochFreshness::Unopened);
    };
    let freshness = staleness(layout, plan, events, coverage)?
        .map_or(EpochFreshness::Fresh { coverage }, |detail| EpochFreshness::Stale { detail });
    Ok(freshness)
}

/// Newest `closed-plan` coverage in the union.
fn newest_coverage(events: &[Event]) -> Option<&ClosedPlanCoverage> {
    events.iter().rev().find_map(|event| match &event.kind {
        EventKind::PlanExecuteStarted { coverage, .. } => Some(coverage),
        _ => None,
    })
}

/// First drift between `coverage` and the live artifacts, or `None`
/// when the epoch is fresh.
fn staleness(
    layout: Layout<'_>, plan: &Plan, events: &[Event], coverage: &ClosedPlanCoverage,
) -> Result<Option<String>, Error> {
    let ClosedPlanCoverage::ClosedPlan {
        plan_digest, specs, ..
    } = coverage;

    let plan_bytes = std::fs::read(layout.plan_path())?;
    let live_plan = format!("sha256:{}", sha256_hex(&plan_bytes));
    if live_plan != *plan_digest {
        return Ok(Some(format!(
            "`plan.yaml` digest drifted (epoch {plan_digest}, live {live_plan}) — re-run \
             `emery plan execute`"
        )));
    }

    let ladders = project_ladders(plan, events);
    for entry in &plan.entries {
        let slice_dir = layout.slice_dir(entry.name.as_str());
        let meta = load_meta(&slice_dir)?;
        if !in_scope(plan, entry, meta.as_ref()) {
            continue;
        }
        if ladders.get(&entry.name).copied() == Some(Status::Done) {
            continue;
        }
        let name = entry.name.as_str();
        let Some(leaf) = specs.get(name) else {
            return Ok(Some(format!(
                "slice `{name}` is not in the covering epoch's per-leaf coverage — re-run \
                 `emery plan execute`"
            )));
        };
        match leaf {
            LeafSpecCoverage::Existing { digest } => {
                let live = dir_cid(&slice_dir.join("specs"))?.to_string();
                if live != *digest {
                    return Ok(Some(format!(
                        "covered spec digest for `{name}` drifted (epoch {digest}, live {live}) — \
                         re-run `emery plan execute`"
                    )));
                }
            }
            LeafSpecCoverage::RefineUnderEpoch => {
                // Epoch authorized refine-before-build; the specs
                // produced under this epoch are the covered artifact.
            }
        }
    }
    Ok(None)
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
