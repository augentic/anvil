//! Dispatcher for `specify slice *`. Owns the `match action` table and
//! the omnia `artifact_classes` synthesiser shared by `slice merge` and
//! `slice touched-specs`.

use std::path::Path;

use specify_error::{Error, Result};
use specify_workflow::config::Layout;
use specify_workflow::journal::{self, EventKind};
use specify_workflow::merge::{ArtifactClass, MergeStrategy};
use specify_workflow::slice::LifecycleStatus;

mod build;
pub mod cli;
mod lifecycle;
mod merge;
mod model;
mod provenance;
mod synthesize;
mod task;
mod touched;
mod validate;

use cli::{SliceAction, SliceMergeAction, SliceModelAction, SliceTaskAction};

use crate::runtime::context::Ctx;

/// Default omnia [`ArtifactClass`] set: `specs` (3-way merge) and
/// `contracts` (opaque replace). Single source of truth in the
/// binary; future adapter manifests should drive this through
/// `specify-adapter`.
fn artifact_classes(project_root: &Path, slice_dir: &Path) -> Vec<ArtifactClass> {
    vec![
        ArtifactClass {
            name: "specs".to_string(),
            staged_dir: slice_dir.join("specs"),
            baseline_dir: Layout::new(project_root).specify_dir().join("specs"),
            strategy: MergeStrategy::ThreeWayMerge,
        },
        ArtifactClass {
            name: "contracts".to_string(),
            staged_dir: slice_dir.join("contracts"),
            baseline_dir: project_root.join("contracts"),
            strategy: MergeStrategy::OpaqueReplace,
        },
    ]
}

/// Best-effort lifecycle bracket shared by `slice merge run` and
/// `slice build --phase finalize`. Emits `started`, runs `work`, then
/// emits `succeeded` on `Ok` (returning the value) or
/// `failed(err.variant_str())` on `Err` (re-propagating the error).
/// Every emit is best-effort under `scope`, so a journal-write failure
/// never changes the verb's exit code; the work's outcome alone drives
/// it. `scope` is the dotted event family (`slice.merge` / `slice.build`).
fn bracket<T>(
    ctx: &Ctx, scope: &str, started: EventKind, succeeded: EventKind,
    failed: impl FnOnce(String) -> EventKind, work: impl FnOnce() -> Result<T>,
) -> Result<T> {
    journal::emit_best_effort(ctx.layout(), ctx.now(), started, scope);
    match work() {
        Ok(value) => {
            journal::emit_best_effort(ctx.layout(), ctx.now(), succeeded, scope);
            Ok(value)
        }
        Err(err) => {
            // `reason` is the error's stable kebab discriminant. The
            // failed event is best-effort, but the original error still
            // propagates so the exit code is unchanged.
            journal::emit_best_effort(
                ctx.layout(),
                ctx.now(),
                failed(err.variant_str().into_owned()),
                scope,
            );
            Err(err)
        }
    }
}

pub fn run(ctx: &Ctx, action: SliceAction) -> Result<()> {
    match action {
        SliceAction::Create {
            name,
            target,
            if_exists,
        } => lifecycle::create(ctx, &name, target, if_exists),
        SliceAction::Validate { name } => validate::run(ctx, &name),
        SliceAction::Provenance { name } => provenance::run(ctx, &name),
        SliceAction::Model { action } => match action {
            SliceModelAction::Show { name } => model::show(ctx, &name),
        },
        SliceAction::Synthesize { name, dry_run, from } => {
            synthesize::run(ctx, &name, dry_run, from.as_deref())
        }
        SliceAction::Build { name, phase } => build::run(ctx, &name, phase),
        SliceAction::Merge { action } => match action {
            SliceMergeAction::Run {
                name,
                allow_composition_replace,
            } => merge::run(ctx, &name, allow_composition_replace),
            SliceMergeAction::Preview { name } => merge::preview(ctx, &name),
            SliceMergeAction::ConflictCheck { name } => merge::conflicts(ctx, &name),
        },
        SliceAction::Task { action } => match action {
            SliceTaskAction::Progress { name } => task::progress(ctx, &name),
            SliceTaskAction::Mark { name, task_number } => task::mark(ctx, &name, task_number),
        },
        SliceAction::Transition { name, target } => {
            if matches!(target, LifecycleStatus::Merged) {
                return Err(Error::Argument {
                    flag: "<target>",
                    detail: "use `specify slice merge run` to reach `merged`".to_string(),
                });
            }
            lifecycle::transition(ctx, name, target)
        }
        SliceAction::TouchedSpecs { name, scan, set } => touched::specs(ctx, name, scan, &set),
        SliceAction::Overlap { name } => touched::overlap(ctx, name),
        SliceAction::Drop { name, reason } => {
            lifecycle::discard_slice(ctx, name, reason.as_deref())
        }
    }
}
