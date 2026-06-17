//! Archival move of `plan.yaml` (plus optional working directory and
//! operator brief) into the archive tree.
//! §"`specify plan archive`" for the preflight + execute flow.

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use specify_error::Error;

use super::model::{Plan, Status};
use crate::slice::actions::move_atomic;

impl Plan {
    /// Move `plan.yaml` — and, when present, the Layer-2 authoring
    /// working directory `.specify/plans/<plan.name>/` and the
    /// operator brief (`change.md`) — into the archive directory.
    ///
    /// Archive semantics (`change plan archive`):
    ///
    /// 1. Load the plan at `path`.
    /// 2. Collect every entry whose status is non-terminal for archival
    ///    purposes — anything not equal to `Done`. If the list is
    ///    non-empty and `force == false`, return [`Error::Diag`] with
    ///    `code = "plan-has-outstanding-work"` carrying those names in
    ///    plan list order. When `force == true`, proceed; the archived
    ///    file preserves the statuses verbatim.
    /// 3. Preflight the on-disk destinations (before any mutation):
    ///    - `<archive_dir>/<plan.name>-<YYYYMMDD>.yaml` must not exist.
    ///    - If a co-movable working directory or change brief
    ///      exists, `<archive_dir>/<plan.name>-<YYYYMMDD>/` must not
    ///      exist either. Any collision errors out before any file
    ///      or directory is moved, so a failure here leaves the
    ///      working tree untouched.
    /// 4. Create `archive_dir` if missing.
    /// 5. Execute: move `plan.yaml` via `move_atomic`,
    ///    then (when present) move the working directory via the same
    ///    helper. It dispatches on `src.is_dir()` and does an atomic
    ///    `fs::rename` with a `copy + remove` fallback on `EXDEV`
    ///    (cross-device).
    /// 6. Return `(archived_plan_path, archived_plans_dir)` — the
    ///    second element is `Some` iff a working directory or brief
    ///    was co-moved.
    ///
    /// `change_brief_path` is the absolute location of the operator
    /// brief at `<project_dir>/change.md`. The archive co-moves the
    /// file when present so operators do not orphan it alongside the
    /// closed plan.
    ///
    /// `now` supplies the `YYYYMMDD` segment in the destination name;
    /// dispatchers pass `Timestamp::now` and tests pin a fixed value.
    ///
    /// # Errors
    ///
    /// Errors when archive targets already exist, when load/move
    /// underlying calls fail, or when entries are non-terminal without
    /// `force`.
    pub fn archive(
        path: &Path, change_brief_path: &Path, archive_dir: &Path, force: bool, now: Timestamp,
    ) -> Result<(PathBuf, Option<PathBuf>), Error> {
        let plan = Self::load(path)?;

        if !force {
            let entries: Vec<String> = plan
                .entries
                .iter()
                .filter(|c| c.status != Status::Done)
                .map(|c| c.name.to_string())
                .collect();
            if !entries.is_empty() {
                return Err(Error::Diag {
                    code: "plan-has-outstanding-work",
                    detail: format!("plan has outstanding non-terminal work: {entries:?}"),
                });
            }
        }

        let today = now.strftime("%Y%m%d").to_string();
        let dest_plan = archive_dir.join(format!("{}-{}.yaml", plan.name, today));

        let project_root = path.parent();
        let plans_dir =
            project_root.map(|root| root.join(".specify").join("plans").join(plan.name.as_str()));
        let co_move_plans = plans_dir.as_ref().filter(|p| p.is_dir()).cloned();

        let brief_src = Some(change_brief_path.to_path_buf()).filter(|p| p.is_file());

        let dest_plans_dir = (co_move_plans.is_some() || brief_src.is_some())
            .then(|| archive_dir.join(format!("{}-{}", plan.name, today)));

        if dest_plan.exists() {
            return Err(Error::Diag {
                code: "plan-archive-target-exists",
                detail: format!(
                    "archive target '{}' already exists; either move it out of the archive dir (`git mv` is safe — the path is not load-bearing) or wait until tomorrow to re-archive",
                    dest_plan.display()
                ),
            });
        }
        if let Some(dest_dir) = &dest_plans_dir
            && dest_dir.exists()
        {
            return Err(Error::Diag {
                code: "plan-archive-target-exists",
                detail: format!(
                    "archive target '{}' already exists; either move it out of the archive dir (`git mv` is safe — the path is not load-bearing) or wait until tomorrow to re-archive",
                    dest_dir.display()
                ),
            });
        }

        std::fs::create_dir_all(archive_dir)?;

        move_atomic(path, &dest_plan)?;
        if let (Some(src), Some(dst)) = (co_move_plans.as_ref(), dest_plans_dir.as_ref()) {
            move_atomic(src, dst)?;
        }
        if let (Some(src), Some(dst)) = (brief_src.as_ref(), dest_plans_dir.as_ref()) {
            std::fs::create_dir_all(dst)?;
            move_atomic(src, &dst.join("change.md"))?;
        }

        Ok((dest_plan, dest_plans_dir))
    }
}

#[cfg(test)]
mod tests;
