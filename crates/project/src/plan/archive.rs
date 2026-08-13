//! Archival move of `plan.yaml` (plus optional working directory and
//! operator brief) into the archive tree.

use std::path::PathBuf;

use error::Error;
use jiff::Timestamp;

use super::execution::{collect_events, project_ladders};
use super::model::{Plan, Status};
use crate::config::Layout;
use crate::fs::move_atomic;

impl Plan {
    /// Move `plan.yaml` — plus, when present, the authoring working
    /// directory and the `change.md` brief — into
    /// `<change>/archive/plans/<plan.name>-<YYYYMMDD>{.yaml,/}`.
    ///
    /// Entries that do not project `done` from the fact union refuse
    /// the move (`plan-has-outstanding-work`) unless `force`.
    /// Destination collisions error before any file moves. Returns the
    /// archived plan path plus `Some(dir)` iff a working directory or
    /// brief was co-moved.
    ///
    /// # Errors
    ///
    /// `plan-has-outstanding-work` on non-`done` projected entries
    /// without `force`, `plan-archive-target-exists` on a destination
    /// collision, plus load and move I/O failures.
    pub fn archive(
        layout: Layout<'_>, force: bool, now: Timestamp,
    ) -> Result<(PathBuf, Option<PathBuf>), Error> {
        let path = layout.plan_path();
        let plan = Self::load(&path)?;

        if !force {
            let events = collect_events(layout)?;
            let ladders = project_ladders(&plan, &events);
            let entries: Vec<String> = plan
                .entries
                .iter()
                .filter(|entry| ladders.get(&entry.name).copied() != Some(Status::Done))
                .map(|entry| entry.name.to_string())
                .collect();
            if !entries.is_empty() {
                return Err(Error::Diag {
                    code: "plan-has-outstanding-work",
                    detail: format!("plan has outstanding non-terminal work: {entries:?}"),
                });
            }
        }

        let archive_dir = layout.archive_dir().join("plans");
        let today = now.strftime("%Y%m%d").to_string();
        let dest_plan = archive_dir.join(format!("{}-{}.yaml", plan.name, today));

        let plans_dir = layout.change_root().join("plans").join(plan.name.as_str());
        let co_move_plans = plans_dir.is_dir().then_some(plans_dir);

        let brief_src = Some(layout.change_brief_path()).filter(|p| p.is_file());

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

        std::fs::create_dir_all(&archive_dir)?;

        move_atomic(&path, &dest_plan)?;
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
