use std::io::Write;

use serde::Serialize;
use specify_diagnostics::{Diagnostic, Severity, blocking, blocking_present};
use specify_error::{Error, Result};
use specify_workflow::change::{
    Lifecycle, NextActionKind, NextBody, NextReason, Plan, Status, StatusBody, drained_line,
    plan_doctor, plan_finding, plan_next_body, plan_status_body,
};
use specify_workflow::config::with_state;
use specify_workflow::registry::Registry;

use super::{Ref, plan_ref, require_file};
use crate::runtime::context::Ctx;

pub(super) fn validate(ctx: &Ctx) -> Result<()> {
    let plan_path = require_file(ctx)?;
    let plan = Plan::load(&plan_path)?;
    let slices_dir = ctx.layout().slices_dir();

    let (registry, registry_err) = match Registry::load(&ctx.project_dir) {
        Ok(reg) => (reg, None),
        Err(err) => (None, Some(err)),
    };

    let mut results: Vec<Diagnostic> =
        plan_doctor(&plan, Some(&slices_dir), registry.as_ref(), Some(&ctx.project_dir));

    if let Some(err) = registry_err {
        results.push(plan_finding("registry-shape", Severity::Important, err.to_string(), None));
    }
    if let Some(reg) = &registry {
        let workspace_base = ctx.project_dir.join("workspace");
        results.extend(specify_workflow::registry::cache_staleness(
            reg,
            &workspace_base,
            &ctx.layout().topology_lock_path(),
        ));
    }

    let has_errors = blocking_present(&results);
    render_validate_report(ctx, results)?;
    if has_errors {
        Err(Error::validation_failed(
            "plan-structural-errors",
            "plan must be free of structural errors",
            "run 'specify plan validate' for detail",
        ))
    } else {
        Ok(())
    }
}

/// `specify plan next` — return the active in-progress entry, or
/// transition the next eligible `Pending` entry to `InProgress` and
/// return it. The only writer of per-entry `in-progress` per
/// workflow §CLI surface.
pub(super) fn next(ctx: &Ctx) -> Result<()> {
    // Plan-lock gate: refuse an unlocked driver before touching plan state.
    // A missing plan.yaml falls through to the artifact-not-found error
    // below — the lock refusal only makes sense once a plan exists.
    if ctx.layout().plan_path().exists() {
        specify_workflow::plan_lock::require_held(ctx.layout())?;
    }
    // The slice's target adapter is resolved on demand from the bound
    // project's topology, so the
    // topology inputs (`config` / `project_dir`) ride into the state
    // closure for `plan_next_body` to resolve the advanced entry's
    // `$TARGET` lazily. All projection logic lives in `specify-workflow`;
    // the handler only renders the returned body and owns the
    // journal/emit bracket.
    let slices_dir = ctx.layout().slices_dir();
    let config = ctx.config.clone();
    let project_dir = ctx.project_dir.clone();

    let (body, plan_name) = with_state::<Plan, _, _>(ctx.layout(), "plan.yaml", move |plan| {
        let body = plan_next_body(plan, &slices_dir, &config, &project_dir)?;
        Ok((body, plan.name.clone()))
    })?;
    // workflow §Observability: `plan.entry.advanced` fires only when an
    // entry actually moved `pending → in-progress` (`body.next`
    // populated). Returning the active entry or reporting
    // drained/stuck emits nothing, so a parked execute loop leaves no
    // advance event behind.
    if let Some(advanced) = &body.next {
        let event = specify_workflow::journal::Event::new(
            ctx.now(),
            specify_workflow::journal::EventKind::PlanEntryAdvanced {
                plan_name,
                slice_name: advanced.clone().into(),
            },
        );
        specify_workflow::journal::append_batch(ctx.layout(), std::slice::from_ref(&event))?;
    }
    ctx.write(&body, write_next_text)?;
    Ok(())
}

/// `specify plan status` — read-only projection of the plan's
/// execution state into a deterministic `next-action`. All projection
/// logic lives in `specify-workflow` (`plan_status_body`); the handler
/// loads the plan and renders the body. No journal emit, no writes.
pub(super) fn status(ctx: &Ctx) -> Result<()> {
    let plan_path = require_file(ctx)?;
    let plan = Plan::load(&plan_path)?;
    let body = plan_status_body(&plan, ctx.layout())?;
    ctx.write(&body, write_status_text)?;
    Ok(())
}

