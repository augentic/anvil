//! The `adapter upgrade --all` collection kernel: the bare adapter
//! names a project's recorded bindings resolve.

use std::collections::BTreeSet;
use std::path::Path;

use error::Error;

use super::AdapterSelector;
use crate::config::{Layout, ProjectConfig};
use crate::plan::Plan;

/// Collect the bare adapter names bound by the project at
/// `project_dir`.
///
/// The set is the `project.yaml` target binding plus every
/// `plan.yaml.sources.<key>` adapter, keeping only selectors that
/// parse as bare names. Pinned bindings are immutable and local
/// components refresh through `adapter add`, so neither joins the set.
///
/// A missing or unreadable `plan.yaml` contributes nothing — the
/// project binding alone is a valid answer. Both the guest handler
/// and the launcher's refresh widening call this kernel, so the
/// refresh set and the resolve loop agree on the same names.
///
/// # Errors
///
/// Propagates [`ProjectConfig::load`] failures — notably
/// [`Error::NotInitialized`] when `.emery/project.yaml` is absent.
pub fn targets(project_dir: &Path) -> Result<BTreeSet<String>, Error> {
    let config = ProjectConfig::load(project_dir)?;
    let mut names = BTreeSet::new();
    if let Some(adapter) = &config.adapter
        && let Ok(AdapterSelector::Bare { name }) = AdapterSelector::parse(adapter)
    {
        names.insert(name);
    }
    if let Ok(plan) = Plan::load(&Layout::new(project_dir).plan_path()) {
        for binding in plan.sources.values() {
            if let AdapterSelector::Bare { name } = binding.selector() {
                names.insert(name);
            }
        }
    }
    Ok(names)
}
