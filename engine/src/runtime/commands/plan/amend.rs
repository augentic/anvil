//! `specify plan amend` handler. Routes wholesale edits through
//! [`Plan::amend`], additive `--add-source` / `--remove-source`
//! through direct entry mutation, and authority-override flags
//! through the shared domain engine.

use specify_error::{Error, Result};
use specify_model::evidence::ClaimKind;
use specify_workflow::change::{
    Divergence, EntryPatch, Patch, Plan, SliceSourceBinding, entry_mut, mutate_authority_overrides,
    reject_duplicate_source_keys, reject_orphan_overrides,
};
use specify_workflow::config::with_state;
use specify_workflow::journal;
use specify_workflow::schema::validate_plan;

use super::args::{
    bindings_from_args, load_discovery, parse_divergence, parse_override_assigns,
    parse_slice_pair_args,
};
use super::cli::AmendArgs;
use super::entry::{Action, EntryBody, write_entry_text};
use super::{check_project, plan_ref};
use crate::runtime::context::Ctx;

pub(super) fn amend(ctx: &Ctx, args: AmendArgs) -> Result<()> {
    let AmendArgs {
        name,
        depends_on,
        sources,
        add_source,
        remove_source,
        divergence,
        description,
        project,
        context,
        authority_override,
        clear_authority_override,
        clear_authority_overrides,
    } = args;

    if let Some(proj) = &project
        && !proj.is_empty()
    {
        check_project(&ctx.project_dir, proj)?;
    }

    let divergence = divergence.as_deref().map(parse_divergence).transpose()?;
    let override_sets = parse_override_assigns(&authority_override)?;
    let override_clears: Vec<(String, ClaimKind)> = parse_slice_pair_args::<ClaimKind>(
        &clear_authority_override,
        "--clear-authority-override",
    )?;
    let override_clear_all: Vec<String> = clear_authority_overrides;
    let plan_path = ctx.layout().plan_path();
    let discovery = load_discovery(ctx.layout())?;
    let now = ctx.now();
    let (body, journal_events) =
        with_state::<Plan, _, _>(ctx.layout(), "plan.yaml", move |plan| {
            let sources_replace = sources
                .as_ref()
                .map(|v| bindings_from_args(v.clone(), &name, discovery.as_ref()))
                .transpose()?;
            let add_bindings = bindings_from_args(add_source.clone(), &name, discovery.as_ref())?;

            let plan_name = plan.name.clone();
            let previous_divergence =
                plan.entries.iter().find(|e| e.name == name).and_then(|e| e.divergence);

            let patch = EntryPatch {
                depends_on: depends_on.clone().map(|v| v.into_iter().map(Into::into).collect()),
                sources: sources_replace,
                project: Patch::from_string_option(project.clone()),
                description: Patch::from_string_option(description.clone()),
                context: context.clone(),
                divergence,
            };
            plan.amend(&name, patch)?;

            apply_source_edits(plan, &plan_name, &name, add_bindings, &remove_source)?;
            // `--add-source` mutates after `Plan::amend`'s own
            // validate-and-rollback gate, so re-gate duplicate source
            // keys here (a duplicate would silently overwrite
            // `evidence/<source>.yaml` at refine time).
            reject_duplicate_source_keys(plan)?;

            let override_journal = mutate_authority_overrides(
                plan,
                &plan_name,
                &override_sets,
                &override_clears,
                &override_clear_all,
                now,
            )?;
            reject_orphan_overrides(plan)?;

            validate_plan(plan)?;
            let amended =
                plan.entries.iter().find(|c| c.name == name).ok_or_else(|| {
                    specify_workflow::change::unknown_slice_err(&plan_name, &name)
                })?;

            let mut journal_events: Vec<journal::Event> = Vec::new();
            if let Some(to) = divergence {
                journal_events.push(journal::Event::new(
                    now,
                    journal::EventKind::PlanAmendDivergence {
                        plan_name,
                        slice_name: amended.name.clone(),
                        from: previous_divergence.unwrap_or(Divergence::None),
                        to,
                    },
                ));
            }
            journal_events.extend(override_journal);

            Ok((
                EntryBody {
                    plan: plan_ref(plan, &plan_path),
                    action: Action::Amend,
                    entry: amended.clone(),
                },
                journal_events,
            ))
        })?;
    journal::append_batch(ctx.layout(), &journal_events)?;

    ctx.write(&body, write_entry_text)?;
    Ok(())
}

/// Apply `--add-source` / `--remove-source` edits to `slice`'s entry,
/// run after the wholesale `amend` so additive edits compose cleanly
/// with a simultaneous `--sources` replacement.
fn apply_source_edits(
    plan: &mut Plan, plan_name: &str, slice: &str, add_bindings: Vec<SliceSourceBinding>,
    remove_source: &[String],
) -> Result<()> {
    if add_bindings.is_empty() && remove_source.is_empty() {
        return Ok(());
    }
    let entry = entry_mut(plan, plan_name, slice)?;
    for key in remove_source {
        let before = entry.sources.len();
        entry.sources.retain(|b| b.source() != key.as_str());
        if entry.sources.len() == before {
            return Err(Error::Diag {
                code: "plan-binding-not-found",
                detail: format!("slice `{slice}` has no source binding with key `{key}`"),
            });
        }
    }
    for binding in add_bindings {
        entry.sources.push(binding);
    }
    Ok(())
}