/// `specify plan transition <name> <target>` — dispatches to either
/// the plan-level Gate 1 stamp (`<plan-name> approved`) or the
/// per-entry close (`<entry-name> done`). `--undo` swaps the
/// forward verb for the one-rung reverse walk on per-entry status
/// (`done → in-progress`, `in-progress → pending`); plan-level
/// lifecycle has no undo path in v1.
///
/// `<plan-name> approved` against an already-approved plan is an
/// idempotent no-op (exit 0, no journal event) per auto-approve Gate-1 contract —
/// running the explicit transition after `specify plan create
/// --auto-approve` must not double-stamp the lifecycle nor double-
/// fire `plan.transition.approved`.
///
/// `--actor <operator|agent>` (default `operator`) is recorded on the
/// `plan.transition.approved` event only — self-reported grading
/// evidence for eval probes, ignored on per-entry and `--undo` paths.
pub(super) fn transition(
    ctx: &Ctx, name: String, target: Option<String>, undo: bool, actor: &str,
) -> Result<()> {
    let actor: specify_workflow::journal::Actor =
        actor.parse().map_err(|detail| Error::Argument {
            flag: "--actor",
            detail,
        })?;
    let plan_path = ctx.layout().plan_path();
    let layout = ctx.layout();
    let body = with_state::<Plan, _, _>(ctx.layout(), "plan.yaml", move |plan| {
        if undo {
            dispatch_undo(plan, layout, &plan_path, &name)
        } else {
            // Clap's `required_unless_present = "undo"` guarantees a
            // target here; the unwrap_or surfaces the same usage
            // diagnostic clap would have if it slipped through.
            let target = target.ok_or_else(|| Error::Argument {
                flag: "<target>",
                detail: "transition target is required unless --undo is set".to_string(),
            })?;
            dispatch_transition(plan, layout, &plan_path, &name, &target)
        }
    })?;
    // workflow §Observability: every status / lifecycle move emits
    // exactly one journal event when the on-disk state actually
    // changed. The same-state no-op path (already-`approved` plan)
    // flags `changed = false` so we skip the emit.
    match (body.kind, body.changed) {
        (TransitionKind::Plan, true) => {
            let event = specify_workflow::journal::Event::new(
                ctx.now(),
                specify_workflow::journal::EventKind::PlanTransitionApproved {
                    plan_name: body.name.clone().into(),
                    actor,
                },
            );
            specify_workflow::journal::append_batch(ctx.layout(), std::slice::from_ref(&event))?;
        }
        (TransitionKind::Undo, true) => {
            let pair = body.undo.ok_or_else(|| Error::Diag {
                code: "plan-transition-undo",
                detail: "undo body must carry the status pair".to_string(),
            })?;
            let event = specify_workflow::journal::Event::new(
                ctx.now(),
                specify_workflow::journal::EventKind::PlanTransitionUndone {
                    plan_name: body.plan.name.clone().into(),
                    slice_name: body.name.clone().into(),
                    from: pair.from,
                    to: pair.to,
                },
            );
            specify_workflow::journal::append_batch(ctx.layout(), std::slice::from_ref(&event))?;
        }
        _ => {}
    }
    ctx.write(&body, write_transition_text)?;
    Ok(())
}

