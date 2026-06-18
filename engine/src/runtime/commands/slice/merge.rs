//! `slice merge run | preview | conflict-check`. Owns the merge-side
//! JSON DTOs and summarisers; the workspace-clone auto-commit git side
//! effects live in `specify_workflow::merge::clone_commit`.

use std::io::Write;
use std::path::PathBuf;

use jiff::Timestamp;
use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::change::{Plan, Status};
use specify_workflow::config::with_state;
use specify_workflow::journal::{self, EventKind};
use specify_workflow::merge::{
    BaselineConflict, MergeOperation, MergePreviewEntry, OpaqueAction, clone_commit,
    conflict_check, slice,
};

use super::artifact_classes;
use crate::runtime::context::Ctx;

pub(super) fn run(ctx: &Ctx, name: &str, allow_composition_replace: bool) -> Result<()> {
    // Plan-lock gate: a plan-backed `merge run` writes plan state (the
    // per-entry `done` stamp) — refuse an unlocked driver before the
    // merge bracket so a refusal never journals `slice.merge.*`.
    // Standalone merges (no plan.yaml) stamp nothing and stay unguarded.
    if ctx.layout().plan_path().exists() {
        specify_workflow::plan_lock::require_held(ctx.layout())?;
    }
    // The `slice.merge.*` pair fires on the validator outcome.
    // `started` brackets entry; the fallible body runs the validator +
    // apply and (on success) the durable `slice.archive.created` ledger
    // entry; `succeeded` brackets a fully completed run. Ordering is
    // started → … → archive.created → succeeded, so the lifecycle pair
    // wraps the ledger entry rather than racing it.
    super::bracket(
        ctx,
        "slice.merge",
        EventKind::SliceMergeStarted {
            slice_name: name.into(),
        },
        EventKind::SliceMergeSucceeded {
            slice_name: name.into(),
        },
        |reason| EventKind::SliceMergeFailed {
            slice_name: name.into(),
            reason,
        },
        || commit_run(ctx, name, allow_composition_replace),
    )
}

/// Validator + apply core of `slice merge run`: commit the deltas,
/// auto-commit the workspace clone, append the outcome-ledger entry,
/// stamp the plan entry `done`, and write the run output. Wrapped by
/// [`run`] so the `slice.merge.*` lifecycle pair can bracket it.
fn commit_run(ctx: &Ctx, name: &str, allow_composition_replace: bool) -> Result<()> {
    let slice_dir = ctx.slices_dir().join(name);
    let archive_dir = ctx.archive_dir();
    let classes = artifact_classes(&ctx.project_dir, &slice_dir);

    // Single clock read for the whole merge: the commit, the
    // outcome-ledger event, and the archive path date all derive from the
    // same `now` so they cannot disagree across a midnight boundary.
    let now = ctx.now();
    let merged = slice::commit(&slice_dir, &classes, &archive_dir, now, allow_composition_replace)?;

    // The merge-owned workspace commit is limited to the baseline spec
    // tree and archived slice (opaque/generated outputs remain as residue
    // for the execute driver). The git side effects live in
    // `specify-workflow`; the handler renders the returned warnings.
    if clone_commit::is_clone_eligible(&ctx.project_dir) {
        for warning in clone_commit::auto_commit(&ctx.project_dir, name) {
            eprintln!("{warning}");
        }
    }

    // Append the durable outcome-ledger entry (DECISIONS.md §"History
    // via git plus an outcome ledger"). Best-effort: a journal write
    // failure must not undo a committed merge, so the error is logged,
    // not propagated.
    emit_archive_created(ctx, name, &merged, &merged.decisions, now);

    stamp_plan_entry_done(ctx, name)?;

    let today = now.strftime("%Y-%m-%d").to_string();
    let archive_path = archive_dir.join(format!("{today}-{name}"));

    ctx.write(
        &RunBody {
            merged_specs: &merged,
            archive_path,
        },
        write_run_text,
    )?;
    Ok(())
}

/// Append the `slice.archive.created` outcome-ledger event. Captures
/// the merged baseline spec names, a one-line summary, and the git HEAD
/// SHA after the merge (best-effort). A journal-write or git failure is
/// logged and swallowed — the merge has already committed to disk, so a
/// ledger hiccup must never surface as a non-zero exit.
fn emit_archive_created(
    ctx: &Ctx, name: &str, merged: &[MergePreviewEntry], decisions: &[String], now: Timestamp,
) {
    let touched_specs: Vec<String> = merged.iter().map(|e| e.name.clone()).collect();
    let outcome_summary = if merged.is_empty() {
        "no baseline specs touched".to_string()
    } else {
        merged
            .iter()
            .map(|e| format!("{}: {}", e.name, summarise_ops(&e.result.operations)))
            .collect::<Vec<_>>()
            .join("; ")
    };
    journal::emit_best_effort(
        ctx.layout(),
        now,
        EventKind::SliceArchiveCreated {
            slice_name: name.into(),
            touched_specs,
            outcome_summary,
            merge_sha: clone_commit::head_sha(&ctx.project_dir),
            decisions: decisions.to_vec(),
        },
        "slice.archive.created",
    );
}

/// workflow §Workflow: `/spec:merge` is the sole writer of per-entry
/// `done`. Standalone merge fixtures without `plan.yaml` skip this
/// step silently.
fn stamp_plan_entry_done(ctx: &Ctx, name: &str) -> Result<()> {
    if !ctx.layout().plan_path().exists() {
        return Ok(());
    }
    with_state::<Plan, _, _>(ctx.layout(), "plan.yaml", move |plan| {
        if !plan.entries.iter().any(|e| e.name == name) {
            return Err(Error::Diag {
                code: "plan-entry-not-found",
                detail: format!("no slice named '{name}' in plan"),
            });
        }
        plan.transition(name, Status::Done)?;
        Ok(())
    })?;
    Ok(())
}

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
struct RunBody<'a> {
    merged_specs: &'a [MergePreviewEntry],
    #[serde(skip)]
    archive_path: PathBuf,
}

fn write_run_text(w: &mut dyn Write, body: &RunBody<'_>) -> std::io::Result<()> {
    for entry in body.merged_specs {
        writeln!(w, "{}: {}", entry.name, summarise_ops(&entry.result.operations))?;
    }
    writeln!(w, "Archived to {}", body.archive_path.display())
}

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
            writeln!(w, "{}: {}", entry.name, summarise_ops(&entry.result.operations))?;
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

fn summarise_ops(ops: &[MergeOperation]) -> String {
    let mut counts: [(u32, &str); 4] =
        [(0, "added"), (0, "modified"), (0, "removed"), (0, "renamed")];
    let mut created_baseline = None;
    for op in ops {
        match op {
            MergeOperation::Added { .. } => counts[0].0 += 1,
            MergeOperation::Modified { .. } => counts[1].0 += 1,
            MergeOperation::Removed { .. } => counts[2].0 += 1,
            MergeOperation::Renamed { .. } => counts[3].0 += 1,
            MergeOperation::CreatedBaseline { requirement_count } => {
                created_baseline = Some(*requirement_count);
            }
        }
    }
    if let Some(count) = created_baseline {
        return format!("created baseline with {count} requirement(s)");
    }
    let parts: Vec<String> =
        counts.iter().filter(|(c, _)| *c > 0).map(|(c, label)| format!("{c} {label}")).collect();
    if parts.is_empty() { "no-op".to_string() } else { parts.join(", ") }
}
