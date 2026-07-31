//! `plan execute` — the Gate 1 stamp plus the drained refine → build →
//! merge loop over the plan.

use std::io::Write;

use error::Error;
use omnia_guest::Model;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::Resolver;
use project::handler::{Anchor, Ctx, Render};
use project::journal;
use project::plan::LoopStep;
use project::seam::{Source, Target, WorkingTree};
use serde::{Deserialize, Serialize};

use crate::orchestrate::{self, ExecuteOutcome};

/// Wire input for `plan execute`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExecuteInput {
    /// Who is driving this invocation — `operator` (default) or
    /// `agent`. Recorded on the `plan.transition.approved` journal
    /// event when this run stamps Gate 1.
    #[serde(default = "default_actor")]
    pub actor: String,
}

fn default_actor() -> String {
    "operator".to_string()
}

impl Default for ExecuteInput {
    fn default() -> Self {
        Self {
            actor: default_actor(),
        }
    }
}

/// `emery plan execute` → the internal execute orchestration — the
/// Gate 1 stamp (`pending → approved`, invoking execute is the
/// approval act) plus the drained refine → build → merge loop.
#[derive(Clone, Copy, Debug)]
pub struct Execute;

impl<P: Anchor + Model + Resolver + Source + Target> Operation<P> for Execute {
    type Error = project::handler::Error;
    type Input = ExecuteInput;
    type Output = ExecuteBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let actor: journal::Actor = input.actor.parse().map_err(|detail| Error::Argument {
            flag: "--actor",
            detail,
        })?;
        let tree = WorkingTree::live();
        let caps = orchestrate::Capabilities::provider(context.provider);
        let outcome = orchestrate::execute(caps, &cx.paths, cx.now(), &tree, actor).await?;
        match outcome {
            ExecuteOutcome::Drained {
                plan,
                gate1_stamped,
                phases,
            } => Ok(ExecuteBody {
                status: "drained",
                plan,
                gate1_stamped,
                actor,
                phases: phases
                    .into_iter()
                    .map(|run| ExecutePhase {
                        slice: run.slice,
                        step: run.step,
                    })
                    .collect(),
            }),
            // A stop is the loop's typed halt — surface it on the
            // error envelope (exit 2 / 422) so a driver can tell a
            // parked loop from a drained one without parsing prose.
            ExecuteOutcome::Stopped {
                reason,
                detail,
                hint,
                slice,
                ..
            } => Err(Error::validation_failed(
                "plan-execute-stopped",
                "the execute loop drains the plan",
                match (slice, detail) {
                    (Some(slice), Some(detail)) => {
                        format!("stop {reason} ({slice}): {detail} — {hint}")
                    }
                    (Some(slice), None) => format!("stop {reason} ({slice}) — {hint}"),
                    (None, Some(detail)) => format!("stop {reason}: {detail} — {hint}"),
                    (None, None) => format!("stop {reason} — {hint}"),
                },
            )
            .into()),
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
    /// Whether this invocation performed the Gate 1 stamp
    /// (`pending → approved`); `false` on re-entry.
    pub gate1_stamped: bool,
    /// Actor recorded on the Gate 1 stamp — display only (the text
    /// `approved:` line); the JSON envelope carries `gate1-stamped`.
    #[serde(skip)]
    pub actor: journal::Actor,
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
        if self.gate1_stamped {
            writeln!(w, "approved: {} (actor: {})", self.plan, self.actor)?;
        }
        for phase in &self.phases {
            writeln!(w, "{} {}", phase.step, phase.slice)?;
        }
        writeln!(w, "{}", project::plan::drained_line(&self.plan))
    }
}
