//! `plan undo` — reverse one projected ladder rung via retract facts.
//!
//! Forward progress is expressed as facts (`plan advance` claims;
//! refine/build/merge append phase facts; archive facts project
//! `done`). Undo retracts those facts — it does not rewrite stored
//! `Entry.status`.

use std::io::Write;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::handler::{Anchor, Ctx, Render};
use project::plan::{Status as EntryStatus, UndoStep, undo_entry};
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
/// The reverse walk on the projected ladder (`done → in-progress`,
/// `in-progress → pending`). Default is one rung; `--to` walks rung
/// by rung until the entry reaches the target. Each rung appends
/// `fact.retracted` (plus a projection-label `plan.transition.undone`)
/// and leaves `plan.yaml` untouched.
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
        let plan = project::plan::Plan::load(&plan_path)?;
        let steps = undo_entry(cx.layout(), cx.now(), &name, to)?;
        let first = steps.first().copied().expect("the walk visited at least one rung");
        let last = steps.last().copied().expect("the walk visited at least one rung");
        Ok(UndoBody {
            plan: plan_ref(&plan, &plan_path),
            name,
            previous: first.from.to_string(),
            current: last.to.to_string(),
            undo: steps.into_iter().map(UndoPair::from).collect(),
        })
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

impl From<UndoStep> for UndoPair {
    fn from(step: UndoStep) -> Self {
        Self {
            from: step.from,
            to: step.to,
        }
    }
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
