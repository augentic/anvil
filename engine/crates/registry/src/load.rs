//! Loaders and merge helpers for project and plugin tool declarations.
//!
//! Plugin-scope extensions are projected from an adapter manifest's
//! singular `extension` declaration against the installed adapter tree
//! (RFC-48 D11); the `tools.yaml` sidecar reader has been retired.

use std::collections::HashSet;

use crate::manifest::{Extension, ExtensionScope};

/// Attach a project scope to tools parsed by the binary from `ProjectConfig`.
#[must_use]
pub fn project_tools(
    project_name: impl Into<String>, tools: Vec<Extension>,
) -> Vec<(ExtensionScope, Extension)> {
    let scope = ExtensionScope::Project {
        project_name: project_name.into(),
    };
    tools.into_iter().map(|tool| (scope.clone(), tool)).collect()
}

/// Merge project and plugin declarations. Project-scope tools win on
/// name collision so operators can override plugin-shipped declarations.
#[must_use]
pub fn merge_scoped(
    project: Vec<(ExtensionScope, Extension)>, plugin: Vec<(ExtensionScope, Extension)>,
) -> (Vec<(ExtensionScope, Extension)>, Vec<String>) {
    let mut merged: Vec<(ExtensionScope, Extension)> =
        Vec::with_capacity(project.len() + plugin.len());
    let mut project_names: HashSet<String> = HashSet::new();
    let mut warnings: Vec<String> = Vec::new();

    for (scope, tool) in project {
        project_names.insert(tool.name.clone());
        merged.push((scope, tool));
    }

    for (scope, tool) in plugin {
        if project_names.contains(&tool.name) {
            warnings.push(tool.name);
            continue;
        }
        merged.push((scope, tool));
    }

    (merged, warnings)
}

// The project-wins-on-name-collision merge (project tool kept, plugin
// dropped, `tool-name-collision` warning emitted) is asserted end-to-end by
// `engine/tests/extension.rs::run::name_collision_project_scope_wins`, and the
// non-colliding plugin merge by every adapter-tool run (e.g.
// `adapter_non_zero_exit_caches_by_scope`), so the unit duplicate was deleted.
