//! The plan scaffold gates: kebab name, overwrite refusal, and
//! [`Plan::init`]. Driven by the guest `plan author` orchestration;
//! Gate 1 prose and journal events stay with the owning operation.

use std::collections::BTreeMap;
use std::path::Path;

use error::Error;

use super::model::{Plan, SourceBinding};
use crate::name::is_kebab;

/// Gate a fresh plan scaffold and build the in-memory [`Plan`]:
/// kebab-case name, refuse overwriting an existing `plan.yaml`, then
/// [`Plan::init`]. Writes nothing — the caller decides what to mutate
/// before the single atomic [`Plan::save`].
///
/// # Errors
///
/// - `change-name-not-kebab` when `name` is not kebab-case.
/// - `already-exists` when `plan_path` already exists.
/// - whatever [`Plan::init`] surfaces.
pub fn scaffold(
    plan_path: &Path, name: &str, sources: BTreeMap<String, SourceBinding>,
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
        return Err(Error::Diag {
            code: "already-exists",
            detail: format!("refusing to overwrite existing plan at {}", plan_path.display()),
        });
    }
    Plan::init(name, sources)
}
