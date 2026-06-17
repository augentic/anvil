//! [`Plan::next_eligible`] (single-step scheduler) and the
//! [`plan_next_body`] one-shot projection behind `specify plan next`.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;
use specify_diagnostics::blocking_present;
use specify_error::Error;

use super::model::{Entry, Plan, SliceSourceBinding, Status};
use super::propose::{resolve_target, resolve_topology};
use crate::change::detect;
use crate::config::ProjectConfig;

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
    /// [`Plan::validate`].
    #[must_use]
    pub fn next_eligible(&self) -> Option<&Entry> {
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
    /// only, and `plan transition` writes `Done` only.
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
    pub fn advance_next(&mut self) -> Result<Option<&Entry>, Error> {
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

/// Why `specify plan next` returned no freshly advanced entry.
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

/// Wire body for `specify plan next` (text + JSON). At most one of
/// `next` / `active` populates per call; `reason` carries the
/// selection outcome.
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct NextBody {
    /// Name of the freshly advanced `pending → in-progress` entry.
    pub next: Option<String>,
    /// Selection reason when no entry advanced (or the active entry was
    /// returned).
    pub reason: Option<NextReason>,
    /// Name of the already-active in-progress entry, when one exists.
    pub active: Option<String>,
    /// Bound project for the advanced entry.
    pub project: Option<String>,
    /// Resolved target adapter (`name@vN`); best-effort, `None` when the
    /// topology cannot be resolved.
    pub target: Option<String>,
    /// Advanced entry description.
    pub description: Option<String>,
    /// Advanced entry source bindings.
    pub sources: Option<Vec<SliceSourceBinding>>,
}

/// One-shot `specify plan next` projection behind the dispatcher.
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
/// - Whatever [`Plan::advance_next`] surfaces (in practice unreachable —
///   `next_eligible` only selects `Pending` entries).
pub fn plan_next_body(
    plan: &mut Plan, slices_dir: &Path, config: &ProjectConfig, project_dir: &Path,
) -> Result<NextBody, Error> {
    let validate_results = plan.validate(Some(slices_dir), None);
    if blocking_present(&validate_results) {
        return Err(structural_errors());
    }
    if !detect(&plan.entries).is_empty() {
        return Err(structural_errors());
    }

    // workflow §CLI surface: "plan next returns the active in-progress
    // entry before selecting a new pending entry, and reports drained
    // only when no active or pending entries remain."
    let was_executing = plan.is_executing();
    let advanced = plan.advance_next()?;
    Ok(match advanced {
        None => {
            let reason = if plan.is_drained() { NextReason::Drained } else { NextReason::Stuck };
            NextBody {
                reason: Some(reason),
                ..NextBody::default()
            }
        }
        Some(entry) if was_executing => NextBody {
            reason: Some(NextReason::InProgress),
            active: Some(entry.name.to_string()),
            ..NextBody::default()
        },
        Some(entry) => {
            let target = resolve_topology(config, project_dir)
                .and_then(|topology| resolve_target(entry, &topology))
                .ok()
                .map(|t| t.to_string());
            NextBody {
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
        "run 'specify plan validate' for detail",
    )
}

#[cfg(test)]
mod tests;
