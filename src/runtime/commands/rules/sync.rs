//! `specify rules sync` handler — shared codex distribution (RM-07).
//!
//! Resolves the project's adapter source (the recorded `adapter:`
//! value, or the `--source` override) and mirrors the shared codex
//! packs into the out-of-tree `<project-cache>/codex/`, pinned to the same source/ref.
//! The codex resolver's rules-root probe then finds shared `UNI-*`
//! rules without `--rules-root`. Writes only under the codex cache.

use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::init::{CodexMeta, sync_codex};

use crate::runtime::commands::rules::cli::SyncArgs;
use crate::runtime::context::Ctx;

/// Handler entry point dispatched from `src/runtime/commands.rs`.
///
/// # Errors
///
/// Returns `rules-sync-no-adapter` when the project declares no adapter
/// and no `--source` override is given (the workspace case). Bubbles up
/// adapter-resolution and filesystem errors from [`sync_codex`].
pub fn run(ctx: &Ctx, args: &SyncArgs) -> Result<()> {
    let adapter_value = args
        .source
        .as_deref()
        .or(ctx.config.adapter.as_deref())
        .ok_or_else(|| Error::Diag {
            code: "rules-sync-no-adapter",
            detail: "this project declares no adapter (workspaces distribute no codex); \
                     pass --source <adapter> to sync the shared codex from an explicit source"
                .to_string(),
        })?
        .to_string();

    let distributed =
        sync_codex(&ctx.project_dir, &adapter_value, args.include_framework, ctx.now())?;

    let body = Body {
        distributed,
        include_framework: args.include_framework,
        source: adapter_value,
        codex_meta: distributed.then(|| CodexMeta::path(&ctx.project_dir).display().to_string()),
    };
    ctx.write(&body, write_text)
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct Body {
    /// `true` when the shared `universal/` pack was copied into the
    /// codex cache; `false` when the adapter source carries no shared
    /// tree (fail-soft).
    distributed: bool,
    /// Whether the framework `core/` pack was distributed too.
    include_framework: bool,
    /// The adapter source value the codex was pinned to.
    source: String,
    /// Path to the stamped `CodexMeta`; `None` when nothing was distributed.
    #[serde(skip_serializing_if = "Option::is_none")]
    codex_meta: Option<String>,
}

fn write_text(w: &mut dyn std::io::Write, body: &Body) -> std::io::Result<()> {
    if body.distributed {
        writeln!(w, "Synced shared codex into the per-project cache (codex/)")?;
        writeln!(w, "  source: {}", body.source)?;
        writeln!(w, "  framework core pack: {}", body.include_framework)?;
        if let Some(meta) = &body.codex_meta {
            writeln!(w, "  provenance: {meta}")?;
        }
    } else {
        writeln!(
            w,
            "No shared codex distributed: adapter source `{}` carries no \
             adapters/shared/rules/universal/ pack",
            body.source
        )?;
    }
    Ok(())
}
