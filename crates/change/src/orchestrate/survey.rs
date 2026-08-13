//! Source-axis survey: per-binding dispatch and the plan-order fan-out.

use std::path::Path;

use artifacts::leads::{Lead as CatalogLead, Leads, validate_leads};
use error::Error;
use jiff::Timestamp;
use project::adapter::{Resolver, SourceOperation};
use project::config::Layout;
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind};
use project::plan::{Plan, SourceBinding, retain_leads};
use project::seam::{
    Lead, Source, Workspaces, bind_source, discard_source_view, seam_failure, source_id,
};

/// One source's merged survey result under [`survey_all`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurveyedSource {
    /// Plan source binding key (`plan.yaml.sources.<key>`).
    pub source: String,
    /// Bound source adapter name.
    pub adapter: String,
    /// Lead ids merged into `leads.md`, in survey order.
    pub leads: Vec<String>,
}

/// Survey every `plan.yaml` source binding through the seam and merge
/// each lead set into `leads.md`. Unfocused: the complete current set.
///
/// Bindings run in plan order; each is dispatched, source-attributed,
/// validated before it becomes visible, merged, and journalled.
/// The first failure aborts the fan-out with earlier sources already
/// merged.
///
/// # Errors
///
/// Plan-load failures plus whatever [`survey`] surfaces for the first
/// failing binding.
pub async fn survey_all(
    seam: &impl Source, resolver: &impl Resolver, workspaces: &impl Workspaces,
    paths: &ExecutionPaths, now: Timestamp,
) -> Result<Vec<SurveyedSource>, Error> {
    let layout = Layout::new(paths.project_root());
    let plan = Plan::load(&layout.plan_path())?;
    let mut surveyed = Vec::with_capacity(plan.sources.len());
    for (source, binding) in &plan.sources {
        surveyed
            .push(survey_one(seam, resolver, workspaces, paths, now, source, binding, None).await?);
    }
    Ok(surveyed)
}

/// Survey one `plan.yaml` source binding (`emery source survey
/// <source> [--focus <lead>]`) and merge its lead set into `leads.md`.
///
/// Resolves `source` against `plan.yaml.sources.<key>`, optionally
/// guards the plan name, then dispatch → attribute → validate → merge
/// → journal. A focused survey looks the parent up in the catalog,
/// merges children as a new revision, retains `leads/<digest>.md`, and
/// stamps `plan.leads_digest`. Unfocused survey-merge does not retain.
///
/// # Errors
///
/// `plan-source-unknown` for an unbound source key, an `--plan`
/// argument error when the guard fails, `leads-lead-unknown` when
/// `--focus` names no catalog row, adapter ensure/resolve failures,
/// seam and schema-gate failures from the adapter's survey leg, plus
/// plan-load and merge I/O failures.
#[expect(
    clippy::too_many_arguments,
    reason = "source dispatch carries the workspace capability beside the existing seam kernel"
)]
pub async fn survey(
    seam: &impl Source, resolver: &impl Resolver, workspaces: &impl Workspaces,
    paths: &ExecutionPaths, now: Timestamp, source: &str, plan_guard: Option<&str>,
    focus: Option<&str>,
) -> Result<SurveyedSource, Error> {
    let layout = Layout::new(paths.project_root());
    let plan = Plan::load(&layout.plan_path())?;
    if let Some(expected) = plan_guard
        && plan.name.as_str() != expected
    {
        return Err(Error::Argument {
            flag: "--plan",
            detail: format!(
                "--plan `{expected}` does not match the active plan `{}` at plan.yaml",
                plan.name
            ),
        });
    }
    let binding = plan
        .sources
        .get(source)
        .ok_or_else(|| plan.source_not_found("emery source survey", source))?
        .clone();
    let parent = match focus {
        Some(lead) => Some(resolve_focus(layout, source, lead)?),
        None => None,
    };
    survey_one(seam, resolver, workspaces, paths, now, source, &binding, parent.as_ref()).await
}

