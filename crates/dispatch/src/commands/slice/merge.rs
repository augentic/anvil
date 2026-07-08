//! `slice merge preview | conflict-check`. Owns the merge-side JSON
//! DTOs and summarisers; `slice merge run` itself is a guest-owned
//! collapsed orchestration (`workflow_lib::orchestrate::merge`).

use std::io::Write;

use error::Result;
use serde::Serialize;
use workflow_lib::merge::{
    BaselineConflict, MergeOperation, MergePreviewEntry, OpaqueAction, conflict_check, slice,
    summarise_operations,
};

use super::artifact_classes;
use crate::context::Ctx;

pub(super) fn preview(ctx: &Ctx, name: &str) -> Result<()> {
    let slice_dir = ctx.slices_dir().join(name);
    let classes = artifact_classes(&ctx.project_dir, &slice_dir);
    let result = slice::preview(&slice_dir, &classes)?;

    // The JSON preview surface keeps its `specs` and `contracts` arrays
    // by grouping the engine's class-tagged entries by their `class_name`.
    // The literal output keys live here — alongside the omnia-default
    // synthesiser — rather than in the engine.
    let specs: Vec<&MergePreviewEntry> =
        result.three_way.iter().filter(|e| e.class_name == "specs").collect();
    let contracts: Vec<ContractItem> = result
        .opaque
        .iter()
        .filter(|e| e.class_name == "contracts")
        .map(|entry| ContractItem {
            path: entry.relative_path.clone(),
            action: entry.action,
        })
        .collect();

    ctx.write(
        &PreviewBody {
            slice_dir: slice_dir.display().to_string(),
            specs,
            contracts,
        },
        write_preview_text,
    )?;
    Ok(())
}

pub(super) fn conflicts(ctx: &Ctx, name: &str) -> Result<()> {
    let slice_dir = ctx.slices_dir().join(name);
    let classes = artifact_classes(&ctx.project_dir, &slice_dir);
    let conflicts = conflict_check(&slice_dir, &classes)?;

    ctx.write(
        &ConflictCheckBody {
            slice_dir: slice_dir.display().to_string(),
            conflicts: &conflicts,
        },
        write_conflict_check_text,
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Bodies.
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct PreviewBody<'a> {
    slice_dir: String,
    specs: Vec<&'a MergePreviewEntry>,
    contracts: Vec<ContractItem>,
}

fn write_preview_text(w: &mut dyn Write, body: &PreviewBody<'_>) -> std::io::Result<()> {
    if body.specs.is_empty() {
        writeln!(w, "No delta specs to merge.")?;
    } else {
        for entry in &body.specs {
            writeln!(w, "{}: {}", entry.name, summarise_operations(&entry.result.operations))?;
            for op in &entry.result.operations {
                writeln!(w, "  {}", operation_label(op))?;
            }
        }
    }
    if !body.contracts.is_empty() {
        writeln!(w, "\nContract changes:")?;
        for c in &body.contracts {
            let (sigil, label) = match c.action {
                OpaqueAction::Added => ("+", "added"),
                OpaqueAction::Replaced => ("~", "replaced"),
            };
            writeln!(w, "  {sigil} contracts/{} ({label})", c.path)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct ContractItem {
    path: String,
    action: OpaqueAction,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct ConflictCheckBody<'a> {
    slice_dir: String,
    conflicts: &'a [BaselineConflict],
}

fn write_conflict_check_text(
    w: &mut dyn Write, body: &ConflictCheckBody<'_>,
) -> std::io::Result<()> {
    if body.conflicts.is_empty() {
        return writeln!(w, "No baseline conflicts.");
    }
    for c in body.conflicts {
        writeln!(
            w,
            "{}: baseline modified {} (defined_at {})",
            c.adapter,
            c.baseline_modified_at.strftime("%Y-%m-%dT%H:%M:%SZ"),
            c.defined_at,
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// MergeOperation rendering.
// ---------------------------------------------------------------------------

fn operation_label(op: &MergeOperation) -> String {
    match op {
        MergeOperation::Added { id, name } => format!("ADDING: {id} — {name}"),
        MergeOperation::Modified { id, name } => format!("MODIFYING: {id} — {name}"),
        MergeOperation::Removed { id, name } => format!("REMOVING: {id} — {name}"),
        MergeOperation::Renamed {
            id,
            old_name,
            new_name,
        } => format!("RENAMING: {id} — {old_name} -> {new_name}"),
        MergeOperation::CreatedBaseline { requirement_count } => {
            format!("CREATING baseline with {requirement_count} requirement(s)")
        }
    }
}
