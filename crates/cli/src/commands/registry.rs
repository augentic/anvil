//! `specify registry *` dispatcher.
//!
//! Validated edits to `registry.yaml` inside the project root. Pure
//! filesystem, so the handlers run on both sides of the seam (native
//! and in-guest, guest routing). Per-subcommand handlers live in
//! sibling modules; shared response DTOs live in `registry/dto.rs`.

mod add;
pub mod cli;
mod dto;
mod remove;
mod validate;

use error::Result;

use self::cli::RegistryAction;
use crate::context::Ctx;

/// Dispatch one parsed `specify registry` action against a loaded
/// project context.
///
/// # Errors
///
/// Propagates the per-subcommand handler's error for the dispatcher
/// boundary to map onto the exit contract.
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
