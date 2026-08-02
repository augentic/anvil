//! `Plan::next_eligible` (single-step scheduler), the
//! [`plan_next_body`] one-shot projection, and the [`claim_next`]
//! claim kernel behind `emery plan next` and the execute loop.

use std::collections::HashMap;
use std::path::Path;

use diagnostics::has_blocking;
use error::Error;
use jiff::Timestamp;
use serde::Serialize;

use super::model::{Entry, Plan, SliceSourceBinding, Status};
use crate::adapter::Resolver;
use crate::config::{Layout, Mutation, ProjectConfig, with_state};
use crate::handler::ExecutionPaths;
use crate::journal::{self, Event, EventKind};
use crate::plan::claim_gate;

impl Plan {
    /// First entry in list order whose dependencies are all `done` and
    /// whose own status is `pending`. Returns `None` when nothing is
    /// eligible (plan finished, blocked, empty) **or when any entry is
    /// currently `in-progress`** — the driver must not pick a new
    /// change while one is active. The in-progress check runs before
    /// dependency eligibility checks.
    ///
    /// An unknown `depends_on` target is treated as "not done", so the
    /// entry is not eligible. Orphan-reference diagnostics belong to
    /// `Plan::validate`.
    #[must_use]
    pub(crate) fn next_eligible(&self) -> Option<&Entry> {
        if self.entries.iter().any(|c| c.status == Status::InProgress) {
            return None;
        }
        let status_by_name: HashMap<&str, Status> =
            self.entries.iter().map(|c| (c.name.as_str(), c.status)).collect();
        self.entries.iter().find(|c| {
            c.status == Status::Pending
                && c.depends_on
                    .iter()
                    .all(|dep| status_by_name.get(dep.as_str()).copied() == Some(Status::Done))
        })
    }

    /// Atomically advance the plan: if there is no active in-progress
    /// entry, transition the next eligible `Pending` entry to
    /// `InProgress` and return it; otherwise return the existing
    /// active entry without writing anything.
    ///
    /// This is the **only** writer of per-entry `InProgress` per
    /// workflow §CLI surface — `plan add` / `amend` write `Pending`
    /// only, and `slice merge` writes `Done` only.
    ///
    /// Returns `None` when the plan is drained (no active and no
    /// eligible pending entry).
    ///
    /// # Errors
    ///
    /// Errors when the underlying state transition is illegal —
    /// in practice unreachable since `next_eligible` filters for
    /// `Pending` entries and the only legal edge from `Pending` is
    /// `→ InProgress`.
    pub(crate) fn advance_next(&mut self) -> Result<Option<&Entry>, Error> {
        if self.is_executing() {
            return Ok(self.entries.iter().find(|e| e.status == Status::InProgress));
        }
        let Some(name) = self.next_eligible().map(|e| e.name.clone()) else {
            return Ok(None);
        };
        self.transition(&name, Status::InProgress)?;
        Ok(self.entries.iter().find(|e| e.name == name))
    }
}

/// Why `emery plan next` returned no freshly advanced entry.
///
/// Also signals when the active in-progress entry was returned instead.
/// The kebab-case wire values (`drained` / `stuck` / `in-progress`) are
/// the stable contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NextReason {
    /// No active and no eligible pending entry remain.
    Drained,
    /// Pending entries remain but all are blocked on unmet dependencies.
    Stuck,
    /// An already-active in-progress entry was returned unchanged.
    InProgress,
}

