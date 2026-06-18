//! `specify tool gc` handler.

use std::fs;

use specify_error::{Error, Result};
use specify_registry::cache;

use super::dto::{GcBody, write_gc_text};
use super::{build_inventory, kept_by_scope};
use crate::runtime::context::Ctx;

pub fn run(ctx: &Ctx) -> Result<()> {
    let inventory = build_inventory(ctx)?;
    let mut kept_by_scope = kept_by_scope(&inventory);
    let mut removed = Vec::new();
    for scope in &inventory.scopes {
        let kept = kept_by_scope.remove(scope).unwrap_or_default();
        for path in cache::scan_for_gc(scope, &kept)? {
            fs::remove_dir_all(&path).map_err(|err| Error::Diag {
                code: "tool-cache-remove-failed",
                detail: format!("failed to remove tool cache directory {}: {err}", path.display()),
            })?;
            removed.push(path.display().to_string());
        }
    }
    removed.sort();

    let body = GcBody {
        removed,
        warnings: inventory.warnings,
    };
    ctx.write(&body, write_gc_text)?;
    Ok(())
}
