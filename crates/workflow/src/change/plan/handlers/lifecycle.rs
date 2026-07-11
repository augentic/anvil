//! Plan lifecycle verbs: validate / next / status / transition /
//! archive.

use std::io::Write;

use error::{Error, Result};
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use schema::diagnostics::{Diagnostic, Severity, blocking, blocking_present};
use serde::{Deserialize, Serialize};

use super::{Ref, plan_ref, require_file};
use crate::adapter::Resolver;
use crate::change::{
    Lifecycle, NextActionKind, NextBody, NextReason, Plan, Status as EntryStatus, StatusBody,
    drained_line, plan_doctor, plan_finding, plan_next_body, plan_status_body,
};
use crate::config::with_state;
use crate::handler::{Anchor, Ctx, Render, ReportBody};
use crate::registry::Registry;

// ---------------------------------------------------------------------------
// plan validate
// ---------------------------------------------------------------------------

/// Wire input for `plan validate` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct ValidateInput {}

/// `specify plan validate` — structure + plan/change consistency,
/// including the health diagnostics (`cycle-in-depends-on`,
/// `orphan-source`, `stale-workspace-clone`).
#[derive(Clone, Copy, Debug)]
pub struct Validate;

impl<P: Anchor + Resolver> Operation<P> for Validate {
    type Error = crate::handler::Error;
    type Input = ValidateInput;
    type Output = ReportBody;

    async fn call(
        _input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let plan_path = require_file(&cx)?;
        let plan = Plan::load(&plan_path)?;
        let slices_dir = cx.layout().slices_dir();

        let (registry, registry_err) = match Registry::load(&cx.project_dir) {
            Ok(reg) => (reg, None),
            Err(err) => (None, Some(err)),
        };

        let mut results: Vec<Diagnostic> =
            plan_doctor(&plan, Some(&slices_dir), registry.as_ref(), Some(&cx.project_dir));

        if let Some(err) = registry_err {
            results.push(plan_finding(
                "registry-shape",
                Severity::Important,
                err.to_string(),
                None,
            ));
        }
        if let Some(reg) = &registry {
            let workspace_base = cx.project_dir.join("workspace");
            results.extend(crate::registry::cache_staleness(
                context.provider,
                reg,
                &workspace_base,
                &cx.layout().topology_lock_path(),
            ));
        }

        let has_errors = blocking_present(&results);
        let body = ReportBody::new(results, Some("Plan OK"), write_validate_row_text);
        if has_errors {
            Err(crate::handler::Error::Report {
                body,
                source: Error::validation_failed(
                    "plan-structural-errors",
                    "plan must be free of structural errors",
                    "run 'specify plan validate' for detail",
                ),
            })
        } else {
            Ok(body)
        }
    }
}

fn write_validate_row_text(w: &mut dyn Write, finding: &Diagnostic) -> std::io::Result<()> {
    let label = if blocking(finding) { "ERROR  " } else { "WARNING" };
    let code = finding.rule_id.as_deref().unwrap_or("<unknown>");
    let entry_col = finding.slice.as_ref().map_or_else(String::new, |e| format!("[{e}]"));
    writeln!(w, "{label} {:<32} {:<24} {}", code, entry_col, finding.impact)
}

// ---------------------------------------------------------------------------
// plan next
// ---------------------------------------------------------------------------

/// Wire input for `plan next` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct NextInput {}

/// `specify plan next`.
///
/// Return the active in-progress entry, or transition the next
/// eligible `Pending` entry to `InProgress` and return it. The only
/// writer of per-entry `in-progress` per workflow §CLI surface.
#[derive(Clone, Copy, Debug)]
pub struct Next;

impl<P: Anchor + Resolver> Operation<P> for Next {
    type Error = crate::handler::Error;
    type Input = NextInput;
    type Output = NextBody;

    async fn call(
        _input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        // The slice's target adapter is resolved on demand from the bound
        // project's topology, so the
        // topology inputs (`config` / `project_dir`) ride into the state
        // closure for `plan_next_body` to resolve the advanced entry's
        // `$TARGET` lazily. All projection logic lives in `workflow`;
        // the handler only returns the body and owns the journal bracket.
        let slices_dir = cx.layout().slices_dir();
        let config = cx.config.clone();
        let project_dir = cx.project_dir.clone();

        let (body, plan_name) = with_state::<Plan, _, _>(cx.layout(), "plan.yaml", move |plan| {
            let body = plan_next_body(context.provider, plan, &slices_dir, &config, &project_dir)?;
            Ok((body, plan.name.clone()))
        })?;
        // workflow §Observability: `plan.entry.advanced` fires only when an
        // entry actually moved `pending → in-progress` (`body.next`
        // populated). Returning the active entry or reporting
        // drained/stuck emits nothing, so a parked execute loop leaves no
        // advance event behind.
        if let Some(advanced) = &body.next {
            let event = crate::journal::Event::new(
                cx.now(),
                crate::journal::EventKind::PlanEntryAdvanced {
                    plan_name,
                    slice_name: advanced.clone().into(),
                },
            );
            crate::journal::append_batch(cx.layout(), std::slice::from_ref(&event))?;
        }
        Ok(body)
    }
}

