//! The plan scaffold gates: kebab name, overwrite policy, and
//! `Plan::init`. Driven by the guest `plan author` orchestration;
//! review prose and journal events stay with the owning operation.

use std::collections::BTreeMap;
use std::path::Path;

use error::Error;

use super::model::{Plan, SourceBinding};
use crate::name::is_kebab;

/// Gate a fresh plan scaffold and build the in-memory [`Plan`].
///
/// Checks the kebab-case name, then applies the overwrite policy:
/// an existing `plan.yaml` is refused unless `force`; with `force`
/// the existing plan is recreated unconditionally. Writes nothing —
/// the caller decides what to mutate before the single atomic
/// [`Plan::save`].
///
/// # Errors
///
/// - `change-name-not-kebab` when `name` is not kebab-case.
/// - `plan-already-exists` when `plan_path` already exists and `force`
///   is false.
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
    if plan_path.exists() && !force {
        return Err(Error::Diag {
            code: "plan-already-exists",
            detail: format!(
                "refusing to overwrite existing plan at {}; \
                 pass --force to replace it",
                plan_path.display()
            ),
        });
    }
    Plan::init(name, sources)
}
