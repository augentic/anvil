//! `specify rules sync` handler — shared codex materialization (RM-07).
//!
//! Materializes the shared codex packs embedded in this binary into the
//! out-of-tree `<project-cache>/codex/`, pinned to the binary version.
//! The codex resolver's rules-root probe then finds shared `UNI-*`
//! rules without `--rules-root`. Writes only under the codex cache.

use std::path::PathBuf;

use serde::Serialize;
use specify_error::Result;
use specify_workflow::init::{CodexMeta, sync_codex};

use crate::runtime::commands::rules::cli::SyncArgs;
use crate::runtime::context::Ctx;

/// Handler entry point dispatched from `src/runtime/commands.rs`.
///
/// # Errors
///
/// Bubbles up filesystem errors from [`sync_codex`].
pub fn run(ctx: &Ctx, args: SyncArgs) -> Result<()> {
    let meta = sync_codex(&ctx.project_dir, args.include_framework, ctx.now())?;

    let body = Body {
        distributed: true,
        include_framework: meta.include_framework,
        source: meta.source,
        codex_meta: CodexMeta::path(&ctx.project_dir),
    };
    ctx.write(&body, write_text)
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct Body {
    /// Always `true`: the packs are embedded in the binary, so
    /// materialization cannot miss. Kept for envelope stability.
    distributed: bool,
    /// Whether the framework `core/` pack was materialized too.
    include_framework: bool,
    /// The binary version the codex was pinned to.
    source: String,
    /// Path to the stamped `CodexMeta`.
    codex_meta: PathBuf,
}

fn write_text(w: &mut dyn std::io::Write, body: &Body) -> std::io::Result<()> {
    writeln!(w, "Synced shared codex into the per-project cache (codex/)")?;
    writeln!(w, "  source: specify {}", body.source)?;
    writeln!(w, "  framework core pack: {}", body.include_framework)?;
    writeln!(w, "  provenance: {}", body.codex_meta.display())?;
    Ok(())
}
