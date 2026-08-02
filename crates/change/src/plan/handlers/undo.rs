//! `plan undo` — the reverse walk on per-entry status. Forward moves
//! have dedicated writers: `plan add` / `plan amend` write `pending`,
//! `plan advance` writes `in-progress`, and `slice merge` writes
//! `done`.

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
    /// Optional target status: walk rung by rung until the entry
    /// reaches it. Absent means one rung.
    #[serde(default)]
    pub to: Option<EntryStatus>,
}

/// `emery plan undo <entry> [--to <status>]`.
///
/// The reverse walk on per-entry status (`done → in-progress`,
/// `in-progress → pending`). Default is one rung; `--to` walks rung
/// by rung until the entry reaches the target, emitting one
/// `plan.transition.undone` journal event per rung so the journal
/// cadence is identical either way. Forward moves are owned by their
/// dedicated writers — `slice merge` stamps `done` (re-run `emery
/// slice merge` to heal a missing stamp).
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
        let UndoInput { name, to } = input;
        let plan_path = cx.layout().plan_path();
        // workflow §Observability: every rung emits exactly one
        // `plan.transition.undone` journal event, whether walked one
        // call at a time or through `--to`.
        let (body, events) = with_state::<Plan, _, _>(cx.layout(), "plan.yaml", move |plan| {
            let entry = plan
                .entries
                .iter()
                .find(|e| e.name == name)
                .ok_or_else(|| plan.entry_not_found(&name))?;
            if let Some(target) = to
                && entry.status == target
            {
                return Err(error::Error::Diag {
                    code: "plan-transition-undo",
                    detail: format!("entry `{name}` is already `{target}`; nothing to undo"),
                });
            }
            let mut steps: Vec<UndoPair> = Vec::new();
            let mut events: Vec<EventKind> = Vec::new();
            loop {
                let (from, to_step) = plan.transition_undo(&name)?;
                steps.push(UndoPair { from, to: to_step });
                events.push(EventKind::PlanTransitionUndone {
                    plan_name: plan.name.clone(),
                    slice_name: name.clone().into(),
                    from,
                    to: to_step,
                });
                match to {
                    None => break,
                    Some(target) if to_step == target => break,
                    Some(_) => {}
                }
            }
            let first = steps.first().copied().expect("the walk visited at least one rung");
            let last = steps.last().copied().expect("the walk visited at least one rung");
            let body = UndoBody {
                plan: plan_ref(plan, &plan_path),
                name: name.clone(),
                previous: first.from.to_string(),
                current: last.to.to_string(),
                undo: steps,
            };
            Ok(Mutation::changed((body, events)))
        })?;
        for event in events {
            journal::append_one(cx.layout(), &Event::new(cx.now(), event))?;
        }
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
    /// Status before the walk's first rung.
    pub previous: String,
    /// Status after the walk's last rung.
    pub current: String,
    /// Every `(from, to)` rung the walk visited in order, surfaced on
    /// the JSON envelope under `undo: [{ from, to }, …]` so wire
    /// consumers can replay the reverse steps without re-parsing
    /// `previous` / `current`.
    pub undo: Vec<UndoPair>,
}

/// One `(from, to)` rung an undo walk visited.
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
        write!(w, "undid `{}`: {}", self.name, self.previous)?;
        for step in &self.undo {
            write!(w, " \u{2192} {}", step.to)?;
        }
        writeln!(w)?;
        writeln!(w, "  plan: {}", self.plan.path.display())
    }
}