/// Wire body for `emery plan next` (text + JSON). At most one of
/// `next` / `active` populates per call; `reason` carries the
/// selection outcome.
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct NextBody {
    /// Plan name from `plan.yaml.name`.
    pub plan: String,
    /// Name of the freshly advanced `pending → in-progress` entry.
    pub next: Option<String>,
    /// Selection reason when no entry advanced (or the active entry was
    /// returned).
    pub reason: Option<NextReason>,
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

/// One-shot `emery plan next` projection behind the dispatcher.
///
/// Validates the plan, advances to the next eligible entry (the sole
/// writer of per-entry `in-progress` per workflow §CLI surface), and
/// builds the wire [`NextBody`] the dispatcher renders. The handler
/// keeps only the journal/emit bracket around this call.
///
/// `slices_dir` enables the on-disk slice cross-reference checks;
/// `config` + `project_dir` resolve the advanced entry's `$TARGET` from
/// the bound project's topology. Target resolution is best-effort: an
/// unresolvable topology leaves `target: None` rather than failing
/// (mirroring the pre-removal behaviour for entries that carried no
/// target — the build phase re-resolves the target before use).
///
/// # Errors
///
/// - [`Error::Validation`] `plan-structural-errors` when the plan has
///   blocking validate findings or a dependency cycle.
/// - Whatever `Plan::advance_next` surfaces (in practice unreachable —
///   `next_eligible` only selects `Pending` entries).
pub fn plan_next_body(
    resolver: &impl Resolver, plan: &mut Plan, slices_dir: &Path, config: &ProjectConfig,
    paths: &ExecutionPaths,
) -> Result<NextBody, Error> {
    if has_blocking(&claim_gate(plan, slices_dir)) {
        return Err(structural_errors());
    }

    // workflow §CLI surface: "plan next returns the active in-progress
    // entry before selecting a new pending entry, and reports drained
    // only when no active or pending entries remain."
    let was_executing = plan.is_executing();
    let plan_name = plan.name.to_string();
    let advanced = plan.advance_next()?;
    Ok(match advanced {
        None => {
            let reason = if plan.is_drained() { NextReason::Drained } else { NextReason::Stuck };
            NextBody {
                plan: plan_name,
                reason: Some(reason),
                ..NextBody::default()
            }
        }
        Some(entry) if was_executing => NextBody {
            plan: plan_name,
            reason: Some(NextReason::InProgress),
            active: Some(entry.name.to_string()),
            ..NextBody::default()
        },
        Some(entry) => {
            let target = crate::target_policy::best_effort_next(resolver, config, paths, entry);
            NextBody {
                plan: plan_name,
                next: Some(entry.name.to_string()),
                project: entry.project.clone(),
                target,
                description: entry.description.clone(),
                sources: Some(entry.sources.clone()),
                ..NextBody::default()
            }
        }
    })
}

fn structural_errors() -> Error {
    Error::validation_failed(
        "plan-structural-errors",
        "plan must be free of structural errors",
        "run 'emery plan validate' for detail",
    )
}

/// Claim the next plan entry: the shared kernel behind both `emery
/// plan next` and the execute loop's per-phase claim.
///
/// Runs [`plan_next_body`] inside the atomic state loop — `plan.yaml`
/// is rewritten only when an entry actually advanced (`pending →
/// in-progress`); returning the active entry or reporting
/// drained/stuck leaves the file untouched. workflow §Observability:
/// `plan.entry.advanced` fires only on a fresh advance, so a parked
/// loop leaves no advance event behind.
///
/// # Errors
///
/// - [`Error::ArtifactNotFound`] when `plan.yaml` is absent.
/// - `plan-structural-errors` and transition failures from
///   [`plan_next_body`].
/// - journal append failures for the advance event.
pub fn claim_next(
    resolver: &impl Resolver, paths: &ExecutionPaths, now: Timestamp, config: &ProjectConfig,
) -> Result<NextBody, Error> {
    let layout = Layout::new(paths.project_root());
    let slices_dir = layout.slices_dir();
    let (body, plan_name) = with_state::<Plan, _, _>(layout, "plan.yaml", move |plan| {
        let body = plan_next_body(resolver, plan, &slices_dir, config, paths)?;
        let changed = body.next.is_some();
        let pair = (body, plan.name.clone());
        Ok(if changed { Mutation::changed(pair) } else { Mutation::unchanged(pair) })
    })?;
    if let Some(advanced) = &body.next {
        let event = Event::new(
            now,
            EventKind::PlanEntryAdvanced {
                plan_name,
                slice_name: advanced.clone().into(),
            },
        );
        journal::append_one(layout, &event)?;
    }
    Ok(body)
}

/// Text rendering for `plan next`: the active or newly claimed entry
/// (labelled, with its project/target context), or the drained /
/// blocked explanation.
impl crate::handler::Render for NextBody {
    fn render(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        if let Some(active) = &self.active {
            return writeln!(w, "active: {active} (plan entry already in-progress)");
        }
        if let Some(name) = &self.next {
            writeln!(w, "next: {name}")?;
            if let Some(project) = &self.project {
                writeln!(w, "  project: {project}")?;
            }
            if let Some(target) = &self.target {
                writeln!(w, "  target: {target}")?;
            }
            return Ok(());
        }
        if self.reason == Some(NextReason::Drained) {
            return writeln!(w, "{}", super::status::drained_line(&self.plan));
        }
        writeln!(w, "no eligible entries \u{2014} remaining entries wait on unmet dependencies")
    }
}
