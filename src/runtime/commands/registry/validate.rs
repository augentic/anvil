//! `specify registry validate` handler.

use specify_error::Result;
use specify_workflow::config::ProjectConfig;
use specify_workflow::registry::Registry;

use super::dto::{ValidateBody, write_validate_text};
use crate::runtime::context::Ctx;

pub(super) fn run(ctx: &Ctx) -> Result<()> {
    let path = Registry::path(&ctx.project_dir).display().to_string();
    // Workspaces opt into the stricter shape via `project.yaml:workspace:
    // true`. Tolerate a missing/unparseable project.yaml here —
    // `specify registry validate` is allowed to run before `specify
    // init`, in which case there is no workspace flag to honour and the base
    // shape check is the right behaviour.
    let workspace_mode = ProjectConfig::load(&ctx.project_dir).is_ok_and(|cfg| cfg.workspace);
    let registry = Registry::load(&ctx.project_dir)?;
    if workspace_mode && let Some(reg) = registry.as_ref() {
        reg.validate_shape_workspace()?;
    }
    ctx.write(
        &ValidateBody {
            registry,
            path,
            workspace_mode,
        },
        write_validate_text,
    )?;
    Ok(())
}
