//! Fact-based [`advance_next`] kernel behind `emery plan advance` and
//! the execute loop (RFC-86 D2 / D7 / D23).
//!
//! Advance claims the next eligible slice and appends
//! `plan.entry.advanced`. Ladder labels project from the fact union.

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::Path;

use diagnostics::has_blocking;
use error::Error;
use jiff::Timestamp;
use serde::Serialize;

use super::execution::{collect_events, next_eligible, project_ladders};
use super::model::{Entry, Plan, SliceSourceBinding, Status};
use crate::adapter::Resolver;
use crate::config::{Layout, ProjectConfig};
use crate::handler::ExecutionPaths;
use crate::journal::{self, Event, EventKind, claim};
use crate::name::SliceName;
use crate::plan::advance_gate;

/// Why `emery plan advance` returned no freshly advanced entry.
///
/// Also signals when the active in-progress entry was returned instead.
/// The kebab-case wire values (`drained` / `stuck` / `in-progress`) are
/// the stable contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdvanceReason {
    /// No active and no eligible pending entry remain.
    Drained,
    /// Pending entries remain but all are blocked on unmet dependencies.
    Stuck,
    /// An already-active in-progress entry was returned unchanged.
    InProgress,
}

/// Wire body for `emery plan advance` (text + JSON). At most one of
/// `advanced` / `active` populates per call; `reason` carries the
/// selection outcome.
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct AdvanceBody {
    /// Plan name from `plan.yaml.name`.
    pub plan: String,
    /// Name of the freshly advanced `pending → in-progress` entry.
    pub advanced: Option<String>,
    /// Selection reason when no entry advanced (or the active entry was
    /// returned).
    pub reason: Option<AdvanceReason>,
    /// Name of the already-active in-progress entry, when one exists.
    pub active: Option<String>,
    /// Bound project for the advanced entry.
    pub project: Option<String>,
    /// Resolved target adapter (`name[@vN]`, bare for an unpinned cache
    /// resolve); best-effort, `None` when the topology cannot be
    /// resolved.
    pub target: Option<String>,
    /// Advanced entry description.
    pub description: Option<String>,
    /// Advanced entry source bindings.
    pub sources: Option<Vec<SliceSourceBinding>>,
}

/// Select the advance outcome from a loaded plan + fact-projected
/// ladders. Does not append facts — [`advance_next`] owns the claim /
/// `plan.entry.advanced` writes.
///
/// # Errors
///
/// [`Error::Validation`] `plan-structural-errors` when the plan has
/// blocking validate findings or a dependency cycle.
pub fn plan_advance_body<S: BuildHasher>(
    resolver: &impl Resolver, plan: &Plan, slices_dir: &Path, config: &ProjectConfig,
    paths: &ExecutionPaths, ladders: &HashMap<SliceName, Status, S>,
) -> Result<AdvanceBody, Error> {
    if has_blocking(&advance_gate(plan, slices_dir)) {
        return Err(structural_errors());
    }

    let plan_name = plan.name.to_string();
    if let Some(entry) = next_eligible(plan, ladders) {
        let target = crate::target_policy::best_effort_advance(resolver, config, paths, entry);
        return Ok(AdvanceBody {
            plan: plan_name,
            advanced: Some(entry.name.to_string()),
            project: entry.project.clone(),
            target,
            description: entry.description.clone(),
            sources: Some(entry.sources.clone()),
            ..AdvanceBody::default()
        });
    }

    if let Some(entry) = first_in_progress(plan, ladders) {
        return Ok(AdvanceBody {
            plan: plan_name,
            reason: Some(AdvanceReason::InProgress),
            active: Some(entry.name.to_string()),
            ..AdvanceBody::default()
        });
    }

    let drained = plan.entries.is_empty() || ladders.values().all(|status| *status == Status::Done);
    Ok(AdvanceBody {
        plan: plan_name,
        reason: Some(if drained { AdvanceReason::Drained } else { AdvanceReason::Stuck }),
        ..AdvanceBody::default()
    })
}

fn first_in_progress<'a, S: BuildHasher>(
    plan: &'a Plan, ladders: &HashMap<SliceName, Status, S>,
) -> Option<&'a Entry> {
    plan.entries.iter().find(|entry| ladders.get(&entry.name).copied() == Some(Status::InProgress))
}

fn structural_errors() -> Error {
    Error::validation_failed(
        "plan-structural-errors",
        "plan must be free of structural errors",
        "run 'emery plan validate' for detail",
    )
}

/// Advance the plan one entry: the shared kernel behind both `emery
/// plan advance` and the execute loop's per-phase advance.
///
/// Claims the next eligible slice (`slice.claimed`) and appends
/// `plan.entry.advanced`. Does **not** rewrite `plan.yaml` entry
/// status (RFC-86 D2 / D7). Returning the active entry or reporting
/// drained/stuck emits nothing.
///
/// # Errors
///
/// - [`Error::ArtifactNotFound`] when `plan.yaml` is absent.
/// - `plan-structural-errors` from structural validate.
/// - `slice-claim-conflict` when another writer owns the eligible slice.
/// - journal append failures for the claim / advance facts.
pub fn advance_next(
    resolver: &impl Resolver, paths: &ExecutionPaths, now: Timestamp, config: &ProjectConfig,
) -> Result<AdvanceBody, Error> {
    let layout = Layout::new(paths.project_root());
    let plan = Plan::load(&layout.plan_path())?;
    let events = collect_events(&plan, layout)?;
    let ladders = project_ladders(&plan, &events);
    let body = plan_advance_body(resolver, &plan, &layout.slices_dir(), config, paths, &ladders)?;
    if let Some(advanced) = &body.advanced {
        let slice: SliceName = advanced.clone().into();
        let writer = journal::writer_id();
        let ownership = claim::project(&events);
        let claimed = claim::claim(&ownership, slice.clone(), &writer)?;
        journal::append_one(layout, &Event::new(now, claimed))?;
        journal::append_one(
            layout,
            &Event::new(
                now,
                EventKind::PlanEntryAdvanced {
                    plan_name: plan.name,
                    slice_name: slice,
                },
            ),
        )?;
    }
    Ok(body)
}

/// Text rendering for `plan advance`: the active or freshly advanced
/// entry (labelled, with its project/target context), or the drained /
/// blocked explanation.
impl crate::handler::Render for AdvanceBody {
    fn render(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        if let Some(active) = &self.active {
            return writeln!(w, "active: {active} (plan entry already in-progress)");
        }
        if let Some(name) = &self.advanced {
            writeln!(w, "advanced: {name}")?;
            if let Some(project) = &self.project {
                writeln!(w, "  project: {project}")?;
            }
            if let Some(target) = &self.target {
                writeln!(w, "  target: {target}")?;
            }
            return Ok(());
        }
        if self.reason == Some(AdvanceReason::Drained) {
            return writeln!(w, "{}", super::status::drained_line(&self.plan));
        }
        writeln!(w, "no eligible entries \u{2014} remaining entries wait on unmet dependencies")
    }
}
