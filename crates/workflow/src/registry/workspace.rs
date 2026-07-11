//! Workspace-slot inspection for registry projects.

mod git;
mod slot_problem;

use std::path::{Component, Path, PathBuf};

use error::Error;
pub use slot_problem::{
    Problem as SlotProblem, Reason as SlotProblemReason, inspect as slot_problem,
};

fn workspace_base(project_dir: &Path) -> PathBuf {
    project_dir.join("workspace")
}

fn workspace_slot_path(base: &Path, project_name: &str) -> Result<PathBuf, Error> {
    let name_path = Path::new(project_name);
    let mut components = name_path.components();
    let Some(Component::Normal(component)) = components.next() else {
        return Err(slot_escape_error(project_name));
    };
    if components.next().is_some() || component.to_string_lossy() != project_name {
        return Err(slot_escape_error(project_name));
    }
    Ok(base.join(project_name))
}

fn slot_escape_error(project_name: &str) -> Error {
    Error::Diag {
        code: "workspace-slot-name-invalid",
        detail: format!(
            "registry project name `{project_name}` would escape `workspace/<project>/`; \
             project names must be a single path component"
        ),
    }
}

fn registry_symlink_target(project_dir: &Path, url: &str) -> Result<PathBuf, Error> {
    let path = if url == "." { project_dir.to_path_buf() } else { project_dir.join(url) };
    std::fs::canonicalize(&path).map_err(|err| Error::Diag {
        code: "workspace-registry-url-unresolved",
        detail: format!(
            "could not resolve registry url `{url}` relative to {}: {err}",
            project_dir.display()
        ),
    })
}
