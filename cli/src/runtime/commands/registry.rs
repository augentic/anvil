//! `specify registry *` dispatcher. Per-subcommand handlers live in
//! sibling modules; shared response DTOs live in `registry/dto.rs`.

mod add;
pub mod cli;
mod dto;
mod remove;
mod validate;

use specify_error::Result;

use self::cli::RegistryAction;
use crate::runtime::context::Ctx;

pub fn run(ctx: &Ctx, action: RegistryAction) -> Result<()> {
    match action {
        RegistryAction::Validate => validate::run(ctx),
        RegistryAction::Add {
            name,
            url,
            adapter,
            description,
        } => add::run(ctx, name, url, adapter, description),
        RegistryAction::Remove { name } => remove::run(ctx, name),
    }
}
