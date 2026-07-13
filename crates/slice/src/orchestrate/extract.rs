//! Source-axis Evidence extraction for one `(source, lead)` pair.

use std::path::PathBuf;

use artifacts::atomic::bytes_write;
use artifacts::discovery::Discovery;
use artifacts::evidence::AuthorityClass;
use error::Error;
use jiff::Timestamp;
use project::adapter::SourceOperation;
use project::config::Layout;
use project::journal::{self, Event, EventKind};
use project::plan::Plan;
use project::schema_gate;
use project::seam::{SourceSeam, seam_failure, source_id};
use serde::Serialize;
use serde_json::Value as JsonValue;

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

/// Extract Evidence for one `(source, lead)` pair (`specify source
/// extract`) and persist it into the slice's `evidence/` directory.
///
/// The Evidence document is schema-gated *before* it becomes visible
/// to synthesis; a validation failure returns early with nothing on
/// the persisted path.
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

    let id = source_id(&binding.adapter);
    let evidence = seam
        .extract(id.clone(), seam_lead)
        .await
        .map_err(|err| seam_failure("extract", &id, &err))?;

    let yaml = evidence_yaml(lead, evidence.authority, &evidence.claims)?;
    let path = layout.slice_dir(slice).join("evidence").join(format!("{source}.yaml"));
    schema_gate::validate_evidence(&yaml, &path)?;
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
) -> Result<project::seam::Lead, Error> {
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
    Ok(project::seam::Lead {
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
    project::fs::yaml(&EvidenceDocument {
        lead,
        authority,
        claims,
    })
}

/// Append one journal event, propagating the write failure — the
/// source-axis emits are fallible (strict [`journal::append_one`]),
/// unlike the best-effort build/merge brackets.
fn emit(layout: Layout<'_>, now: Timestamp, kind: EventKind) -> Result<(), Error> {
    journal::append_one(layout, &Event::new(now, kind))
}