impl Render for NextBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if let Some(active) = &self.active {
            writeln!(w, "Active change in progress: {active}")
        } else if let Some(name) = &self.next {
            writeln!(w, "{name}")
        } else if self.reason == Some(NextReason::Drained) {
            writeln!(w, "Plan drained — no per-entry pending or in-progress remains.")
        } else {
            writeln!(
                w,
                "No eligible changes \u{2014} remaining entries are waiting on unmet dependencies."
            )
        }
    }
}

// ---------------------------------------------------------------------------
// plan status
// ---------------------------------------------------------------------------

/// Wire input for `plan status` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct StatusInput {}

/// `specify plan status`.
///
/// Read-only projection of the plan's execution state into a
/// deterministic `next-action`. All projection logic lives in
/// `workflow` (`plan_status_body`); the handler loads the plan and
/// returns the body. No journal emit, no writes.
#[derive(Clone, Copy, Debug)]
pub struct Status;

impl<P: Anchor> Operation<P> for Status {
    type Error = crate::handler::Error;
    type Input = StatusInput;
    type Output = StatusBody;

    async fn call(
        _input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let plan_path = require_file(&cx)?;
        let plan = Plan::load(&plan_path)?;
        let body = plan_status_body(&plan, cx.layout())?;
        Ok(body)
    }
}

/// Text rendering for `plan status`: a plan/entries header, then the
/// next-action line. Stops render the stop-conditions block shape
/// (`stop: <reason>` + indented context + `hint:`); drained renders
/// the literal stop-conditions drained string.
impl Render for StatusBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "plan: {} ({})", self.plan, self.lifecycle)?;
        writeln!(
            w,
            "entries: {} done / {} in-progress / {} pending",
            self.counts.done, self.counts.in_progress, self.counts.pending
        )?;
        match (self.action, &self.stop) {
            (NextActionKind::Drained, _) => writeln!(w, "{}", drained_line(&self.plan))?,
            (NextActionKind::Stop, Some(stop)) => {
                writeln!(w, "stop: {}", stop.reason)?;
                if let Some(slice) = &self.slice {
                    writeln!(w, "  slice: {slice}")?;
                    writeln!(w, "  project: {}", self.project.as_deref().unwrap_or("-"))?;
                }
                if let Some(detail) = &stop.detail {
                    writeln!(w, "  detail: {detail}")?;
                }
                writeln!(w, "hint: {}", stop.hint)?;
            }
            _ => writeln!(w, "next-action: {}", self.next_action)?,
        }
        if self.action != NextActionKind::Drained
            && let Some(resume) = &self.resume
        {
            writeln!(w, "resume: {resume}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// plan transition
// ---------------------------------------------------------------------------

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
/// idempotent no-op (exit 0, no journal event) per auto-approve Gate-1 contract —
/// running the explicit transition after `specify plan create
/// --auto-approve` must not double-stamp the lifecycle nor double-
/// fire `plan.transition.approved`.
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
        let actor: crate::journal::Actor = actor.parse().map_err(|detail| Error::Argument {
            flag: "--actor",
            detail,
        })?;
        let plan_path = cx.layout().plan_path();
        let body = with_state::<Plan, _, _>(cx.layout(), "plan.yaml", move |plan| {
            if undo {
                dispatch_undo(plan, &plan_path, &name)
            } else {
                // The CLI grammar's `required_unless_present = "undo"`
                // guarantees a target on that transport; the error
                // surfaces the same usage diagnostic for the others.
                let target = target.ok_or_else(|| Error::Argument {
                    flag: "<target>",
                    detail: "transition target is required unless --undo is set".to_string(),
                })?;
                dispatch_transition(plan, &plan_path, &name, &target)
            }
        })?;
        // workflow §Observability: every status / lifecycle move emits
        // exactly one journal event when the on-disk state actually
        // changed. The same-state no-op path (already-`approved` plan)
        // flags `changed = false` so we skip the emit.
        match (body.kind, body.changed) {
            (TransitionKind::Plan, true) => {
                let event = crate::journal::Event::new(
                    cx.now(),
                    crate::journal::EventKind::PlanTransitionApproved {
                        plan_name: body.name.clone().into(),
                        actor,
                    },
                );
                crate::journal::append_batch(cx.layout(), std::slice::from_ref(&event))?;
            }
            (TransitionKind::Undo, true) => {
                let pair = body.undo.ok_or_else(|| Error::Diag {
                    code: "plan-transition-undo",
                    detail: "undo body must carry the status pair".to_string(),
                })?;
                let event = crate::journal::Event::new(
                    cx.now(),
                    crate::journal::EventKind::PlanTransitionUndone {
                        plan_name: body.plan.name.clone().into(),
                        slice_name: body.name.clone().into(),
                        from: pair.from,
                        to: pair.to,
                    },
                );
                crate::journal::append_batch(cx.layout(), std::slice::from_ref(&event))?;
            }
            _ => {}
        }
        Ok(body)
    }
}

