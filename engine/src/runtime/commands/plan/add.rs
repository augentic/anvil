//! `specify plan add` handler — append one slice entry to an
//! existing `plan.yaml`. Authority-override seeding is delegated to
//! the shared domain helper so the journal events match `plan create`
//! and `plan amend` byte-for-byte.

use std::collections::BTreeMap;

use specify_error::Result;
use specify_workflow::change::{
    Entry, Plan, SliceAuthorityOverride, Status, emit_authority_override_seed_events, entry_mut,
};
use specify_workflow::config::with_state;
use specify_workflow::journal;
use specify_workflow::schema::validate_plan;

use super::args::{bindings_from_args, load_discovery};
use super::cli::AddArgs;
use super::entry::{Action, EntryBody, write_entry_text};
use super::{check_project, plan_ref};
use crate::runtime::context::Ctx;

pub(super) fn add(ctx: &Ctx, args: AddArgs) -> Result<()> {
    let AddArgs {
        name,
        depends_on,
        sources,
        description,
        project,
        context,
        authority_override,
    } = args;
    let name = name.as_str();
    let authority_override = authority_override.as_slice();

    if let Some(proj) = &project {
        check_project(&ctx.project_dir, proj)?;
    }

    // When `discovery.md` exists, resolve `--sources <key>=<lead>` to the
    // canonical lead id before persisting. Absence of `discovery.md`
    // short-circuits to the verbatim path.
    let discovery = load_discovery(ctx.layout())?;
    let sources = bindings_from_args(sources, name, discovery.as_ref())?;
    let authority_override_map = SliceAuthorityOverride {
        by_kind: authority_override
            .iter()
            .map(|a| (a.kind, a.source.clone()))
            .collect::<BTreeMap<_, _>>(),
    };
    let entry = Entry {
        name: name.into(),
        project,
        status: Status::Pending,
        depends_on: depends_on.into_iter().map(Into::into).collect(),
        sources,
        context,
        description,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: authority_override_map,
    };
    let plan_path = ctx.layout().plan_path();
    let now = ctx.now();
    let (body, override_events) =
        with_state::<Plan, _, _>(ctx.layout(), "plan.yaml", move |plan| {
            plan.create(entry)?;
            validate_plan(plan)?;
            let plan_name = plan.name.clone();
            // Route the seeded overrides through the shared writer
            // (no clears on the add path) so all three handlers emit
            // identically-shaped, identically-sorted Set events.
            let created_entry = entry_mut(plan, &plan_name, name)?.clone();
            let events = emit_authority_override_seed_events(&plan_name, &created_entry, now);
            Ok((
                EntryBody {
                    plan: plan_ref(plan, &plan_path),
                    action: Action::Create,
                    entry: created_entry,
                },
                events,
            ))
        })?;

    journal::append_batch(ctx.layout(), &override_events)?;
    ctx.write(&body, write_entry_text)?;
    Ok(())
}
