//! `emery plan amend` — wholesale edits via [`Plan::amend`], additive
//! `add-source` / `remove-source` edits via direct entry mutation, and
//! authority-override assignments via the shared domain engine.

use artifacts::evidence::ClaimKind;
use error::Error;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::config::with_state;
use project::handler::{Anchor, Ctx};
use project::journal;
use project::plan::{
    Divergence, EntryPatch, Patch, Plan, SliceSourceBinding, authority_override, entry_mut,
    reject_duplicate_source_keys,
};
use serde::{Deserialize, Serialize};

use super::entry::{Action, EntryBody};
use super::plan_ref;
use crate::plan::wire::{
    BindingArg, KindAssign, bindings_from_args, load_discovery, parse_divergence,
};

/// Wire input for `plan amend`. Option-typed fields distinguish
/// "leave unchanged" (`None`) from "replace/clear" (`Some`), matching
/// the flag semantics.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AmendInput {
    /// Kebab-case plan entry (slice) name being edited.
    pub name: String,
    /// Replace `depends_on` wholesale; `None` leaves it unchanged.
    #[serde(default)]
    pub depends_on: Option<Vec<String>>,
    /// Replace per-slice source bindings wholesale; `None` leaves
    /// them unchanged.
    #[serde(default)]
    pub sources: Option<Vec<BindingArg>>,
    /// Add single per-slice source bindings.
    #[serde(default)]
    pub add_source: Vec<BindingArg>,
    /// Remove per-slice source bindings by key.
    #[serde(default)]
    pub remove_source: Vec<String>,
    /// Set the slice's `divergence` field (`likely` / `accepted` /
    /// `rejected`); `None` leaves it unchanged.
    #[serde(default)]
    pub divergence: Option<String>,
    /// Replace description (`Some("")` clears); `None` leaves it
    /// unchanged.
    #[serde(default)]
    pub description: Option<String>,
    /// Replace context paths; `None` leaves them unchanged.
    #[serde(default)]
    pub context: Option<Vec<String>>,
    /// `<kind>=<source>` authority-override sets on the amended entry.
    #[serde(default)]
    pub authority_override: Vec<KindAssign>,
    /// Claim kinds cleared from the amended entry's override map.
    #[serde(default)]
    pub clear_authority_override: Vec<ClaimKind>,
    /// Wipe the amended entry's whole authority-override map.
    #[serde(default)]
    pub clear_authority_overrides: bool,
    /// Set the entry's `allow-composition-replace` field — the merge
    /// step's whole-document composition-overwrite authorization;
    /// `None` leaves it unchanged.
    #[serde(default)]
    pub allow_composition_replace: Option<bool>,
}

/// `emery plan amend <name> [flags]` — edit non-status fields on an
/// existing plan entry.
#[derive(Clone, Copy, Debug)]
pub struct Amend;

impl<P: Anchor> Operation<P> for Amend {
    type Error = project::handler::Error;
    type Input = AmendInput;
    type Output = EntryBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let AmendInput {
            name,
            depends_on,
            sources,
            add_source,
            remove_source,
            divergence,
            description,
            context,
            authority_override,
            clear_authority_override,
            clear_authority_overrides,
            allow_composition_replace,
        } = input;

        let divergence = divergence.as_deref().map(parse_divergence).transpose()?;
        // Overrides are scoped to the entry being amended — the shared
        // mutation engine keys by (slice, kind), so widen here.
        let override_sets: Vec<(String, ClaimKind, String)> =
            authority_override.iter().map(|a| (name.clone(), a.kind, a.source.clone())).collect();
        let override_clears: Vec<(String, ClaimKind)> =
            clear_authority_override.iter().map(|kind| (name.clone(), *kind)).collect();
        let override_clear_all: Vec<String> =
            if clear_authority_overrides { vec![name.clone()] } else { Vec::new() };
        let plan_path = cx.layout().plan_path();
        let discovery = load_discovery(cx.layout())?;
        let now = cx.now();
        let (body, journal_events) =
            with_state::<Plan, _, _>(cx.layout(), "plan.yaml", move |plan| {
                let sources_replace = sources
                    .as_ref()
                    .map(|v| bindings_from_args(v, &name, discovery.as_ref()))
                    .transpose()?;
                let add_bindings = bindings_from_args(&add_source, &name, discovery.as_ref())?;

                let plan_name = plan.name.clone();
                let previous_divergence =
                    plan.entries.iter().find(|e| e.name == name).and_then(|e| e.divergence);

                let patch = EntryPatch {
                    depends_on: depends_on.clone().map(|v| v.into_iter().map(Into::into).collect()),
                    sources: sources_replace,
                    description: Patch::from_string_option(description.clone()),
                    context: context.clone(),
                    divergence,
                    allow_composition_replace,
                };
                plan.amend(&name, patch)?;

                apply_source_edits(plan, &plan_name, &name, add_bindings, &remove_source)?;
                // `--add-source` mutates after `Plan::amend`'s validate-and-
                // rollback gate, so re-gate duplicate keys here (a duplicate
                // would silently overwrite `evidence/<source>.yaml` at refine).
                reject_duplicate_source_keys(plan)?;

                let override_journal = authority_override::mutate(
                    plan,
                    &plan_name,
                    &override_sets,
                    &override_clears,
                    &override_clear_all,
                    now,
                )?;
                authority_override::reject_orphans(plan)?;

                let amended = plan
                    .entries
                    .iter()
                    .find(|c| c.name == name)
                    .ok_or_else(|| project::plan::unknown_slice_err(&plan_name, &name))?;

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

                Ok(project::config::Mutation::changed((
                    EntryBody {
                        plan: plan_ref(plan, &plan_path),
                        action: Action::Amend,
                        entry: amended.clone(),
                    },
                    journal_events,
                )))
            })?;
        journal::append_batch(cx.layout(), &journal_events)?;

        Ok(body)
    }
}

/// Apply `add-source` / `remove-source` edits to `slice`'s entry, run
/// after the wholesale `amend` so additive edits compose cleanly with
/// a simultaneous `sources` replacement.
fn apply_source_edits(
    plan: &mut Plan, plan_name: &str, slice: &str, add_bindings: Vec<SliceSourceBinding>,
    remove_source: &[String],
) -> Result<(), Error> {
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
