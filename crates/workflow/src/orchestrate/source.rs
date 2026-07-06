//! Source-axis orchestrators: survey fan-out and Evidence extraction.

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::Serialize;
use serde_json::Value as JsonValue;
use specify_error::Error;
use specify_model::atomic::bytes_write;
use specify_model::discovery::{Discovery, Lead as DiscoveryLead};
use specify_model::evidence::AuthorityClass;

use super::{seam_failure, source_adapter_id};
use crate::adapter::SourceOperation;
use crate::change::{Plan, SourceBinding};
use crate::config::Layout;
use crate::journal::{self, Event, EventKind};
use crate::schema;
use crate::seam::SourceSeam;

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
/// The guest collapse of per-source `specify source survey`: for each
/// binding it journals `source.execution.agent`, dispatches
/// `seam.survey`, stamps the source key onto every returned lead (the
/// CLI-owned attribution rule), schema-gates the set *before* it
/// becomes visible, merges via `Discovery::merge_survey`, and journals
/// `source.survey.completed`. Bindings run in plan order; the first
/// failure aborts the fan-out with earlier sources already merged —
/// the same partial-progress posture as running the native verb
/// per-source.
///
/// # Errors
///
/// - propagates `plan.yaml` load failures.
/// - `seam-dispatch-failed` when a seam dispatch fails.
/// - propagates lead schema-validation and discovery-merge failures.
pub async fn survey_all(
    seam: &impl SourceSeam, layout: Layout<'_>, now: Timestamp,
) -> Result<Vec<SurveyedSource>, Error> {
    let plan = Plan::load(&layout.plan_path())?;
    let mut surveyed = Vec::with_capacity(plan.sources.len());
    for (source, binding) in &plan.sources {
        surveyed.push(survey_one(seam, layout, now, source, binding).await?);
    }
    Ok(surveyed)
}

/// Survey one `plan.yaml` source binding through the seam and merge
/// its lead set into `discovery.md`.
///
/// The guest collapse of one `specify source survey <source>`
/// invocation (the per-source counterpart of [`survey_all`]): resolves
/// `source` against `plan.yaml.sources.<key>`, optionally guards the
/// plan name, and runs the same dispatch → attribute → validate →
/// merge → journal leg.
///
/// # Errors
///
/// - propagates `plan.yaml` load failures.
/// - `Error::Argument` (`--plan`) when `plan_guard` names a different
///   plan — the native verb's guard verbatim.
/// - `source-unknown` when `source` is not a `plan.yaml.sources` key.
/// - `seam-dispatch-failed` when the seam dispatch fails.
/// - propagates lead schema-validation and discovery-merge failures.
pub async fn survey(
    seam: &impl SourceSeam, layout: Layout<'_>, now: Timestamp, source: &str,
    plan_guard: Option<&str>,
) -> Result<SurveyedSource, Error> {
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
    survey_one(seam, layout, now, source, binding).await
}

/// Survey one binding: dispatch, attribute, validate, merge, journal.
async fn survey_one(
    seam: &impl SourceSeam, layout: Layout<'_>, now: Timestamp, source: &str,
    binding: &SourceBinding,
) -> Result<SurveyedSource, Error> {
    emit(
        layout,
        now,
        EventKind::SourceExecutionAgent {
            source: source.to_string(),
            adapter: binding.adapter.clone(),
            operation: SourceOperation::Survey,
        },
    )?;

    let id = source_adapter_id(&binding.adapter);
    let raw = seam.survey(id.clone()).await.map_err(|err| seam_failure("survey", &id, &err))?;

    // Attribution is orchestrator-owned, mirroring the native verb: a
    // `survey` for `source` produces `source`'s leads, so stamp every
    // lead before the schema check (which requires `source`) and the
    // merge.
    let leads: Vec<DiscoveryLead> = raw
        .into_iter()
        .map(|lead| DiscoveryLead {
            lead: lead.lead,
            source: source.to_string(),
            synopsis: lead.synopsis,
            topics: lead.topics,
        })
        .collect();
    schema::validate_leads(&leads)?;
    let lead_ids: Vec<String> = leads.iter().map(|lead| lead.lead.clone()).collect();

    let discovery_path = layout.discovery_path();
    let mut discovery = load_or_empty_discovery(&discovery_path)?;
    discovery.merge_survey(source, leads, &discovery_path)?;

    emit(
        layout,
        now,
        EventKind::SourceSurveyCompleted {
            source: source.to_string(),
            adapter: binding.adapter.clone(),
        },
    )?;
    Ok(SurveyedSource {
        source: source.to_string(),
        adapter: binding.adapter.clone(),
        leads: lead_ids,
    })
}

/// The result of a completed [`extract`]: the persisted Evidence path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractOutcome {
    /// Plan source binding key (`plan.yaml.sources.<key>`).
    pub source: String,
    /// Bound source adapter name.
    pub adapter: String,
    /// Persisted `.specify/slices/<slice>/evidence/<source>.yaml`.
    pub evidence: PathBuf,
}

