//! The plan scaffold gates: kebab name, overwrite policy, and
//! `Plan::init`. Driven by the guest `plan author` orchestration;
//! Gate 1 prose and journal events stay with the owning operation.

use std::collections::BTreeMap;
use std::path::Path;

use error::Error;

use super::model::{Plan, SourceBinding};
use crate::name::is_kebab;

/// Gate a fresh plan scaffold and build the in-memory [`Plan`].
///
/// Checks the kebab-case name, then applies the overwrite policy:
/// an existing `plan.yaml` is refused unless `force`, and `--force`
/// itself only proceeds while the loaded plan is replaceable
/// (`lifecycle: pending` and every entry `pending`). Writes nothing —
/// the caller decides what to mutate before the single atomic
/// [`Plan::save`].
///
/// # Errors
///
/// - `change-name-not-kebab` when `name` is not kebab-case.
/// - `already-exists` when `plan_path` already exists and `force` is
///   false.
/// - `plan-author-not-replaceable` when `force` is set but the
///   existing plan is not replaceable.
/// - load failures from the existing plan when `force` is set.
/// - whatever `Plan::init` surfaces.
pub fn scaffold(
    plan_path: &Path, name: &str, sources: BTreeMap<String, SourceBinding>, force: bool,
) -> Result<Plan, Error> {
    if !is_kebab(name) {
        return Err(Error::Diag {
            code: "change-name-not-kebab",
            detail: format!(
                "change: name `{name}` must be kebab-case \
                 (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens)"
            ),
        });
    }
    if plan_path.exists() {
        if !force {
            return Err(Error::Diag {
                code: "already-exists",
                detail: format!(
                    "refusing to overwrite existing plan at {}; \
                     pass --force to replace a pending plan, or archive first \
                     (`emery plan archive`)",
                    plan_path.display()
                ),
            });
        }
        let existing = Plan::load(plan_path)?;
        if !existing.is_replaceable() {
            return Err(Error::validation_failed(
                "plan-author-not-replaceable",
                "plan author --force requires a replaceable plan",
                "lifecycle is approved or any entry is in-progress or done; \
                 archive the plan first (`emery plan archive`)",
            ));
        }
    }
    Plan::init(name, sources)
}
