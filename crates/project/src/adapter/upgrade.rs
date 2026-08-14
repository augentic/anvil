//! The `adapter upgrade --all` collection kernel: the bare adapter
//! names a project's recorded bindings resolve.

use std::collections::BTreeSet;
use std::path::Path;

use error::Error;

use super::AdapterSelector;
use crate::config::ProjectConfig;

/// Collect the bare adapter names bound by the project at
/// `project_dir`.
///
/// The set is the `project.yaml` target binding when it is a bare
/// name. Plan source and target rows are exact pins and never
/// contribute. The guest handler and the launcher's refresh widening
/// both call this, so the refresh set and the resolve loop agree on
/// the same names.
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
    Ok(names)
}