/// Survey one binding: ensure/resolve, bind value-or-view, dispatch,
/// attribute, validate, merge, journal.
#[expect(
    clippy::too_many_arguments,
    reason = "source dispatch carries the workspace capability beside the existing seam kernel"
)]
#[tracing::instrument(
    name = "source.survey",
    skip_all,
    fields(source = %source, adapter = tracing::field::Empty)
)]
async fn survey_one(
    seam: &impl Source, resolver: &impl Resolver, workspaces: &impl Workspaces,
    paths: &ExecutionPaths, now: Timestamp, source: &str, binding: &SourceBinding,
    parent: Option<&Lead>,
) -> Result<SurveyedSource, Error> {
    let layout = Layout::new(paths.project_root());

    // Ensure/resolve before dispatch: the binding's pin and the
    // adapter's `emery_floor` are enforced by the deployment's
    // resolver, and dispatch routes by the exact resolved identity.
    let adapter = resolver.ensure_source(&binding.selector(), paths).await?.manifest;
    tracing::Span::current().record("adapter", adapter.name.as_str());

    emit(
        layout,
        now,
        EventKind::SourceExecutionAgent {
            source: source.to_string(),
            adapter: adapter.name.clone(),
            operation: SourceOperation::Survey,
        },
    )?;

    let (input, view) = bind_source(workspaces, source, binding, parent.cloned()).await?;
    let id = source_id(&adapter);
    let raw = seam.survey(id.clone(), input).await.map_err(|err| seam_failure("survey", &id, &err));
    discard_source_view(workspaces, view).await;
    let raw = raw?;

    let incoming = match parent {
        None => {
            if !raw.children.is_empty() {
                return Err(Error::Diag {
                    code: "source-survey-shape",
                    detail: format!(
                        "unfocused survey of `{source}` returned children; the complete \
                         current set belongs in `leads`"
                    ),
                });
            }
            raw.leads
        }
        Some(parent) => {
            if !raw.leads.is_empty() {
                return Err(Error::Diag {
                    code: "source-survey-shape",
                    detail: format!(
                        "focused survey of `{source}` / `{}` returned top-level leads; \
                         children belong in `children`",
                        parent.lead
                    ),
                });
            }
            raw.children
        }
    };

    // Attribution is orchestrator-owned: stamp `source`, and on a
    // focused survey stamp parent/focus so the catalog records the
    // engine-chosen parent rather than an adapter guess.
    let leads: Vec<CatalogLead> = incoming
        .into_iter()
        .map(|lead| CatalogLead {
            lead: lead.lead,
            source: source.to_string(),
            synopsis: lead.synopsis,
            topics: lead.topics,
            parent: parent.map(|parent| parent.lead.clone()).or(lead.parent),
            focus: parent.map(|parent| parent.lead.clone()).or(lead.focus),
        })
        .collect();
    validate_leads(&leads)?;
    let lead_ids: Vec<String> = leads.iter().map(|lead| lead.lead.clone()).collect();

    let leads_path = layout.leads_path();
    let mut catalog = if parent.is_some() {
        Leads::load(&leads_path)?
    } else {
        load_or_empty_leads(&leads_path)?
    };
    catalog.merge_survey(source, leads, &leads_path)?;
    if parent.is_some() {
        let digest = retain_leads(layout)?;
        let mut plan = Plan::load(&layout.plan_path())?;
        plan.leads_digest = Some(digest);
        plan.save(&layout.plan_path())?;
    }

    emit(
        layout,
        now,
        EventKind::SourceSurveyCompleted {
            source: source.to_string(),
            adapter: adapter.name.clone(),
        },
    )?;
    Ok(SurveyedSource {
        source: source.to_string(),
        adapter: adapter.name,
        leads: lead_ids,
    })
}

fn resolve_focus(layout: Layout<'_>, source: &str, lead: &str) -> Result<Lead, Error> {
    let catalog = Leads::load(&layout.leads_path())?;
    let resolved = catalog
        .leads()
        .iter()
        .find(|candidate| candidate.source == source && candidate.lead == lead)
        .ok_or_else(|| Error::Diag {
            code: "leads-lead-unknown",
            detail: format!(
                "no lead `{lead}` for source `{source}` in leads.md; focused survey \
                 looks the parent up in the catalog"
            ),
        })?;
    Ok(Lead::from_catalog(resolved))
}

/// Load `leads.md`, or start from an empty catalog when the file
/// is absent so the first survey can author the inventory.
fn load_or_empty_leads(path: &Path) -> Result<Leads, Error> {
    if path.exists() { Leads::load(path) } else { Ok(Leads::empty()) }
}

/// Append one journal event, propagating the write failure — the
/// source-axis emits are fallible (strict [`journal::append_one`]),
/// unlike the best-effort build/merge brackets.
fn emit(layout: Layout<'_>, now: Timestamp, kind: EventKind) -> Result<(), Error> {
    journal::append_one(layout, &Event::new(now, kind))
}
