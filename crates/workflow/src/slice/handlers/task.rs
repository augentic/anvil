//! `slice task progress | mark` — task list operations for a slice.

use std::io::Write;
use std::path::{Path, PathBuf};

use artifacts::atomic::bytes_write;
use artifacts::task::{Task, mark_complete, parse_tasks};
use error::{Error, Result};
use omnia_guest::api::{Context, Handler, Reply};
use serde::{Deserialize, Serialize};

use crate::handler::{Anchor, Ctx, Out, Render};
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
#[derive(Debug)]
pub struct TaskProgress {
    input: TaskProgressInput,
}

impl<P: Anchor> Handler<P> for TaskProgress {
    type Error = crate::handler::Error;
    type Input = TaskProgressInput;
    type Output = Out<ProgressBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let slice_dir = cx.slices_dir().join(&self.input.name);
        let tasks_path = resolve_tasks_path(&slice_dir)?;
        let content = std::fs::read_to_string(&tasks_path).map_err(Error::Io)?;
        let progress = parse_tasks(&content);

        Ok(Reply::ok(Out(ProgressBody {
            total: progress.total,
            complete: progress.complete,
            pending: progress.total.saturating_sub(progress.complete),
            tasks: progress.tasks,
        })))
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
#[derive(Debug)]
pub struct TaskMark {
    input: TaskMarkInput,
}

impl<P: Anchor> Handler<P> for TaskMark {
    type Error = crate::handler::Error;
    type Input = TaskMarkInput;
    type Output = Out<MarkBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let TaskMarkInput { name, task_number } = self.input;
        let slice_dir = cx.slices_dir().join(&name);
        let tasks_path = resolve_tasks_path(&slice_dir)?;
        let original = std::fs::read_to_string(&tasks_path).map_err(Error::Io)?;
        let updated = mark_complete(&original, &task_number)?;
        let idempotent = updated == original;
        if !idempotent {
            bytes_write(&tasks_path, updated.as_bytes())?;
        }

        Ok(Reply::ok(Out(MarkBody {
            marked: task_number,
            new_content_path: tasks_path.display().to_string(),
            idempotent,
        })))
    }
}

/// Success envelope for `slice task mark`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MarkBody {
    /// The marked task number.
    pub marked: String,
    /// Display path of the rewritten `tasks.md`.
    pub new_content_path: String,
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
