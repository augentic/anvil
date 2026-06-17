//! Workspace variant of `init` — scaffolds a registry-only platform
//! workspace (`registry.yaml` plus `project.yaml { workspace: true }`).
//! Refuses to run when `.specify/` already exists.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use specify_error::{Error, is_kebab};

use crate::config::{Layout, ProjectConfig};
use crate::init::cache::ManifestMeta;
use crate::init::{
    InitOptions, InitResult, resolve_version, resolved_name, scaffold_wasm_pkg_config,
    upsert_gitignore,
};
use crate::registry::Registry;

/// Scaffold a registry-only workspace.
///
/// On-disk shape after success:
///
/// ```text
/// <project_dir>/
/// ├── registry.yaml     # { version: 1, projects: [] }
/// └── .specify/
///     └── project.yaml  # { name: …, workspace: true }
/// ```
///
/// `registry.yaml` is the one platform-component artefact init
/// scaffolds — bootstrapping a workspace *is* bootstrapping its
/// registry. `change.md` and `plan.yaml` stay operator-managed even on
/// a workspace; the operator runs `/spec:plan <name>`
/// (which scaffolds both files atomically) when the work itself begins.
///
/// Adapter resolution is intentionally skipped — there is no
/// `pipeline.define` for a workspace to walk.
///
/// # Errors
///
/// Returns an error if [`InitOptions::adapter`] is set (mutually
/// exclusive with `--workspace`), if the project name is not kebab-case,
/// if `.specify/` already exists, or if any filesystem write fails.
pub(super) fn run(opts: InitOptions<'_>) -> Result<InitResult, Error> {
    if opts.adapter.is_some() {
        return Err(Error::Diag {
            code: "init-requires-adapter-or-workspace",
            detail: "pass <adapter> or --workspace".to_string(),
        });
    }

    let layout = Layout::new(opts.project_dir);
    let specify_dir = layout.specify_dir();
    if specify_dir.exists() {
        return Err(Error::Diag {
            code: "workspace-init-specify-dir-exists",
            detail: format!(
                "init --workspace: refusing to scaffold over an existing `.specify/` at {}; \
                 remove it first or run without --workspace for a regular project",
                specify_dir.display()
            ),
        });
    }

    let name = resolved_name(opts.project_dir, opts.name);
    if !is_kebab(&name) {
        return Err(Error::Diag {
            code: "workspace-init-name-not-kebab",
            detail: format!(
                "init --workspace: project name `{name}` must be kebab-case \
                 (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens). \
                 Pass --name <kebab-name> to override the directory basename."
            ),
        });
    }

    fs::create_dir_all(&specify_dir)?;
    let directories_created: Vec<PathBuf> = vec![specify_dir];

    let specify_version = resolve_version();

    let cfg = ProjectConfig {
        name,
        description: opts.description.map(str::to_string),
        adapter: None,
        specify_version: Some(specify_version.clone()),
        rules: BTreeMap::new(),
        tools: Vec::new(),
        workspace: true,
        platforms: Vec::new(),
    };
    let config_path = layout.config_path();
    let serialised = serde_saphyr::to_string(&cfg)?;
    fs::write(&config_path, serialised)?;

    let wasm_pkg_config_written = scaffold_wasm_pkg_config(&layout)?;

    let registry = Registry {
        version: 1,
        projects: Vec::new(),
    };
    let registry_path = Registry::path(opts.project_dir);
    let registry_yaml = serde_saphyr::to_string(&registry)?;
    fs::write(&registry_path, registry_yaml)?;

    upsert_gitignore(opts.project_dir)?;

    let cache_present = ManifestMeta::path(opts.project_dir).exists();

    Ok(InitResult {
        config_path,
        adapter_name: "workspace".to_string(),
        cache_present,
        codex_present: false,
        directories_created,
        scaffolded_rule_keys: Vec::new(),
        specify_version,
        specify_version_changed: true,
        wasm_pkg_config_written,
    })
}

#[cfg(test)]
mod tests;
