//! `specify plan create` handler. Composes the shared CLI arg
//! parsers in [`super::args`] with the domain authority-override
//! engine in [`specify_workflow::change::mutate_authority_overrides`]
//! so the handler stays declarative.

use std::io::Write;

use serde::Serialize;
use specify_error::{Error, Result, is_kebab};
use specify_workflow::change::{
    Lifecycle, Plan, mutate_authority_overrides, reject_orphan_overrides,
};
use specify_workflow::journal;

use super::args::{build_source_map, parse_override_assigns};
use crate::runtime::cli::SourceArg;
use crate::runtime::context::Ctx;

/// `specify plan create <name> [--source ...] [--auto-approve]`.
///
/// Scaffolds an empty `plan.yaml` (workflow §The Plan); slices are
/// authored later by `specify plan propose --from` or `specify plan
/// add`.
///
/// When `--auto-approve` is set (auto-approve Gate-1 contract), the plan is constructed
/// with `lifecycle: approved` *before* the single atomic
/// `plan.save` — there is never a transient `lifecycle: pending`
/// file on disk. The matching `plan.transition.approved` journal
/// event is appended in the same batched write as any
/// `plan.amend.authority-override` events the same invocation
/// produced; validation failures (kebab-case name, orphan source
/// key) refuse the create with or without the flag and leave the
/// journal untouched.
pub(super) fn create(
    ctx: &Ctx, name: String, sources: Vec<SourceArg>, auto_approve: bool,
    authority_override: &[String],
) -> Result<()> {
    if !is_kebab(&name) {
        return Err(Error::Diag {
            code: "change-name-not-kebab",
            detail: format!(
                "change: name `{name}` must be kebab-case \
                 (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens)"
            ),
        });
    }
    let source_map = build_source_map(sources)?;
    let plan_path = ctx.layout().plan_path();
    if plan_path.exists() {
        return Err(Error::Diag {
            code: "already-exists",
            detail: format!("refusing to overwrite existing plan at {}", plan_path.display()),
        });
    }

    let override_assigns = parse_override_assigns(authority_override)?;

    let mut plan = Plan::init(&name, source_map)?;
    // Route `--authority-override` through the shared mutation
    // helper used by `plan amend` so create and amend produce
    // byte-identical `plan.amend.authority-override` journal events
    // and share the unknown-slice gate. Empty `clears` / `clear_all`
    // slices keep the create path scoped to set-only semantics.
    let now = ctx.now();
    let plan_name = plan.name.clone();
    let override_events =
        mutate_authority_overrides(&mut plan, &plan_name, &override_assigns, &[], &[], now)?;
    // Re-run the orphan-source gate after the override
    // pre-seeding: `Plan::init` ran no validation against the
    // override map (it didn't exist yet) and `validate_plan` only
    // checks JSON Schema. The orphan check is the only per-slice authority override
    // gate that fires on this code path.
    reject_orphan_overrides(&plan)?;
    if auto_approve {
        plan.transition_lifecycle(Lifecycle::Approved)?;
    }
    plan.save(&plan_path)?;

    // Collect every journal event the invocation produced, then
    // hand the slice to `append_batch` so the post-save log write is
    // a single fsynced append. Either every event lands or none
    // does — `--auto-approve` and `--authority-override` compose
    // without a partial-state window in the journal.
    let mut events: Vec<journal::Event> = Vec::new();
    if auto_approve {
        // Typing `--auto-approve` *is* the operator's Gate-1 consent,
        // so the create path always records `actor: operator`.
        events.push(journal::Event::new(
            now,
            journal::EventKind::PlanTransitionApproved {
                plan_name: plan_name.clone(),
                actor: journal::Actor::Operator,
            },
        ));
    }
    events.extend(override_events);
    journal::append_batch(ctx.layout(), &events)?;

    ctx.write(
        &CreateBody {
            name,
            plan: plan_path.display().to_string(),
            lifecycle: plan.lifecycle,
        },
        write_create_text,
    )?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct CreateBody {
    name: String,
    plan: String,
    /// Final plan-level lifecycle persisted to disk — `pending` for
    /// the default create, `approved` when `--auto-approve` was set.
    /// Exposed in the JSON envelope so skill bodies and tests can
    /// branch on the on-disk state without re-reading `plan.yaml`.
    lifecycle: Lifecycle,
}

fn write_create_text(w: &mut dyn Write, body: &CreateBody) -> std::io::Result<()> {
    match body.lifecycle {
        Lifecycle::Pending => writeln!(w, "Initialised plan '{}' at {}.", body.name, body.plan),
        Lifecycle::Approved => writeln!(
            w,
            "Initialised plan '{}' at {} and stamped lifecycle: approved.",
            body.name, body.plan
        ),
    }
}
