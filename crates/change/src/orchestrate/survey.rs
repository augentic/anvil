//! Source-axis survey: per-binding dispatch and the plan-order fan-out.

use std::path::Path;

use artifacts::discovery::{Discovery, Lead as DiscoveryLead, validate_leads};
use error::Error;
use jiff::Timestamp;
use project::adapter::{Resolver, SourceOperation};
use project::config::Layout;
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind};
use project::plan::{Plan, SourceBinding};
use project::seam::{Source, seam_failure, source_id};

/// One source's merged survey result under [`survey_all`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurveyedSource {
    /// Plan source binding key (`plan.yaml.sources.<key>`).
    pub source: String,
    /// Bound source adapter name.
    pub adapter: String,
    /// Lead ids merged into `discovery.md`, in survey order.
    pub leads: Vec<String>,
}

/// Survey every `plan.yaml` source binding through the seam and merge
/// each lead set into `discovery.md`.
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
    seam: &impl Source, resolver: &impl Resolver, paths: &ExecutionPaths, now: Timestamp,
) -> Result<Vec<SurveyedSource>, Error> {
    let layout = Layout::new(paths.project_root());
    let plan = Plan::load(&layout.plan_path())?;
    let mut surveyed = Vec::with_capacity(plan.sources.len());
    for (source, binding) in &plan.sources {
        surveyed.push(survey_one(seam, resolver, paths, now, source, binding).await?);
    }
    Ok(surveyed)
}

/// Survey one `plan.yaml` source binding (`specify source survey
/// <source>`) and merge its lead set into `discovery.md`.
///
/// Resolves `source` against `plan.yaml.sources.<key>`, optionally
/// guards the plan name, then dispatch → attribute → validate → merge
/// → journal.
///
/// # Errors
///
/// `source-unknown` for an unbound source key, an `--plan` argument
/// error when the guard fails, adapter ensure/resolve failures
/// (missing pin, `specify_floor`), seam and schema-gate failures from
/// the adapter's survey leg, plus plan-load and merge I/O failures.
pub async fn survey(
    seam: &impl Source, resolver: &impl Resolver, paths: &ExecutionPaths, now: Timestamp,
    source: &str, plan_guard: Option<&str>,
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
    let binding = plan.sources.get(source).ok_or_else(|| Error::Diag {
        code: "source-unknown",
        detail: format!(
            "no source `{source}` in plan.yaml.sources; `specify source survey` resolves \
             its argument against the plan's source keys, not the adapter name"
        ),
    })?;
    survey_one(seam, resolver, paths, now, source, binding).await
}

/// Survey one binding: ensure/resolve, dispatch, attribute, validate,
/// merge, journal.
async fn survey_one(
    seam: &impl Source, resolver: &impl Resolver, paths: &ExecutionPaths, now: Timestamp,
    source: &str, binding: &SourceBinding,
) -> Result<SurveyedSource, Error> {
    let layout = Layout::new(paths.project_root());

    // Ensure/resolve before dispatch: the binding's pin and the
    // adapter's `specify_floor` are enforced by the deployment's
    // resolver, and dispatch routes by the resolved name only.
    let adapter = resolver.ensure_source(&binding.selector(), paths).await?.manifest.name;

    emit(
        layout,
        now,
        EventKind::SourceExecutionAgent {
            source: source.to_string(),
            adapter: adapter.clone(),
            operation: SourceOperation::Survey,
        },
    )?;

    let id = source_id(&adapter);
    let raw = seam.survey(id.clone()).await.map_err(|err| seam_failure("survey", &id, &err))?;

    // Attribution is orchestrator-owned, mirroring the native verb: a
    // `survey` for `source` produces `source`'s leads, so stamp every
    // lead before the validity check and the merge.
    let leads: Vec<DiscoveryLead> = raw
        .into_iter()
        .map(|lead| DiscoveryLead {
            lead: lead.lead,
            source: source.to_string(),
            synopsis: lead.synopsis,
            topics: lead.topics,
        })
        .collect();
    validate_leads(&leads)?;
    let lead_ids: Vec<String> = leads.iter().map(|lead| lead.lead.clone()).collect();

    let discovery_path = layout.discovery_path();
    let mut discovery = load_or_empty_discovery(&discovery_path)?;
    discovery.merge_survey(source, leads, &discovery_path)?;

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

/// Load `discovery.md`, or start from an empty document when the file
/// is absent so the first survey can author the inventory.
fn load_or_empty_discovery(path: &Path) -> Result<Discovery, Error> {
    if path.exists() { Discovery::load(path) } else { Discovery::parse("") }
}

/// Append one journal event, propagating the write failure — the
/// source-axis emits are fallible (strict [`journal::append_one`]),
/// unlike the best-effort build/merge brackets.
fn emit(layout: Layout<'_>, now: Timestamp, kind: EventKind) -> Result<(), Error> {
    journal::append_one(layout, &Event::new(now, kind))
}
