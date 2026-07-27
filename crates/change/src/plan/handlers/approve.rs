//! `plan approve` — the nameless Gate 1 stamp over the single active
//! plan.

use std::io::Write;

use error::{Error, Result};
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::config::{Mutation, with_state};
use project::handler::{Anchor, Ctx, Render};
use project::journal::{self, Event, EventKind};
use project::plan::{Lifecycle, Plan};
use serde::{Deserialize, Serialize};

use super::{Ref, plan_ref};

/// Wire input for `plan approve`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ApproveInput {
    /// Who is driving this invocation — `operator` (default) or
    /// `agent`.
    #[serde(default = "default_actor")]
    pub actor: String,
}

fn default_actor() -> String {
    "operator".to_string()
}

/// `emery plan approve`.
///
/// Stamps Gate 1 on the single active plan (`pending → approved`).
/// Nameless — there is exactly one `plan.yaml`, so no selector is
/// needed. Operator-only: `/emery:plan` MUST NOT call this verb, and
/// `/emery:execute` runs it only behind an explicit operator
/// confirmation.
///
/// Approving an already-approved plan is an idempotent no-op (exit 0,
/// no disk write, no journal event) — a repeated stamp must not
/// double-fire `plan.transition.approved`.
///
/// `actor` (default `operator`) is recorded on the
/// `plan.transition.approved` event — self-reported grading evidence
/// for eval probes, not an enforcement gate.
#[derive(Clone, Copy, Debug)]
pub struct Approve;

impl<P: Anchor> Operation<P> for Approve {
    type Error = project::handler::Error;
    type Input = ApproveInput;
    type Output = ApproveBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let actor: journal::Actor = input.actor.parse().map_err(|detail| Error::Argument {
            flag: "--actor",
            detail,
        })?;
        let plan_path = cx.layout().plan_path();
        // workflow §Observability: the lifecycle move emits exactly one
        // journal event when the on-disk state actually changed. The
        // same-state no-op path (already-`approved` plan) returns
        // `Mutation::unchanged` with no event, so neither the disk
        // write nor the emit fires.
        let (body, event) = with_state::<Plan, _, _>(cx.layout(), "plan.yaml", move |plan| {
            let previous = plan.lifecycle;
            if matches!(previous, Lifecycle::Approved) {
                return Ok(Mutation::unchanged((approve_body(plan, &plan_path, previous), None)));
            }
            plan.transition_lifecycle(Lifecycle::Approved)?;
            let event = EventKind::PlanTransitionApproved {
                plan_name: plan.name.clone(),
                actor,
            };
            Ok(Mutation::changed((approve_body(plan, &plan_path, previous), Some(event))))
        })?;
        if let Some(kind) = event {
            journal::append_one(cx.layout(), &Event::new(cx.now(), kind))?;
        }
        Ok(body)
    }
}

fn approve_body(plan: &Plan, plan_path: &std::path::Path, previous: Lifecycle) -> ApproveBody {
    ApproveBody {
        plan: plan_ref(plan, plan_path),
        previous: previous.to_string(),
        current: plan.lifecycle.to_string(),
    }
}

/// Success envelope for `plan approve`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ApproveBody {
    /// The governing plan file.
    pub plan: Ref,
    /// Lifecycle before the stamp.
    pub previous: String,
    /// Lifecycle after the stamp.
    pub current: String,
}

impl Render for ApproveBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.previous == self.current {
            // The idempotent no-op: the plan was already approved.
            writeln!(
                w,
                "Plan '{}' is already at lifecycle: {} (no-op).",
                self.plan.name, self.current
            )
        } else {
            writeln!(
                w,
                "Stamped plan '{}': lifecycle {} \u{2192} {}.",
                self.plan.name, self.previous, self.current
            )
        }
    }
}
