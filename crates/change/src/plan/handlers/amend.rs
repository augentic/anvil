//! `emery plan amend` — wholesale edits via [`Plan::amend`], additive
//! `add-source` / `remove-source` edits via direct entry mutation,
//! authority-override assignments, and `--proposal` application.

use artifacts::evidence::ClaimKind;
use error::Error;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::config::with_state;
use project::handler::{Anchor, Ctx, Render};
use project::journal;
use project::plan::proposal::{self, Applied};
use project::plan::{
    Divergence, EntryPatch, Patch, Plan, SliceSourceBinding, authority_override, entry_mut,
    reject_duplicate_source,
};
use project::snapshot::SnapshotId;
use serde::{Deserialize, Serialize};

use super::entry::{Action, EntryBody};
use super::plan_ref;
use crate::plan::wire::{BindingArg, KindAssign, bindings_from_args, load_leads, parse_divergence};

/// Wire input for `plan amend`. Option-typed fields distinguish
/// "leave unchanged" (`None`) from "replace/clear" (`Some`), matching
/// the flag semantics. `name` is required unless `proposal` is set.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AmendInput {
    /// Kebab-case plan entry (slice) name being edited.
    #[serde(default)]
    pub name: Option<String>,
    /// Apply the retained proposal at this digest instead of editing
    /// one entry.
    #[serde(default)]
    pub proposal: Option<String>,
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

/// Success body: an entry edit or a proposal application.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum AmendBody {
    /// Direct entry mutation.
    Entry(EntryBody),
    /// Applied amendment proposal.
    Applied(AppliedBody),
}

/// Wire body for `plan amend --proposal`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AppliedBody {
    /// Applied proposal digest.
    pub digest: SnapshotId,
    /// Projected slice names after reprojection.
    pub slices: Vec<String>,
    /// New `leads-digest`.
    pub leads_digest: SnapshotId,
    /// New `decomposition-digest`.
    pub decomposition_digest: SnapshotId,
}

impl From<Applied> for AppliedBody {
    fn from(applied: Applied) -> Self {
        Self {
            digest: applied.digest,
            slices: applied.slices,
            leads_digest: applied.leads_digest,
            decomposition_digest: applied.decomposition_digest,
        }
    }
}

impl Render for AmendBody {
    fn render(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        match self {
            Self::Entry(body) => body.render(w),
            Self::Applied(body) => {
                writeln!(w, "applied proposal `{}`", body.digest)?;
                if !body.slices.is_empty() {
                    writeln!(w, "  slices: {}", body.slices.join(", "))?;
                }
                Ok(())
            }
        }
    }
}

/// `emery plan amend <name> [flags]` — edit non-status fields on an
/// existing plan entry — or `emery plan amend --proposal <digest>`.
#[derive(Clone, Copy, Debug)]
pub struct Amend;

impl<P: Anchor> Operation<P> for Amend {
    type Error = project::handler::Error;
    type Input = AmendInput;
    type Output = AmendBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        if let Some(digest) = input.proposal.as_ref() {
            refuse_combo(&input)?;
            let digest = SnapshotId::parse(digest)?;
            let applied = proposal::apply(cx.layout(), cx.now(), &digest)?;
            return Ok(AmendBody::Applied(applied.into()));
        }
        Ok(amend_entry(&cx, input)?)
    }
}

