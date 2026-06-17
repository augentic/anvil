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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Axis, ExtensionPermissions, ExtensionSource};

    fn tool(name: &str, version: &str, source: ExtensionSource) -> Extension {
        Extension {
            name: name.to_string(),
            version: version.to_string(),
            source,
            sha256: None,
            permissions: ExtensionPermissions::default(),
        }
    }

    #[test]
    fn merge_project_wins_and_warns() {
        let project_scope = ExtensionScope::Project {
            project_name: "demo".to_string(),
        };
        let plugin_scope = ExtensionScope::Plugin {
            axis: Axis::Target,
            plugin_slug: "contracts".to_string(),
            capability_dir: "/cap".into(),
        };

        let project = vec![(
            project_scope,
            tool("contract", "2.0.0", ExtensionSource::LocalPath("/project/contract.wasm".into())),
        )];
        let plugin = vec![
            (
                plugin_scope.clone(),
                tool("contract", "1.0.0", ExtensionSource::LocalPath("/cap/contract.wasm".into())),
            ),
            (
                plugin_scope,
                tool("other", "1.0.0", ExtensionSource::LocalPath("/cap/other.wasm".into())),
            ),
        ];

        let (merged, warnings) = merge_scoped(project, plugin);
        assert_eq!(warnings, vec!["contract".to_string()]);
        assert_eq!(
            merged.iter().map(|(_, t)| t.name.as_str()).collect::<Vec<_>>(),
            ["contract", "other"]
        );
        assert_eq!(merged[0].1.version, "2.0.0");
    }
}
