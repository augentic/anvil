//! Dispatcher for `specify catalog *`. Owns the `match action` table
//! for the component-catalog inference verb.

pub mod cli;
mod infer;

use specify_error::Result;

use self::cli::CatalogAction;
use crate::runtime::context::Ctx;

pub fn run(ctx: &Ctx, action: CatalogAction) -> Result<()> {
    match action {
        CatalogAction::Infer {
            phase,
            min_occurrences,
            bindings,
            dry_run,
        } => infer::run(ctx, phase, min_occurrences, bindings.as_deref(), dry_run),
    }
}
