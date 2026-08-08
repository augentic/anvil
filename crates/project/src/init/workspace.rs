//! Workspace variant of `init` — scaffolds a registry-only platform
//! workspace (`registry.yaml` plus `project.yaml { workspace: true }`).
//! Refuses to run when `.emery/` already exists.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use error::Error;

use crate::config::{Layout, ProjectConfig};
use crate::init::{InitOptions, InitResult, resolve_version, resolved_name, upsert_gitignore};
use crate::name::is_kebab;
use crate::registry::Registry;

/// Scaffold a registry-only workspace.
///
/// On-disk shape after success:
///
/// ```text
/// <project_dir>/
/// ├── registry.yaml     # { version: 1, projects: [] }
/// └── .emery/
///     └── project.yaml  # { name: …, workspace: true }
/// ```
///
/// Adapter resolution is intentionally skipped — a workspace binds no
/// adapter of its own; member projects declare theirs.
///
/// # Errors
///
/// Returns an error if [`InitOptions::adapter`] is set (mutually
/// exclusive with `--workspace`), if the project name is not kebab-case,
/// if `.emery/` already exists, or if any filesystem write fails.
pub(super) fn run(opts: InitOptions<'_>) -> Result<InitResult, Error> {
    if opts.adapter.is_some() {
        return Err(Error::Diag {
            code: "init-requires-adapter-or-workspace",
            detail: "pass <adapter> or --workspace".to_string(),
        });
    }

    let layout = Layout::new(opts.project_dir);
    let emery_dir = layout.emery_dir();
    if emery_dir.exists() {
        return Err(Error::Diag {
            code: "workspace-init-emery-dir-exists",
            detail: format!(
                "init --workspace: refusing to scaffold over an existing `.emery/` at {}; \
                 remove it first or run without --workspace for a regular project",
                emery_dir.display()
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

    fs::create_dir_all(&emery_dir)?;
    let directories_created: Vec<PathBuf> = vec![emery_dir];

    let emery_version = resolve_version();

    let cfg = ProjectConfig {
        name,
        description: opts.description.map(str::to_string),
        adapter: None,
        emery_version: Some(emery_version.clone()),
        rules: BTreeMap::new(),
        workspace: true,
        platforms: Vec::new(),
    };
    let config_path = layout.config_path();
    artifacts::atomic::yaml_write(&config_path, &cfg)?;

    let registry = Registry {
        version: 1,
        projects: Vec::new(),
    };
    let registry_path = Registry::path(opts.project_dir);
    artifacts::atomic::yaml_write(&registry_path, &registry)?;

    upsert_gitignore(opts.project_dir)?;

    Ok(InitResult {
        config_path,
        adapter_name: "workspace".to_string(),
        adapter_binding: None,
        // A workspace binds no adapter, so no component sidecar to
        // inspect.
        cache_present: false,
        directories_created,
        scaffolded_rule_keys: Vec::new(),
        emery_version,
        context_skip_reason: None,
    })
}
