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
use project::pool;
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
/// Dispatch fans out on the bounded pool (RFC-96 D5); lead sets are
/// joined, merged, and journalled serially in plan order — never
/// completion order — so the catalog is byte-identical at every cap.
/// The first failure (in plan order) drains in-flight siblings and
/// aborts with earlier sources already merged.
///
/// # Errors
///
/// Plan-load failures plus whatever the first failing binding's
/// survey leg surfaces.
pub async fn survey_all(
    seam: &impl Source, resolver: &impl Resolver, workspaces: &impl Workspaces,
    paths: &ExecutionPaths, now: Timestamp,
) -> Result<Vec<SurveyedSource>, Error> {
    let layout = paths.layout();
    let plan = Plan::load(&layout.plan_path())?;
    let bindings: Vec<(&String, &SourceBinding)> = plan.sources.iter().collect();
    let claims = pool::Claims::default();
    let jobs: Vec<pool::Job<'_, (Vec<CatalogLead>, String), Error>> = bindings
        .iter()
        .map(|(source, binding)| pool::Job {
            claim: pool::Claim {
                item: (*source).clone(),
                operation: "survey".to_string(),
                attempt: 1,
            },
            budget: pool::budget::SURVEY,
            future: Box::pin(collect_leads(
                seam, resolver, workspaces, paths, now, source, binding, None,
            )),
        })
        .collect();
    let outcomes = pool::run(pool::cap(), &claims, pool::OnFailure::Drain, jobs).await;

    let mut surveyed = Vec::with_capacity(bindings.len());
    for (outcome, (source, _)) in outcomes.into_iter().zip(&bindings) {
        let (leads, adapter) = settle_survey(outcome, source)?;
        surveyed.push(merge_surveyed(layout, now, source, None, leads, adapter)?);
    }
    Ok(surveyed)
}

/// Fold one pool outcome into the survey error surface.
fn settle_survey(
    outcome: pool::Outcome<(Vec<CatalogLead>, String), Error>, source: &str,
) -> Result<(Vec<CatalogLead>, String), Error> {
    match outcome {
        pool::Outcome::Settled(result) => result,
        pool::Outcome::TimedOut => Err(Error::Diag {
            code: "source-survey-timeout",
            detail: format!("survey of `{source}` exceeded its inactivity budget; re-run"),
        }),
        pool::Outcome::Rejected | pool::Outcome::Cancelled | pool::Outcome::Skipped => {
            Err(Error::Diag {
                code: "source-survey-cancelled",
                detail: format!("survey of `{source}` did not run (a sibling survey failed first)"),
            })
        }
    }
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
    let layout = paths.layout();
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
    let layout = paths.layout();
    let (leads, adapter) =
        collect_leads(seam, resolver, workspaces, paths, now, source, binding, parent).await?;
    merge_surveyed(layout, now, source, parent, leads, adapter)
}

/// Merge one settled lead set into `leads.md` and journal it — the
/// deterministic tail behind both the single survey and the pool
/// fan-out's plan-order join.
fn merge_surveyed(
    layout: Layout<'_>, now: Timestamp, source: &str, parent: Option<&Lead>,
    leads: Vec<CatalogLead>, adapter: String,
) -> Result<SurveyedSource, Error> {
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
            adapter: adapter.clone(),
        },
    )?;
    Ok(SurveyedSource {
        source: source.to_string(),
        adapter,
        leads: lead_ids,
    })
}

/// Focused survey that returns attributed child leads without merging
/// into `leads.md`.
#[expect(
    clippy::too_many_arguments,
    reason = "source dispatch carries the workspace capability beside the existing seam kernel"
)]
pub async fn focused_leads(
    seam: &impl Source, resolver: &impl Resolver, workspaces: &impl Workspaces,
    paths: &ExecutionPaths, now: Timestamp, source: &str, binding: &SourceBinding, catalog: &Leads,
    lead: &str,
) -> Result<Vec<CatalogLead>, Error> {
    let parent = resolve_focus_in(catalog, source, lead)?;
    let (leads, _) =
        collect_leads(seam, resolver, workspaces, paths, now, source, binding, Some(&parent))
            .await?;
    Ok(leads)
}

#[expect(
    clippy::too_many_arguments,
    reason = "source dispatch carries the workspace capability beside the existing seam kernel"
)]
async fn collect_leads(
    seam: &impl Source, resolver: &impl Resolver, workspaces: &impl Workspaces,
    paths: &ExecutionPaths, now: Timestamp, source: &str, binding: &SourceBinding,
    parent: Option<&Lead>,
) -> Result<(Vec<CatalogLead>, String), Error> {
    let layout = paths.layout();
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
    Ok((leads, adapter.name))
}

fn resolve_focus(layout: Layout<'_>, source: &str, lead: &str) -> Result<Lead, Error> {
    let catalog = Leads::load(&layout.leads_path())?;
    resolve_focus_in(&catalog, source, lead)
}

fn resolve_focus_in(catalog: &Leads, source: &str, lead: &str) -> Result<Lead, Error> {
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
