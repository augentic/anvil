//! `slice task progress | mark` — task list operations for a slice.

use std::io::Write;
use std::path::{Path, PathBuf};

use artifacts::atomic::bytes_write;
use artifacts::task::{Task, mark_complete, parse_tasks};
use error::{Error, Result};
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use crate::handler::{Anchor, Ctx, Render};
use crate::slice::SliceMetadata;

// ---------------------------------------------------------------------------
// slice task progress
// ---------------------------------------------------------------------------

/// Wire input for `slice task progress`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TaskProgressInput {
    /// Slice name.
    pub name: String,
}

/// `specify slice task progress <name>` — report task completion
/// counts.
#[derive(Clone, Copy, Debug)]
pub struct TaskProgress;

impl<P: Anchor> Operation<P> for TaskProgress {
    type Error = crate::handler::Error;
    type Input = TaskProgressInput;
    type Output = ProgressBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let slice_dir = cx.layout().slice_dir(&input.name);
        let tasks_path = resolve_tasks_path(&slice_dir)?;
        let tasks = std::fs::read_to_string(&tasks_path).map_err(Error::Io)?;
        let progress = parse_tasks(&tasks);

        Ok(ProgressBody {
            total: progress.total,
            complete: progress.complete,
            pending: progress.total.saturating_sub(progress.complete),
            tasks: progress.tasks,
        })
    }
}

/// Success envelope for `slice task progress`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProgressBody {
    /// Total task count.
    pub total: usize,
    /// Completed task count.
    pub complete: usize,
    /// Pending task count.
    pub pending: usize,
    /// The parsed task rows.
    pub tasks: Vec<Task>,
}

impl Render for ProgressBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "{}/{} tasks complete", self.complete, self.total)?;
        for task in &self.tasks {
            let mark = if task.complete { "x" } else { " " };
            writeln!(w, "  [{}] {} {}", mark, task.number, task.description)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// slice task mark
// ---------------------------------------------------------------------------

/// Wire input for `slice task mark`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TaskMarkInput {
    /// Slice name.
    pub name: String,
    /// Task number (e.g. `1.1`).
    pub task_number: String,
}

/// `specify slice task mark <name> <task-number>` — mark a task
/// complete (idempotent).
#[derive(Clone, Copy, Debug)]
pub struct TaskMark;

impl<P: Anchor> Operation<P> for TaskMark {
    type Error = crate::handler::Error;
    type Input = TaskMarkInput;
    type Output = MarkBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let TaskMarkInput { name, task_number } = input;
        let slice_dir = cx.layout().slice_dir(&name);
        let tasks_path = resolve_tasks_path(&slice_dir)?;
        let original = std::fs::read_to_string(&tasks_path).map_err(Error::Io)?;
        let updated = mark_complete(&original, &task_number)?;
        let idempotent = updated == original;
        if !idempotent {
            bytes_write(&tasks_path, updated.as_bytes())?;
        }

        Ok(MarkBody {
            marked: task_number,
            new_content_path: tasks_path,
            idempotent,
        })
    }
}

/// Success envelope for `slice task mark`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MarkBody {
    /// The marked task number.
    pub marked: String,
    /// Path of the rewritten `tasks.md` (serialised as its display
    /// string).
    pub new_content_path: PathBuf,
    /// `true` when the task was already complete (no write).
    pub idempotent: bool,
}

impl Render for MarkBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.idempotent {
            writeln!(w, "Task {} already complete.", self.marked)
        } else {
            writeln!(w, "Marked task {} complete.", self.marked)
        }
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