/// Extract Evidence for one `(source, lead)` pair through the seam and
/// persist it into the slice's `evidence/` directory.
///
/// The guest collapse of `specify source extract`: resolves the
/// binding and the discovery lead, journals `source.execution.agent`,
/// dispatches `seam.extract`, composes the full Evidence document (the
/// envelope `lead` key rejoins the seam's lead-less answer), schema
/// gates it *before* it becomes visible to synthesis, persists it
/// atomically, and journals `slice.extract.completed`. A validation
/// failure returns early — no Evidence lands on the persisted path.
///
/// # Errors
///
/// - `source-unknown` when `source` is not a `plan.yaml.sources` key.
/// - `discovery-lead-unknown` when `(source, lead)` is not in
///   `discovery.md`.
/// - `seam-dispatch-failed` when the seam dispatch fails.
/// - propagates Evidence schema-validation and persist failures.
pub async fn extract(
    seam: &impl SourceSeam, layout: Layout<'_>, now: Timestamp, source: &str, lead: &str,
    slice: &str,
) -> Result<ExtractOutcome, Error> {
    let plan = Plan::load(&layout.plan_path())?;
    let binding = plan.sources.get(source).ok_or_else(|| Error::Diag {
        code: "source-unknown",
        detail: format!(
            "no source `{source}` in plan.yaml.sources; `specify source extract` resolves \
             its argument against the plan's source keys, not the adapter name"
        ),
    })?;
    let seam_lead = resolve_seam_lead(layout, source, lead)?;

    emit(
        layout,
        now,
        EventKind::SourceExecutionAgent {
            source: source.to_string(),
            adapter: binding.adapter.clone(),
            operation: SourceOperation::Extract,
        },
    )?;

    let id = source_adapter_id(&binding.adapter);
    let evidence = seam
        .extract(id.clone(), seam_lead)
        .await
        .map_err(|err| seam_failure("extract", &id, &err))?;

    let yaml = evidence_yaml(lead, evidence.authority, &evidence.claims)?;
    let path = layout.slices_dir().join(slice).join("evidence").join(format!("{source}.yaml"));
    schema::validate_evidence(&yaml, &path)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }
    bytes_write(&path, yaml.as_bytes())?;

    emit(
        layout,
        now,
        EventKind::SliceExtractCompleted {
            slice_name: slice.into(),
            source: source.to_string(),
        },
    )?;
    Ok(ExtractOutcome {
        source: source.to_string(),
        adapter: binding.adapter.clone(),
        evidence: path,
    })
}

/// Resolve `(source, lead)` from `discovery.md` into the seam's
/// lead record (the WIT shape drops the envelope `source` key).
fn resolve_seam_lead(
    layout: Layout<'_>, source: &str, lead: &str,
) -> Result<crate::seam::Lead, Error> {
    let discovery = Discovery::load(&layout.discovery_path())?;
    let resolved = discovery
        .leads()
        .iter()
        .find(|candidate| candidate.source == source && candidate.lead == lead)
        .ok_or_else(|| Error::Diag {
            code: "discovery-lead-unknown",
            detail: format!(
                "no lead `{lead}` for source `{source}` in discovery.md; extract resolves its \
                 lead against the surveyed inventory"
            ),
        })?;
    Ok(crate::seam::Lead {
        lead: resolved.lead.clone(),
        synopsis: resolved.synopsis.clone(),
        topics: resolved.topics.clone(),
    })
}

/// The full persisted Evidence document: the envelope `lead` key plus
/// the seam answer's `authority` and open-shaped `claims`.
#[derive(Serialize)]
struct EvidenceDocument<'a> {
    lead: &'a str,
    authority: AuthorityClass,
    claims: &'a [JsonValue],
}

/// Compose the persisted Evidence YAML with a trailing newline.
fn evidence_yaml(
    lead: &str, authority: AuthorityClass, claims: &[JsonValue],
) -> Result<String, Error> {
    let document = EvidenceDocument {
        lead,
        authority,
        claims,
    };
    let mut yaml = serde_saphyr::to_string(&document)?;
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    Ok(yaml)
}

/// Load `discovery.md`, or start from an empty document when the file
/// is absent so the first survey can author the inventory.
fn load_or_empty_discovery(path: &Path) -> Result<Discovery, Error> {
    if path.exists() { Discovery::load(path) } else { Discovery::parse("") }
}

/// Append one journal event, propagating the write failure — the
/// source-axis emits are fallible, mirroring the native verbs'
/// `append_batch` posture (unlike the best-effort build/merge
/// brackets).
fn emit(layout: Layout<'_>, now: Timestamp, kind: EventKind) -> Result<(), Error> {
    journal::append_batch(layout, std::slice::from_ref(&Event::new(now, kind)))
}
