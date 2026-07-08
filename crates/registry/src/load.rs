//! Loaders for project-scope tool declarations.
//!
//! Project-scope tools are declared in `project.yaml` `tools[]` and
//! consumed by the `specify lint project` WASI path.

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
