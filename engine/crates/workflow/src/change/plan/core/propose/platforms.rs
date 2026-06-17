//! Platform shell detection and plan-time bootstrap slice insertion.

use std::collections::HashSet;

use specify_error::{Error, Result};

use super::super::model::{Entry, Plan, SliceAuthorityOverride, Status};
use crate::Platform;

/// Consumed by [`Plan::reconcile_platforms`] to insert bootstrap slices.
///
/// Built by the Vectis shell-detect library (`specify-vectis-shell-detect`)
/// via [`crate::vectis_missing_platforms`] for Vectis-bound projects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMissingPlatforms {
    /// Project name from the topology (matches `ProjectRef.name`).
    pub project: String,
    /// Supported platforms (`core`, `ios`, `android`) declared in
    /// `project.yaml.platforms` but absent on disk.
    pub missing: Vec<Platform>,
}

/// Platforms with on-disk shell interpretations today.
const SUPPORTED_PLATFORMS: &[Platform] = &[Platform::Core, Platform::Ios, Platform::Android];

impl Plan {
    /// Insert bootstrap slices for declared-but-absent platform shells,
    /// rewiring every pre-existing feature slice to depend on them.
    ///
    /// Called as a post-pass inside the `with_state` write loop after
    /// [`Plan::propose_from`], so bootstrap slices land in the same
    /// atomic `plan.yaml` write and appear in the
    /// `plan.reconcile.completed` event's `slice-names[]`.
    ///
    /// Returns the bootstrap slice names (prepended to `self.entries`).
    /// An empty `project_missing` list (or one where every entry has an
    /// empty `missing` vec) is a no-op returning an empty list.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] (`plan-reconcile-bootstrap-name-collision`)
    /// when a bootstrap slice name collides with an existing entry.
    pub fn reconcile_platforms(
        &mut self, project_missing: &[ProjectMissingPlatforms],
    ) -> Result<Vec<String>> {
        let mut bootstrap_entries: Vec<Entry> = Vec::new();
        let mut bootstrap_names: Vec<String> = Vec::new();
        let mut names_by_project: std::collections::HashMap<&str, Vec<String>> =
            std::collections::HashMap::new();

        for pm in project_missing {
            if pm.missing.is_empty() {
                continue;
            }

            let all_supported_missing =
                SUPPORTED_PLATFORMS.iter().all(|sp| pm.missing.contains(sp));

            if all_supported_missing {
                let name = bootstrap_slice_name(&pm.project, "app-foundation", project_missing);
                bootstrap_entries.push(bootstrap_entry(&name, &pm.project, &pm.missing));
                bootstrap_names.push(name.clone());
                names_by_project.entry(&pm.project).or_default().push(name);
            } else {
                for platform in &pm.missing {
                    let raw = format!("bootstrap-{platform}");
                    let name = bootstrap_slice_name(&pm.project, &raw, project_missing);
                    bootstrap_entries.push(bootstrap_entry(
                        &name,
                        &pm.project,
                        std::slice::from_ref(platform),
                    ));
                    bootstrap_names.push(name.clone());
                    names_by_project.entry(&pm.project).or_default().push(name);
                }
            }
        }

        if bootstrap_names.is_empty() {
            return Ok(Vec::new());
        }

        // Reject name collisions with existing entries.
        let existing: HashSet<&str> = self.entries.iter().map(|e| e.name.as_str()).collect();
        for name in &bootstrap_names {
            if existing.contains(name.as_str()) {
                return Err(Error::validation_failed(
                    "plan-reconcile-bootstrap-name-collision",
                    "bootstrap slice names must not collide with existing entries",
                    format!("bootstrap slice '{name}' collides with an existing entry"),
                ));
            }
        }

        // Wire every pre-existing entry's depends_on to the bootstrap
        // slice(s) created for its bound project only.
        for entry in &mut self.entries {
            let project_name = entry.project.as_deref().unwrap_or("");
            if let Some(project_boots) = names_by_project.get(project_name) {
                for boot_name in project_boots {
                    if !entry.depends_on.contains(&crate::name::SliceName::new(boot_name)) {
                        entry.depends_on.push(crate::name::SliceName::new(boot_name));
                    }
                }
            }
        }

        // Prepend bootstrap entries.
        bootstrap_entries.append(&mut self.entries);
        self.entries = bootstrap_entries;

        Ok(bootstrap_names)
    }
}

/// Compute a bootstrap slice name. In single-project mode, the raw
/// name is used directly (e.g. `app-foundation`). In multi-project
/// mode, the project name is prepended to disambiguate
/// (e.g. `my-app-app-foundation`).
fn bootstrap_slice_name(
    project: &str, raw_name: &str, all_missing: &[ProjectMissingPlatforms],
) -> String {
    let multi_project = all_missing.iter().filter(|pm| !pm.missing.is_empty()).count() > 1;
    if multi_project { format!("{project}-{raw_name}") } else { raw_name.to_string() }
}

/// Create a bootstrap [`Entry`] bound to the given project.
fn bootstrap_entry(name: &str, project: &str, platforms: &[Platform]) -> Entry {
    use crate::name::SliceName;

    let platform_list: Vec<String> = platforms.iter().map(Platform::to_string).collect();
    Entry {
        name: SliceName::new(name),
        project: Some(project.to_string()),
        status: Status::Pending,
        depends_on: Vec::new(),
        sources: Vec::new(),
        context: Vec::new(),
        description: Some(format!("Bootstrap shell trees for: {}", platform_list.join(", "))),
        divergence: None,
        disagreements: Vec::new(),
        authority_override: SliceAuthorityOverride::default(),
    }
}
