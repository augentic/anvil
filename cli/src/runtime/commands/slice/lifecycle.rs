//! Slice lifecycle handlers: create / transition / archive / drop.

use std::io::Write;

use jiff::Timestamp;
use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::slice::{CreateIfExists, Created, LifecycleStatus, actions as slice_actions};

use crate::runtime::context::Ctx;

pub(super) fn create(
    ctx: &Ctx, name: &str, target: Option<String>, if_exists: CreateIfExists,
) -> Result<()> {
    let target_value = target.map_or_else(
        || {
            ctx.config.adapter.clone().ok_or_else(|| Error::Diag {
                code: "slice-create-target-missing",
                detail: "no project target declared; pass `--target <id>` explicitly or \
                         run `specify init <adapter>` first (workspaces cannot create \
                         changes)"
                    .to_string(),
            })
        },
        Ok,
    )?;
    let slices_dir = ctx.slices_dir();
    std::fs::create_dir_all(&slices_dir)?;

    let outcome = slice_actions::create(&slices_dir, name, &target_value, if_exists, ctx.now())?;

    ctx.write(&outcome, write_create_text)?;
    Ok(())
}

fn write_create_text(w: &mut dyn Write, c: &Created) -> std::io::Result<()> {
    if c.created {
        writeln!(w, "Created slice {}", c.dir.display())?;
    } else {
        writeln!(w, "Reusing existing slice {}", c.dir.display())?;
    }
    if c.restarted {
        writeln!(w, "  (previous directory was removed)")?;
    }
    writeln!(w, "  target: {}", c.metadata.target)?;
    writeln!(w, "  status: {}", c.metadata.status)
}

pub(super) fn transition(ctx: &Ctx, name: String, target: LifecycleStatus) -> Result<()> {
    let slice_dir = ctx.slices_dir().join(&name);
    let metadata = slice_actions::transition(&slice_dir, target, ctx.now())?;
    ctx.write(
        &TransitionBody {
            name,
            status: metadata.status,
            defined_at: metadata.defined_at,
            completed_at: metadata.completed_at,
            merged_at: metadata.merged_at,
            dropped_at: metadata.dropped_at,
        },
        write_transition_text,
    )?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct TransitionBody {
    name: String,
    status: LifecycleStatus,
    #[serde(with = "specify_error::serde_rfc3339_opt")]
    defined_at: Option<Timestamp>,
    #[serde(with = "specify_error::serde_rfc3339_opt")]
    completed_at: Option<Timestamp>,
    #[serde(with = "specify_error::serde_rfc3339_opt")]
    merged_at: Option<Timestamp>,
    #[serde(with = "specify_error::serde_rfc3339_opt")]
    dropped_at: Option<Timestamp>,
}

fn write_transition_text(w: &mut dyn Write, body: &TransitionBody) -> std::io::Result<()> {
    writeln!(w, "{}: status = {}", body.name, body.status)
}

pub(super) fn discard_slice(ctx: &Ctx, name: String, reason: Option<&str>) -> Result<()> {
    let slice_dir = ctx.slices_dir().join(&name);
    let archive_dir = ctx.archive_dir();
    let (metadata, archive_path) =
        slice_actions::discard(&slice_dir, &archive_dir, reason, ctx.now())?;
    ctx.write(
        &DropBody {
            name,
            status: metadata.status,
            archive_path: archive_path.display().to_string(),
            drop_reason: metadata.drop_reason,
        },
        write_drop_text,
    )?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct DropBody {
    name: String,
    status: LifecycleStatus,
    archive_path: String,
    drop_reason: Option<String>,
}

fn write_drop_text(w: &mut dyn Write, body: &DropBody) -> std::io::Result<()> {
    writeln!(w, "{}: dropped and archived to {}", body.name, body.archive_path)?;
    if let Some(r) = &body.drop_reason {
        writeln!(w, "  reason: {r}")?;
    }
    Ok(())
}
