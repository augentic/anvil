//! `plan undo` — the one-rung reverse walk on per-entry status.
//! Forward moves have dedicated writers: `plan add` / `plan amend`
//! write `pending`, `plan next` writes `in-progress`, `slice merge`
//! writes `done`, and plan-level `approved` is stamped by the first
//! `emery plan execute`.

use std::io::Write;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::config::{Mutation, with_state};
use project::handler::{Anchor, Ctx, Render};
use project::journal::{self, Event, EventKind};
use project::plan::{Plan, Status as EntryStatus};
use serde::{Deserialize, Serialize};

use super::{Ref, plan_ref};

/// Wire input for `plan undo`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UndoInput {
    /// Kebab-case plan-entry name.
    pub name: String,
}

/// `emery plan undo <entry>`.
///
/// The one-rung reverse walk on per-entry status (`done →
/// in-progress`, `in-progress → pending`). Forward moves are owned by
/// their dedicated writers — `slice merge` stamps `done` (re-run
/// `emery slice merge` to heal a missing stamp), and plan-level
/// `approved` is stamped by the first `emery plan execute`.
#[derive(Clone, Copy, Debug)]
pub struct Undo;

impl<P: Anchor> Operation<P> for Undo {
    type Error = project::handler::Error;
    type Input = UndoInput;
    type Output = UndoBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let UndoInput { name } = input;
        let plan_path = cx.layout().plan_path();
        // workflow §Observability: every status move emits exactly one
        // `plan.transition.undone` journal event.
        let (body, event) = with_state::<Plan, _, _>(cx.layout(), "plan.yaml", move |plan| {
            let (from, to) = plan.transition_undo(&name)?;
            let entry = plan
                .entries
                .iter()
                .find(|e| e.name == name)
                .ok_or_else(|| plan.entry_not_found(&name))?;
            let body = UndoBody {
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

/// Success envelope for `plan undo`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct UndoBody {
    /// The governing plan file.
    pub plan: Ref,
    /// Entry name the undo acted on.
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

impl Render for UndoBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "undid `{}`: {} \u{2192} {}", self.name, self.previous, self.current)?;
        writeln!(w, "  plan: {}", self.plan.path.display())
    }
}