fn dispatch_undo(
    plan: &mut Plan, layout: specify_workflow::config::Layout<'_>, plan_path: &std::path::Path,
    name: &str,
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
    // Per-entry status walks are loop-phase plan-state
    // writes — refuse an unlocked driver.
    specify_workflow::plan_lock::require_held(layout)?;
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
    plan: &mut Plan, layout: specify_workflow::config::Layout<'_>, plan_path: &std::path::Path,
    name: &str, target: &str,
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
            // The per-entry close is a loop-phase plan-state
            // write — refuse an unlocked driver. The plan-level Gate 1
            // stamp above is exempt (it precedes any driver session);
            // invalid targets fail on the argument before the probe.
            specify_workflow::plan_lock::require_held(layout)?;
            let idx =
                plan.entries.iter().position(|e| e.name == name).ok_or_else(|| Error::Diag {
                    code: "plan-entry-not-found",
                    detail: format!("no slice named '{name}' in plan"),
                })?;
            let previous = plan.entries[idx].status;
            plan.transition(name, Status::Done)?;
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

pub(super) fn archive(ctx: &Ctx, force: bool) -> Result<()> {
    let layout = ctx.layout();
    let plan_path = layout.plan_path();
    if !plan_path.exists() {
        return Err(Error::ArtifactNotFound {
            kind: "plan.yaml",
            path: plan_path,
        });
    }
    let archive_dir = layout.archive_dir().join("plans");
    let brief_path = layout.change_brief_path();
    let plan_name = Plan::load(&plan_path)?.name.into_string();

    let (archived, archived_plans_dir) =
        Plan::archive(&plan_path, &brief_path, &archive_dir, force, ctx.now())?;
    ctx.write(
        &ArchiveBody {
            archived: archived.display().to_string(),
            archived_plans_dir: archived_plans_dir.as_deref().map(|p| p.display().to_string()),
            plan: ArchivedPlan { name: plan_name },
        },
        write_archive_text,
    )?;
    Ok(())
}

/// Render the plan-validate findings through the shared
/// diagnostic-report kernel with the `ERROR`/`WARNING` row format; a
/// finding-free report renders `Plan OK` instead of the banner.
fn render_validate_report(ctx: &Ctx, results: Vec<Diagnostic>) -> Result<()> {
    crate::runtime::commands::render_diagnostic_report(
        ctx,
        results,
        Some("Plan OK"),
        write_validate_row_text,
    )
}

fn write_validate_row_text(w: &mut dyn Write, finding: &Diagnostic) -> std::io::Result<()> {
    let label = if blocking(finding) { "ERROR  " } else { "WARNING" };
    let code = finding.rule_id.as_deref().unwrap_or("<unknown>");
    let entry_col = finding.slice.as_ref().map_or_else(String::new, |e| format!("[{e}]"));
    writeln!(w, "{label} {:<32} {:<24} {}", code, entry_col, finding.impact)
}

/// Text rendering for `plan status`: a plan/entries header, then the
/// next-action line. Stops render the stop-conditions block shape
/// (`stop: <reason>` + indented context + `hint:`); drained renders
/// the literal stop-conditions drained string.
fn write_status_text(w: &mut dyn Write, body: &StatusBody) -> std::io::Result<()> {
    writeln!(w, "plan: {} ({})", body.plan, body.lifecycle)?;
    writeln!(
        w,
        "entries: {} done / {} in-progress / {} pending",
        body.counts.done, body.counts.in_progress, body.counts.pending
    )?;
    match (body.action, &body.stop) {
        (NextActionKind::Drained, _) => writeln!(w, "{}", drained_line(&body.plan))?,
        (NextActionKind::Stop, Some(stop)) => {
            writeln!(w, "stop: {}", stop.reason)?;
            if let Some(slice) = &body.slice {
                writeln!(w, "  slice: {slice}")?;
                writeln!(w, "  project: {}", body.project.as_deref().unwrap_or("-"))?;
            }
            if let Some(detail) = &stop.detail {
                writeln!(w, "  detail: {detail}")?;
            }
            writeln!(w, "hint: {}", stop.hint)?;
        }
        _ => writeln!(w, "next-action: {}", body.next_action)?,
    }
    if body.action != NextActionKind::Drained
        && let Some(resume) = &body.resume
    {
        writeln!(w, "resume: {resume}")?;
    }
    Ok(())
}

fn write_next_text(w: &mut dyn Write, body: &NextBody) -> std::io::Result<()> {
    if let Some(active) = &body.active {
        writeln!(w, "Active change in progress: {active}")
    } else if let Some(name) = &body.next {
        writeln!(w, "{name}")
    } else if body.reason == Some(NextReason::Drained) {
        writeln!(w, "Plan drained — no per-entry pending or in-progress remains.")
    } else {
        writeln!(
            w,
            "No eligible changes \u{2014} remaining entries are waiting on unmet dependencies."
        )
    }
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum TransitionKind {
    Plan,
    Entry,
    Undo,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct TransitionBody {
    plan: Ref,
    kind: TransitionKind,
    name: String,
    previous: String,
    current: String,
    /// `false` when the transition was an idempotent no-op (workflow
    /// rules-root resolution — explicit `approved` after `--auto-approve`); `true`
    /// when the lifecycle / status actually moved. The outer
    /// handler reads this to decide whether to fire the
    /// `plan.transition.approved` journal event.
    #[serde(skip)]
    changed: bool,
    /// Status pair the `--undo` walk visited, if any. `None` on
    /// forward transitions and on undo failures that never reached
    /// the mutation step. Surfaced on the JSON envelope under
    /// `undo: { from, to }` so wire consumers can branch on the
    /// reverse step without re-parsing `previous` / `current`.
    #[serde(skip_serializing_if = "Option::is_none")]
    undo: Option<UndoPair>,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
struct UndoPair {
    from: Status,
    to: Status,
}

fn write_transition_text(w: &mut dyn Write, body: &TransitionBody) -> std::io::Result<()> {
    match body.kind {
        TransitionKind::Plan if !body.changed => {
            writeln!(w, "Plan '{}' is already at lifecycle: {} (no-op).", body.name, body.current)
        }
        TransitionKind::Plan => writeln!(
            w,
            "Stamped plan '{}': lifecycle {} \u{2192} {}.",
            body.name, body.previous, body.current
        ),
        TransitionKind::Entry => writeln!(
            w,
            "Transitioned '{}': {} \u{2192} {}.",
            body.name, body.previous, body.current
        ),
        TransitionKind::Undo => {
            writeln!(w, "Undid '{}': {} \u{2192} {}.", body.name, body.previous, body.current)
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct ArchiveBody {
    archived: String,
    archived_plans_dir: Option<String>,
    plan: ArchivedPlan,
}

fn write_archive_text(w: &mut dyn Write, body: &ArchiveBody) -> std::io::Result<()> {
    match &body.archived_plans_dir {
        Some(dir) => {
            writeln!(w, "Archived plan to {}. Working directory moved to {dir}.", body.archived)
        }
        None => writeln!(w, "Archived plan to {}.", body.archived),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct ArchivedPlan {
    name: String,
}
