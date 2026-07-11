//! `plan transition` — the Gate 1 stamp, the per-entry close, and the
//! one-rung undo walk.

use std::io::Write;

use error::{Error, Result};
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use super::{Ref, plan_ref};
use crate::change::{Lifecycle, Plan, Status as EntryStatus};
use crate::config::{Mutation, with_state};
use crate::handler::{Anchor, Ctx, Render};
use crate::journal::{self, Event, EventKind};

/// Wire input for `plan transition`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TransitionInput {
    /// Plan name (for plan-level `approved`) or kebab-case entry name
    /// (for per-entry `done` / undo).
    pub name: String,
    /// Transition target — `approved` (plan-level) or `done`
    /// (per-entry). Omit when `undo` is set.
    #[serde(default)]
    pub target: Option<String>,
    /// Walk one rung backwards on per-entry status.
    #[serde(default)]
    pub undo: bool,
    /// Who is driving this invocation — `operator` (default) or
    /// `agent`.
    #[serde(default = "default_actor")]
    pub actor: String,
}

fn default_actor() -> String {
    "operator".to_string()
}

/// `specify plan transition <name> <target>`.
///
/// Dispatches to either the plan-level Gate 1 stamp (`<plan-name>
/// approved`) or the per-entry close (`<entry-name> done`). `undo`
/// swaps the forward verb for the one-rung reverse walk on per-entry
/// status (`done → in-progress`, `in-progress → pending`); plan-level
/// lifecycle has no undo path in v1.
///
/// `<plan-name> approved` against an already-approved plan is an
/// idempotent no-op (exit 0, no disk write, no journal event) per the
/// auto-approve Gate-1 contract — running the explicit transition
/// after `specify plan create --auto-approve` must not double-stamp
/// the lifecycle nor double-fire `plan.transition.approved`.
///
/// `actor` (default `operator`) is recorded on the
/// `plan.transition.approved` event only — self-reported grading
/// evidence for eval probes, ignored on per-entry and undo paths.
#[derive(Clone, Copy, Debug)]
pub struct Transition;

impl<P: Anchor> Operation<P> for Transition {
    type Error = crate::handler::Error;
    type Input = TransitionInput;
    type Output = TransitionBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let TransitionInput {
            name,
            target,
            undo,
            actor,
        } = input;
        let actor: journal::Actor = actor.parse().map_err(|detail| Error::Argument {
            flag: "--actor",
            detail,
        })?;
        let plan_path = cx.layout().plan_path();
        // workflow §Observability: every status / lifecycle move emits
        // exactly one journal event when the on-disk state actually
        // changed. The same-state no-op path (already-`approved` plan)
        // returns `Mutation::unchanged` with no event, so neither the
        // disk write nor the emit fires.
        let (body, event) = with_state::<Plan, _, _>(cx.layout(), "plan.yaml", move |plan| {
            if undo {
                dispatch_undo(plan, &plan_path, &name).map(Mutation::changed)
            } else {
                // The CLI grammar's `required_unless_present = "undo"`
                // guarantees a target on that transport; the error
                // surfaces the same usage diagnostic for the others.
                let target = target.ok_or_else(|| Error::Argument {
                    flag: "<target>",
                    detail: "transition target is required unless --undo is set".to_string(),
                })?;
                dispatch_transition(plan, &plan_path, &name, &target, actor)
            }
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
    if name == plan.name.as_str() {
        return Err(Error::Argument {
            flag: "--undo",
            detail: "plan-level lifecycle has no undo path in v1; `--undo` operates on \
                     per-entry status only. To un-stamp `approved`, edit `plan.yaml` directly \
                     (out of scope for the CLI) or drop and re-create the plan."
                .to_string(),
        });
    }
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

fn dispatch_transition(
    plan: &mut Plan, plan_path: &std::path::Path, name: &str, target: &str,
    actor: journal::Actor,
) -> Result<Mutation<(TransitionBody, Option<EventKind>)>> {
    if name == plan.name.as_str() {
        // Plan-level transition: only `approved` is legal.
        return match target {
            "approved" => {
                let previous = plan.lifecycle;
                if matches!(previous, Lifecycle::Approved) {
                    // auto-approve Gate-1 contract: `--auto-approve`
                    // already stamped this plan; the explicit
                    // transition is the operator's belt-and-braces
                    // follow-up. No disk or journal write.
                    let body = TransitionBody {
                        plan: plan_ref(plan, plan_path),
                        kind: TransitionKind::Plan,
                        name: plan.name.to_string(),
                        previous: previous.to_string(),
                        current: plan.lifecycle.to_string(),
                        undo: None,
                    };
                    return Ok(Mutation::unchanged((body, None)));
                }
                plan.transition_lifecycle(Lifecycle::Approved)?;
                let body = TransitionBody {
                    plan: plan_ref(plan, plan_path),
                    kind: TransitionKind::Plan,
                    name: plan.name.to_string(),
                    previous: previous.to_string(),
                    current: plan.lifecycle.to_string(),
                    undo: None,
                };
                let event = EventKind::PlanTransitionApproved {
                    plan_name: plan.name.clone(),
                    actor,
                };
                Ok(Mutation::changed((body, Some(event))))
            }
            other => Err(plan_target_invalid(other)),
        };
    }

    // Per-entry transition: only `done` is legal. `pending` is owned by
    // `plan add`/`amend`; `in-progress` is owned by `plan next`; and
    // `blocked`/`failed`/`skipped` are not v1 states.
    match target {
        "done" => {
            let idx =
                plan.entries.iter().position(|e| e.name == name).ok_or_else(|| Error::Diag {
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
            Ok(Mutation::changed((body, None)))
        }
        other => Err(entry_target_invalid(other)),
    }
}

fn plan_target_invalid(target: &str) -> Error {
    Error::Argument {
        flag: "<target>",
        detail: format!(
            "plan-level transition target must be `approved`; got `{target}`. \
             Run `specify plan transition <plan-name> approved` to stamp Gate 1."
        ),
    }
}

fn entry_target_invalid(target: &str) -> Error {
    Error::Argument {
        flag: "<target>",
        detail: match target {
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
                 `done` is stamped by `/spec:merge` (or by hand once the slice is merged)."
            ),
        },
    }
}

/// Which transition shape ran.
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TransitionKind {
    /// Plan-level Gate 1 stamp.
    Plan,
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
    /// Plan or entry name the transition acted on.
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
            // The idempotent no-op: the plan was already at the target
            // lifecycle, so nothing moved.
            TransitionKind::Plan if self.previous == self.current => {
                writeln!(
                    w,
                    "Plan '{}' is already at lifecycle: {} (no-op).",
                    self.name, self.current
                )
            }
            TransitionKind::Plan => writeln!(
                w,
                "Stamped plan '{}': lifecycle {} \u{2192} {}.",
                self.name, self.previous, self.current
            ),
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
