//! Dispatcher for `specify slice *`. Owns the `match action` table and
//! the omnia `artifact_classes` synthesiser shared by `slice merge` and
//! `slice touched-specs`.

use specify_error::{Error, Result};
use specify_workflow::journal::{self, EventKind};
/// Re-exported so existing dispatch and guest callers keep one import
/// path while the synthesiser itself lives in `specify-workflow`
/// (the guest refine/execute orchestrators resolve classes in-crate).
pub use specify_workflow::merge::artifact_classes;
use specify_workflow::slice::LifecycleStatus;

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

use crate::context::Ctx;

/// Best-effort lifecycle bracket shared by `slice merge run` and
/// `slice build --phase finalize` (the latter lives in the binary
/// crate, hence `pub`).
///
/// Emits `started`, runs `work`, then emits `succeeded` on `Ok`
/// (returning the value) or `failed(err.variant_str())` on `Err`
/// (re-propagating the error). Every emit is best-effort under
/// `scope`, so a journal-write failure never changes the verb's exit
/// code; the work's outcome alone drives it. `scope` is the dotted
/// event family (`slice.merge` / `slice.build`).
///
/// # Errors
///
/// Re-propagates the failure returned by `work`.
pub fn bracket<T>(
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
        SliceAction::Synthesize { name, dry_run, from } => {
            synthesize::run(ctx, &name, dry_run, from.as_deref())
        }
        // `slice build` is peeled off by both dispatchers before this
        // table (the native handler owns the extension prepare-hook
        // path; the guest routes to the build orchestrator). This
        // defensive arm keeps the match exhaustive and never collapses
        // a real run to a misleading success.
        SliceAction::Build { .. } => Err(Error::Argument {
            flag: "<command>",
            detail: "`specify slice build` dispatches outside the shared verb table".to_string(),
        }),
        // Guest-only, the mirror of `plan execute`'s refusal: the guest
        // router peels `slice refine` off into an orchestration before
        // this table; natively the phase is driven by the /spec:refine
        // skill.
        SliceAction::Refine { .. } => Err(Error::Argument {
            flag: "<command>",
            detail: "`specify slice refine` runs only in the workflow guest; natively the \
                     refine phase is driven by the /spec:refine skill"
                .to_string(),
        }),
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
