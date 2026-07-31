//! `plan transition` — the one-rung undo walk on per-entry status.
//! Forward moves have dedicated writers: `plan add` / `plan amend`
//! write `pending`, `plan next` writes `in-progress`, `slice merge`
//! writes `done`, and plan-level `approved` is stamped by the first
//! `emery plan execute`.

use std::io::Write;

use error::{Error, Result};
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::config::{Mutation, with_state};
use project::handler::{Anchor, Ctx, Render};
use project::journal::{self, Event, EventKind};
use project::plan::{Plan, Status as EntryStatus};
use serde::{Deserialize, Serialize};

use super::{Ref, plan_ref};

/// Wire input for `plan transition`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TransitionInput {
    /// Kebab-case plan-entry name.
    pub name: String,
    /// Walk one rung backwards on per-entry status — the only
    /// transition mode.
    #[serde(default)]
    pub undo: bool,
}

/// `emery plan transition <entry> --undo`.
///
/// The one-rung reverse walk on per-entry status (`done →
/// in-progress`, `in-progress → pending`). Forward moves are owned by
/// their dedicated writers — `slice merge` stamps `done` (re-run
/// `emery slice merge run` to heal a missing stamp), and plan-level
/// `approved` is stamped by the first `emery plan execute`.
#[derive(Clone, Copy, Debug)]
pub struct Transition;

impl<P: Anchor> Operation<P> for Transition {
    type Error = project::handler::Error;
    type Input = TransitionInput;
    type Output = TransitionBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let TransitionInput { name, undo } = input;
        if !undo {
            // The CLI grammar requires `--undo`; this guards the other
            // transports with the same contract.
            return Err(Error::Argument {
                flag: "--undo",
                detail: "plan transition only walks per-entry status backwards (--undo); \
                         forward `done` is stamped by `emery slice merge run` — re-run it to \
                         heal a missing stamp"
                    .to_string(),
            }
            .into());
        }
        let plan_path = cx.layout().plan_path();
        // workflow §Observability: every status move emits exactly one
        // `plan.transition.undone` journal event.
        let (body, event) = with_state::<Plan, _, _>(cx.layout(), "plan.yaml", move |plan| {
            let (from, to) = plan.transition_undo(&name)?;
            let entry =
                plan.entries.iter().find(|e| e.name == name).ok_or_else(|| Error::Diag {
                    code: "plan-entry-not-found",
                    detail: format!("no slice named '{name}' in plan"),
                })?;
            let body = TransitionBody {
                plan: plan_ref(plan, &plan_path),
                name: entry.name.to_string(),
                previous: from.to_string(),
                current: to.to_string(),
                undo: UndoPair { from, to },
            };
            let event = EventKind::PlanTransitionUndone {
                plan_name: plan.name.clone(),
                slice_name: entry.name.clone(),
                from,
                to,
            };
            Ok(Mutation::changed((body, event)))
        })?;
        journal::append_one(cx.layout(), &Event::new(cx.now(), event))?;
        Ok(body)
    }
}

/// Success envelope for `plan transition`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TransitionBody {
    /// The governing plan file.
    pub plan: Ref,
    /// Entry name the transition acted on.
    pub name: String,
    /// Status before the reverse step.
    pub previous: String,
    /// Status after the reverse step.
    pub current: String,
    /// The `(from, to)` pair the undo walk visited, surfaced on the
    /// JSON envelope under `undo: { from, to }` so wire consumers can
    /// branch on the reverse step without re-parsing `previous` /
    /// `current`.
    pub undo: UndoPair,
}

/// The `(from, to)` pair an undo walk visited.
#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub struct UndoPair {
    /// Status before the reverse step.
    pub from: EntryStatus,
    /// Status after the reverse step.
    pub to: EntryStatus,
}

impl Render for TransitionBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "Undid '{}': {} \u{2192} {}.", self.name, self.previous, self.current)
    }
}
