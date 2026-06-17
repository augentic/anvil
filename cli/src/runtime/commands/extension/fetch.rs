//! `specify tool fetch` handler.

use specify_error::Result;
use specify_registry::cache::Status as CacheStatus;

use super::dto::{FetchBody, ToolFetchRow, cache_status_for, row_for, write_fetch_text};
use super::{build_inventory, select};
use crate::runtime::context::Ctx;

pub fn run(ctx: &Ctx, name: Option<&str>) -> Result<()> {
    let inventory = build_inventory(ctx)?;
    let selected = select(&inventory, name)?;
    let mut rows = Vec::with_capacity(selected.len());
    for scoped in selected {
        let before = cache_status_for(scoped)?;
        specify_registry::resolver::resolve(
            &scoped.scope,
            &scoped.tool,
            ctx.now(),
            &ctx.project_dir,
        )?;
        rows.push(ToolFetchRow {
            row: row_for(scoped)?,
            fetched: before != CacheStatus::Hit,
        });
    }

    let body = FetchBody {
        tools: rows,
        warnings: inventory.warnings,
    };
    ctx.write(&body, write_fetch_text)?;
    Ok(())
}
