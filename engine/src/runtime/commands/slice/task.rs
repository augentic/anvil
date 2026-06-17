//! `slice task progress | mark` — task list operations for a slice.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use specify_error::Result;
use specify_model::atomic::bytes_write;
use specify_model::task::{Task, mark_complete, parse_tasks};
use specify_workflow::slice::SliceMetadata;

use crate::runtime::context::Ctx;

pub(super) fn progress(ctx: &Ctx, name: &str) -> Result<()> {
    let slice_dir = ctx.slices_dir().join(name);
    let tasks_path = resolve_tasks_path(&slice_dir)?;
    let content = std::fs::read_to_string(&tasks_path)?;
    let progress = parse_tasks(&content);

    ctx.write(
        &ProgressBody {
            total: progress.total,
            complete: progress.complete,
            pending: progress.total.saturating_sub(progress.complete),
            tasks: progress.tasks,
        },
        write_progress_text,
    )?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct ProgressBody {
    total: usize,
    complete: usize,
    pending: usize,
    tasks: Vec<Task>,
}

fn write_progress_text(w: &mut dyn Write, body: &ProgressBody) -> std::io::Result<()> {
    writeln!(w, "{}/{} tasks complete", body.complete, body.total)?;
    for task in &body.tasks {
        let mark = if task.complete { "x" } else { " " };
        writeln!(w, "  [{}] {} {}", mark, task.number, task.description)?;
    }
    Ok(())
}

pub(super) fn mark(ctx: &Ctx, name: &str, task_number: String) -> Result<()> {
    let slice_dir = ctx.slices_dir().join(name);
    let tasks_path = resolve_tasks_path(&slice_dir)?;
    let original = std::fs::read_to_string(&tasks_path)?;
    let updated = mark_complete(&original, &task_number)?;
    let idempotent = updated == original;
    if !idempotent {
        bytes_write(&tasks_path, updated.as_bytes())?;
    }

    ctx.write(
        &MarkBody {
            marked: task_number,
            new_content_path: tasks_path.display().to_string(),
            idempotent,
        },
        write_mark_text,
    )?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct MarkBody {
    marked: String,
    new_content_path: String,
    idempotent: bool,
}

fn write_mark_text(w: &mut dyn Write, body: &MarkBody) -> std::io::Result<()> {
    if body.idempotent {
        writeln!(w, "Task {} already complete.", body.marked)
    } else {
        writeln!(w, "Marked task {} complete.", body.marked)
    }
}

/// Resolve the `tasks.md` path for a slice.
///
/// the workflow contract pins the per-slice tasks artifact to `<slice_dir>/tasks.md`.
/// Verbs that need the tasks path during slice-state mutation
/// can stat the file themselves before mutating.
fn resolve_tasks_path(slice_dir: &Path) -> Result<PathBuf> {
    let _metadata = SliceMetadata::load(slice_dir)?; // surface the standard "not a slice" error
    Ok(slice_dir.join("tasks.md"))
}