fn amend_entry(cx: &Ctx, input: AmendInput) -> Result<AmendBody, Error> {
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
        ..
    } = input;
    let name = name.ok_or_else(|| Error::Argument {
        flag: "name",
        detail: "plan amend requires an entry name or --proposal <digest>".into(),
    })?;

    let divergence = divergence.as_deref().map(parse_divergence).transpose()?;
    let override_sets: Vec<(String, ClaimKind, String)> =
        authority_override.iter().map(|a| (name.clone(), a.kind, a.source.clone())).collect();
    let override_clears: Vec<(String, ClaimKind)> =
        clear_authority_override.iter().map(|kind| (name.clone(), *kind)).collect();
    let override_clear_all: Vec<String> =
        if clear_authority_overrides { vec![name.clone()] } else { Vec::new() };
    let layout = cx.layout();
    let plan_path = layout.plan_path();
    let leads = load_leads(layout)?;
    let now = cx.now();
    let topology =
        has_topology(sources.as_deref(), depends_on.as_deref(), &add_source, &remove_source);
    let (body, journal_events) = with_state::<Plan, _, _>(layout, "plan.yaml", move |plan| {
        let sources_replace =
            sources.as_ref().map(|v| bindings_from_args(v, &name, leads.as_ref())).transpose()?;
        let add_bindings = bindings_from_args(&add_source, &name, leads.as_ref())?;
        let plan_name = plan.name.clone();
        let previous_divergence =
            plan.entries.iter().find(|e| e.name == name).and_then(|e| e.divergence);

        if proposal::has_tree(layout) && topology {
            reproject(
                layout,
                plan,
                &name,
                sources_replace,
                add_bindings,
                &remove_source,
                depends_on,
            )?;
        } else if topology {
            let patch = EntryPatch {
                depends_on: depends_on.map(|v| v.into_iter().map(Into::into).collect()),
                sources: sources_replace,
                description: Patch::Keep,
                context: None,
                divergence: None,
                allow_composition_replace: None,
            };
            plan.amend(&name, patch)?;
            apply_source_edits(plan, &plan_name, &name, add_bindings, &remove_source)?;
            reject_duplicate_source(plan)?;
        }

        let patch = EntryPatch {
            depends_on: None,
            sources: None,
            description: Patch::from_string_option(description.clone()),
            context: context.clone(),
            divergence,
            allow_composition_replace,
        };
        plan.amend(&name, patch)?;

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

        let mut journal_events =
            divergence_events(now, plan_name, previous_divergence, divergence, &amended.name);
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

    Ok(AmendBody::Entry(body))
}

const fn has_topology(
    sources: Option<&[BindingArg]>, depends_on: Option<&[String]>, add_source: &[BindingArg],
    remove_source: &[String],
) -> bool {
    sources.is_some() || depends_on.is_some() || !add_source.is_empty() || !remove_source.is_empty()
}

fn reproject(
    layout: project::config::Layout<'_>, plan: &mut Plan, name: &str,
    mut sources: Option<Vec<SliceSourceBinding>>, add: Vec<SliceSourceBinding>, remove: &[String],
    depends_on: Option<Vec<String>>,
) -> Result<(), Error> {
    if sources.is_none() && (!add.is_empty() || !remove.is_empty()) {
        let current = plan
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .ok_or_else(|| plan.entry_not_found(name))?;
        let mut next = current.sources.clone();
        apply_source_list(&mut next, name, add, remove)?;
        sources = Some(next);
    }
    let scopes = sources.as_ref().map(|bindings| {
        bindings
            .iter()
            .map(|binding| {
                project::plan::decomposition::Scope::new(binding.source(), binding.lead(name))
            })
            .collect()
    });
    proposal::amend_tree(layout, plan, name, scopes, depends_on)
}

fn divergence_events(
    now: jiff::Timestamp, plan_name: project::name::PlanName, previous: Option<Divergence>,
    to: Option<Divergence>, slice: &project::name::SliceName,
) -> Vec<journal::Event> {
    let Some(to) = to else {
        return Vec::new();
    };
    vec![journal::Event::new(
        now,
        journal::EventKind::PlanAmendDivergence {
            plan_name,
            slice_name: slice.clone(),
            from: previous.unwrap_or(Divergence::None),
            to,
        },
    )]
}

fn refuse_combo(input: &AmendInput) -> Result<(), Error> {
    let extra = input.depends_on.is_some()
        || input.sources.is_some()
        || !input.add_source.is_empty()
        || !input.remove_source.is_empty()
        || input.divergence.is_some()
        || input.description.is_some()
        || input.context.is_some()
        || !input.authority_override.is_empty()
        || !input.clear_authority_override.is_empty()
        || input.clear_authority_overrides
        || input.allow_composition_replace.is_some();
    if extra {
        return Err(Error::Argument {
            flag: "--proposal",
            detail: "cannot combine --proposal with entry-edit flags".into(),
        });
    }
    Ok(())
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
    apply_source_list(&mut entry.sources, slice, add_bindings, remove_source)
}

fn apply_source_list(
    sources: &mut Vec<SliceSourceBinding>, slice: &str, add_bindings: Vec<SliceSourceBinding>,
    remove_source: &[String],
) -> Result<(), Error> {
    for key in remove_source {
        let before = sources.len();
        sources.retain(|b| b.source() != key.as_str());
        if sources.len() == before {
            return Err(Error::Diag {
                code: "plan-binding-not-found",
                detail: format!("slice `{slice}` has no source binding with key `{key}`"),
            });
        }
    }
    for binding in add_bindings {
        sources.push(binding);
    }
    Ok(())
}