fn dispatch_undo(
    plan: &mut Plan, plan_path: &std::path::Path, name: &str,
) -> Result<TransitionBody> {
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
    Ok(TransitionBody {
        plan: plan_ref(plan, plan_path),
        kind: TransitionKind::Undo,
        name: entry.name.to_string(),
        previous: from.to_string(),
        current: to.to_string(),
        changed: true,
        undo: Some(UndoPair { from, to }),
    })
}

fn dispatch_transition(
    plan: &mut Plan, plan_path: &std::path::Path, name: &str, target: &str,
) -> Result<TransitionBody> {
    if name == plan.name.as_str() {
        // Plan-level transition: only `approved` is legal.
        return match target {
            "approved" => {
                let previous = plan.lifecycle;
                if matches!(previous, Lifecycle::Approved) {
                    // auto-approve Gate-1 contract: `--auto-approve` already stamped
                    // this plan; the explicit transition is the
                    // operator's belt-and-braces follow-up. No
                    // disk or journal write — `body.changed` is
                    // `false` so the caller suppresses the emit.
                    return Ok(TransitionBody {
                        plan: plan_ref(plan, plan_path),
                        kind: TransitionKind::Plan,
                        name: plan.name.to_string(),
                        previous: previous.to_string(),
                        current: plan.lifecycle.to_string(),
                        changed: false,
                        undo: None,
                    });
                }
                plan.transition_lifecycle(Lifecycle::Approved)?;
                Ok(TransitionBody {
                    plan: plan_ref(plan, plan_path),
                    kind: TransitionKind::Plan,
                    name: plan.name.to_string(),
                    previous: previous.to_string(),
                    current: plan.lifecycle.to_string(),
                    changed: true,
                    undo: None,
                })
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
            Ok(TransitionBody {
                plan: plan_ref(plan, plan_path),
                kind: TransitionKind::Entry,
                name: entry.name.to_string(),
                previous: previous.to_string(),
                current: entry.status.to_string(),
                changed: true,
                undo: None,
            })
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
    /// `false` when the transition was an idempotent no-op (workflow
    /// rules-root resolution — explicit `approved` after `--auto-approve`); `true`
    /// when the lifecycle / status actually moved. The handler reads
    /// this to decide whether to fire the `plan.transition.approved`
    /// journal event.
    #[serde(skip)]
    pub changed: bool,
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
            TransitionKind::Plan if !self.changed => {
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

// ---------------------------------------------------------------------------
// plan archive
// ---------------------------------------------------------------------------

/// Wire input for `plan archive`.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArchiveInput {
    /// Archive even when the plan has pending or in-progress entries.
    #[serde(default)]
    pub force: bool,
}

/// `specify plan archive` — move the current plan to
/// `.specify/archive/plans/<name>-<YYYYMMDD>.yaml`.
#[derive(Clone, Copy, Debug)]
pub struct Archive;

impl<P: Anchor> Operation<P> for Archive {
    type Error = crate::handler::Error;
    type Input = ArchiveInput;
    type Output = ArchiveBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let layout = cx.layout();
        let plan_path = layout.plan_path();
        if !plan_path.exists() {
            return Err(Error::ArtifactNotFound {
                kind: "plan.yaml",
                path: plan_path,
            }
            .into());
        }
        let archive_dir = layout.archive_dir().join("plans");
        let brief_path = layout.change_brief_path();
        let plan_name = Plan::load(&plan_path)?.name.into_string();

        let (archived, archived_plans_dir) =
            Plan::archive(&plan_path, &brief_path, &archive_dir, input.force, cx.now())?;
        Ok(ArchiveBody {
            archived: archived.display().to_string(),
            archived_plans_dir: archived_plans_dir.as_deref().map(|p| p.display().to_string()),
            plan: ArchivedPlan { name: plan_name },
        })
    }
}

/// Success envelope for `plan archive`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArchiveBody {
    /// Display path of the archived plan file.
    pub archived: String,
    /// Display path of the moved working directory, when one moved.
    pub archived_plans_dir: Option<String>,
    /// The archived plan's identity.
    pub plan: ArchivedPlan,
}

/// The archived plan's identity.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArchivedPlan {
    /// Plan name.
    pub name: String,
}

impl Render for ArchiveBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        match &self.archived_plans_dir {
            Some(dir) => {
                writeln!(w, "Archived plan to {}. Working directory moved to {dir}.", self.archived)
            }
            None => writeln!(w, "Archived plan to {}.", self.archived),
        }
    }
}
