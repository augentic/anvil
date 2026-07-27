//! `plan transition` — the per-entry close and the one-rung undo walk.
//! Plan-level Gate 1 lives on `plan approve`.

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
    /// Transition target — `done` is the only forward target. Omit
    /// when `undo` is set.
    #[serde(default)]
    pub target: Option<String>,
    /// Walk one rung backwards on per-entry status.
    #[serde(default)]
    pub undo: bool,
}

/// `emery plan transition <entry> <target>`.
///
/// The per-entry close (`<entry> done`; the `/emery:merge` skill is the
/// canonical caller) and, with `--undo`, the one-rung reverse walk on
/// per-entry status (`done → in-progress`, `in-progress → pending`).
/// Plan-level Gate 1 is `emery plan approve`.
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
        let TransitionInput { name, target, undo } = input;
        let plan_path = cx.layout().plan_path();
        // workflow §Observability: every status move emits exactly one
        // journal event (`plan.transition.undone` on the undo path;
        // the forward `done` close is silent — `slice.merge.*` carries
        // the observable record).
        let (body, event) = with_state::<Plan, _, _>(cx.layout(), "plan.yaml", move |plan| {
            let pair = if undo {
                dispatch_undo(plan, &plan_path, &name)
            } else {
                // The CLI grammar's `required_unless_present = "undo"`
                // guarantees a target on that transport; the error
                // surfaces the same usage diagnostic for the others.
                let target = target.ok_or_else(|| Error::Argument {
                    flag: "<target>",
                    detail: "transition target is required unless --undo is set".to_string(),
                })?;
                dispatch_done(plan, &plan_path, &name, &target)
            }?;
            Ok(Mutation::changed(pair))
        })?;
        if let Some(kind) = event {
            journal::append_one(cx.layout(), &Event::new(cx.now(), kind))?;
        }
        Ok(body)
    }
}

fn dispatch_undo(
    plan: &mut Plan, plan_path: &std::path::Path, name: &str,
) -> Result<(TransitionBody, Option<EventKind>)> {
    let (from, to) = plan.transition_undo(name)?;
    let entry = plan.entries.iter().find(|e| e.name == name).ok_or_else(|| Error::Diag {
        code: "plan-entry-not-found",
        detail: format!("no slice named '{name}' in plan"),
    })?;
    let body = TransitionBody {
        plan: plan_ref(plan, plan_path),
        kind: TransitionKind::Undo,
        name: entry.name.to_string(),
        previous: from.to_string(),
        current: to.to_string(),
        undo: Some(UndoPair { from, to }),
    };
    let event = EventKind::PlanTransitionUndone {
        plan_name: plan.name.clone(),
        slice_name: entry.name.clone(),
        from,
        to,
    };
    Ok((body, Some(event)))
}

fn dispatch_done(
    plan: &mut Plan, plan_path: &std::path::Path, name: &str, target: &str,
) -> Result<(TransitionBody, Option<EventKind>)> {
    // Per-entry transition: only `done` is legal. `pending` is owned by
    // `plan add`/`amend`; `in-progress` is owned by `plan next`; and
    // `blocked`/`failed`/`skipped` are not v1 states.
    if target != "done" {
        return Err(target_invalid(target));
    }
    let idx = plan.entries.iter().position(|e| e.name == name).ok_or_else(|| Error::Diag {
        code: "plan-entry-not-found",
        detail: format!("no slice named '{name}' in plan"),
    })?;
    let previous = plan.entries[idx].status;
    plan.transition(name, EntryStatus::Done)?;
    let entry = &plan.entries[idx];
    let body = TransitionBody {
        plan: plan_ref(plan, plan_path),
        kind: TransitionKind::Entry,
        name: entry.name.to_string(),
        previous: previous.to_string(),
        current: entry.status.to_string(),
        undo: None,
    };
    Ok((body, None))
}

fn target_invalid(target: &str) -> Error {
    Error::Argument {
        flag: "<target>",
        detail: match target {
            "approved" => "plan-level `approved` is stamped by `emery plan approve`, not \
                           `plan transition`."
                .to_string(),
            "pending" => {
                "per-entry `pending` is written by `plan add` / `plan amend`, not `plan transition`. \
                 To clear an entry, drop and re-add it.".to_string()
            }
            "in-progress" => {
                "per-entry `in-progress` is written only by `plan next`; \
                 `plan transition` cannot move an entry into the active slot."
                    .to_string()
            }
            "blocked" | "failed" | "skipped" => format!(
                "per-entry `{target}` is not a v1 state — the 2.0 collapse removed the per-entry enum to \
                 `pending | in-progress | done`. Build failures and merge conflicts leave the \
                 active entry `in-progress`."
            ),
            other => format!(
                "per-entry transition target must be `done`; got `{other}`. \
                 `done` is stamped by `/emery:merge` (or by hand once the slice is merged)."
            ),
        },
    }
}

/// Which transition shape ran.
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TransitionKind {
    /// Per-entry close.
    Entry,
    /// One-rung reverse walk.
    Undo,
}

/// Success envelope for `plan transition`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TransitionBody {
    /// The governing plan file.
    pub plan: Ref,
    /// Which transition shape ran.
    pub kind: TransitionKind,
    /// Entry name the transition acted on.
    pub name: String,
    /// Status before the transition.
    pub previous: String,
    /// Status after the transition.
    pub current: String,
    /// Status pair the undo walk visited, if any. `None` on forward
    /// transitions and on undo failures that never reached the
    /// mutation step. Surfaced on the JSON envelope under
    /// `undo: { from, to }` so wire consumers can branch on the
    /// reverse step without re-parsing `previous` / `current`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo: Option<UndoPair>,
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
        match self.kind {
            TransitionKind::Entry => writeln!(
                w,
                "Transitioned '{}': {} \u{2192} {}.",
                self.name, self.previous, self.current
            ),
            TransitionKind::Undo => {
                writeln!(w, "Undid '{}': {} \u{2192} {}.", self.name, self.previous, self.current)
            }
        }
    }
}
