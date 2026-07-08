//! Dispatcher for `specify slice *`. Owns the `match action` table and
//! the omnia `artifact_classes` synthesiser shared by `slice merge` and
//! `slice touched-specs`.

use error::{Error, Result};
/// Re-exported so existing dispatch and guest callers keep one import
/// path while the synthesiser itself lives in `workflow`
/// (the guest refine/execute orchestrators resolve classes in-crate).
pub use workflow::merge::artifact_classes;
use workflow::slice::LifecycleStatus;

pub mod cli;
mod lifecycle;
mod merge;
mod model;
mod provenance;
mod task;
mod touched;
mod validate;

use cli::{SliceAction, SliceMergeAction, SliceModelAction, SliceTaskAction};

use crate::context::Ctx;

/// Dispatch one parsed `specify slice` action against the loaded `ctx`.
///
/// # Errors
///
/// Propagates the invoked handler's failure.
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
        // `slice build` / `slice refine` / `slice merge run` are
        // guest-owned collapsed orchestrations peeled off by both
        // dispatchers before this table (the native triage routes them
        // to the guest leg; the guest router routes them to
        // `workflow::orchestrate`). The defensive arms keep the
        // match exhaustive and never collapse a real run to a
        // misleading success.
        SliceAction::Build { .. } => Err(Error::Argument {
            flag: "<command>",
            detail: "`specify slice build` dispatches outside the shared verb table".to_string(),
        }),
        SliceAction::Refine { .. } => Err(Error::Argument {
            flag: "<command>",
            detail: "`specify slice refine` dispatches outside the shared verb table".to_string(),
        }),
        SliceAction::Merge { action } => match action {
            SliceMergeAction::Run { .. } => Err(Error::Argument {
                flag: "<command>",
                detail: "`specify slice merge run` dispatches outside the shared verb table"
                    .to_string(),
            }),
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
