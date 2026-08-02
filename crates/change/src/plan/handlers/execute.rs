//! `plan execute` — the drained refine → build → merge loop over the
//! plan.

use std::io::Write;

use error::Error;
use omnia_guest::Model;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::Resolver;
use project::handler::{Anchor, Ctx, Render};
use project::plan::LoopStep;
use project::seam::{Source, Target, WorkingTree};
use serde::{Deserialize, Serialize};

use crate::orchestrate::{self, ExecuteOutcome};

/// Wire input for `plan execute` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct ExecuteInput {}

/// `emery plan execute` → the internal execute orchestration — the
/// drained refine → build → merge loop.
#[derive(Clone, Copy, Debug)]
pub struct Execute;

impl<P: Anchor + Model + Resolver + Source + Target> Operation<P> for Execute {
    type Error = project::handler::Error;
    type Input = ExecuteInput;
    type Output = ExecuteBody;

    async fn call(
        _input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let tree = WorkingTree::live();
        let caps = orchestrate::Capabilities::provider(context.provider);
        let outcome = orchestrate::execute(caps, &cx.paths, cx.now(), &tree).await?;
        match outcome {
            ExecuteOutcome::Drained { plan, phases } => Ok(ExecuteBody {
                status: "drained",
                plan,
                phases: phases
                    .into_iter()
                    .map(|run| ExecutePhase {
                        slice: run.slice,
                        step: run.step,
                    })
                    .collect(),
            }),
            // A stop is the loop's typed halt — the canonical plan
            // status card (`stop:` / `hint:` / `resume:`) rides the
            // stdout channel while the payload-free
            // `plan-execute-stopped` envelope keeps stderr (exit 2 /
            // 422), so a driver gets the structured next action
            // without a follow-up `emery plan status` call.
            ExecuteOutcome::Stopped {
                reason,
                detail,
                slice,
                ..
            } => {
                let source = Error::validation_failed(
                    "plan-execute-stopped",
                    "the execute loop drains the plan",
                    match (slice, detail) {
                        (Some(slice), Some(detail)) => format!("stop {reason} ({slice}): {detail}"),
                        (Some(slice), None) => format!("stop {reason} ({slice})"),
                        (None, Some(detail)) => format!("stop {reason}: {detail}"),
                        (None, None) => format!("stop {reason}"),
                    },
                );
                let plan = project::plan::Plan::load(&cx.layout().plan_path())?;
                let status = project::plan::plan_status_body(&plan, cx.layout())?;
                Err(project::handler::Error::stopped(status, source))
            }
        }
    }
}

/// Success envelope for the `plan execute` drained exit.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExecuteBody {
    /// Always `drained` — a stop surfaces on the error envelope.
    pub status: &'static str,
    /// Plan name from `plan.yaml.name`.
    pub plan: String,
    /// Completed phases in run order.
    pub phases: Vec<ExecutePhase>,
}

/// One completed phase in the drained run.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExecutePhase {
    /// Slice name.
    pub slice: String,
    /// Loop step (refine / build / merge).
    pub step: LoopStep,
}

impl Render for ExecuteBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        for phase in &self.phases {
            writeln!(w, "{} {}", phase.step, phase.slice)?;
        }
        writeln!(w, "{}", project::plan::drained_line(&self.plan))
    }
}
