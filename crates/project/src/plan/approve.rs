//! The Gate 1 stamp kernel: `pending → approved` on the single active
//! plan. Invoking `emery plan execute` is the approval act — the guest
//! execute loop stamps here before its first status projection.

use error::Error;
use jiff::Timestamp;

use super::model::{Lifecycle, Plan};
use crate::config::{Layout, Mutation, with_state};
use crate::journal::{self, Actor, Event, EventKind};

/// Stamp Gate 1 on the single active plan (`pending → approved`) and
/// journal one `plan.transition.approved` event carrying `actor`.
/// Returns `true` when this invocation performed the stamp.
///
/// Idempotent: an already-`approved` plan is a no-op (no disk write,
/// no journal event, returns `false`) — a repeated stamp must not
/// double-fire `plan.transition.approved`.
///
/// # Errors
///
/// - [`Error::ArtifactNotFound`] when `plan.yaml` does not exist.
/// - `plan-lifecycle-transition` on an illegal edge (unreachable for
///   the two-state lifecycle; kept as the kernel's backstop).
/// - I/O and YAML failures from the atomic write or journal append.
pub fn stamp_approved(layout: Layout<'_>, now: Timestamp, actor: Actor) -> Result<bool, Error> {
    let event = with_state::<Plan, _, _>(layout, "plan.yaml", move |plan| {
        if matches!(plan.lifecycle, Lifecycle::Approved) {
            return Ok(Mutation::unchanged(None));
        }
        plan.transition_lifecycle(Lifecycle::Approved)?;
        Ok(Mutation::changed(Some(EventKind::PlanTransitionApproved {
            plan_name: plan.name.clone(),
            actor,
        })))
    })?;
    if let Some(kind) = event {
        journal::append_one(layout, &Event::new(now, kind))?;
        return Ok(true);
    }
    Ok(false)
}
