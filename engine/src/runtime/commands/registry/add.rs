//! `specify registry add` handler.

use specify_error::{Error, Result, is_kebab};
use specify_model::atomic::yaml_write;
use specify_workflow::registry::{Registry, RegistryProject};

use super::dto::{AddBody, write_add_text};
use crate::runtime::context::Ctx;

pub(super) fn run(
    ctx: &Ctx, name: String, url: String, adapter: Option<String>, description: Option<String>,
) -> Result<()> {
    if !is_kebab(&name) {
        return Err(Error::Diag {
            code: "registry-add-name-not-kebab",
            detail: format!(
                "registry add: project name `{name}` must be kebab-case \
                 (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens)"
            ),
        });
    }

    let registry_path = Registry::path(&ctx.project_dir);
    let path = registry_path.display().to_string();
    let workspace_mode = ctx.config.workspace;
    // `--adapter` is an optional greenfield scaffold seed only.
    let candidate = RegistryProject {
        name,
        url,
        adapter: adapter.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        description: description.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        contracts: None,
        greenfield_seed: None,
    };

    // `registry add` is "create or update": an absent `registry.yaml`
    // is synthesised from the canonical empty shape so the first
    // `add` against a fresh project succeeds without a separate
    // bootstrap step.
    let mut registry = Registry::load(&ctx.project_dir)?.unwrap_or_else(|| Registry {
        version: 1,
        projects: Vec::new(),
    });

    if registry.projects.iter().any(|p| p.name == candidate.name) {
        return Err(Error::Diag {
            code: "registry-add-name-duplicate",
            detail: format!("registry add: project `{}` already exists in {path}", candidate.name),
        });
    }

    let added = candidate.clone();
    registry.projects.push(candidate);

    // Surface validate_shape / validate_shape_workspace errors verbatim —
    // their diagnostic codes (`description-missing-multi-repo`,
    // `workspace-cannot-be-project`, etc.) are the documented contract.
    // Returning Err here aborts before the atomic write, so the
    // on-disk registry is never left in a shape-invalid state.
    if workspace_mode {
        registry.validate_shape_workspace()?;
    } else {
        registry.validate_shape()?;
    }

    yaml_write(&registry_path, &registry)?;

    ctx.write(
        &AddBody {
            registry,
            path,
            added,
        },
        write_add_text,
    )?;
    Ok(())
}
