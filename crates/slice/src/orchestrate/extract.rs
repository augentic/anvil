//! Source-axis Evidence extraction for one `(source, lead)` pair.

use std::path::PathBuf;

use artifacts::atomic::bytes_write;
use artifacts::discovery::Discovery;
use error::Error;
use jiff::Timestamp;
use project::adapter::{Resolver, SourceOperation};
use project::config::Layout;
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind};
use project::plan::Plan;
use project::seam::{Source, seam_failure, source_id};

/// The result of a completed [`extract`]: the persisted Evidence path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractOutcome {
    /// Plan source binding key (`plan.yaml.sources.<key>`).
    pub source: String,
    /// Bound source adapter name.
    pub adapter: String,
    /// Persisted `.emery/slices/<slice>/evidence/<source>.yaml`.
    pub evidence: PathBuf,
}

/// Extract Evidence for one `(source, lead)` pair (`emery source
/// extract`) and persist it into the slice's `evidence/` directory.
///
/// The typed Evidence document is deterministically validated
/// ([`artifacts::evidence::Document::validate`]) *before* it becomes
/// visible to synthesis; a validation failure returns early with
/// nothing on the persisted path.
///
/// # Errors
///
/// `plan-source-unknown` for an unbound source key, adapter
/// ensure/resolve failures (missing pin, `emery_floor`), seam and
/// `evidence-schema` validation failures from the adapter's extract
/// leg, plus plan-load and persistence I/O failures.
#[tracing::instrument(
    name = "source.extract",
    skip_all,
    fields(source = %source, slice = %slice, adapter = tracing::field::Empty)
)]
pub async fn extract(
    seam: &impl Source, resolver: &impl Resolver, paths: &ExecutionPaths, now: Timestamp,
    source: &str, lead: &str, slice: &str,
) -> Result<ExtractOutcome, Error> {
    let layout = Layout::new(paths.project_root());
    let plan = Plan::load(&layout.plan_path())?;
    let binding = plan
        .sources
        .get(source)
        .ok_or_else(|| plan.source_not_found("emery source extract", source))?;
    let seam_lead = resolve_seam_lead(layout, source, lead)?;

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
            operation: SourceOperation::Extract,
        },
    )?;

    let id = source_id(&adapter);
    let evidence = seam
        .extract(id.clone(), seam_lead)
        .await
        .map_err(|err| seam_failure("extract", &id, &err))?;

    let document = artifacts::evidence::Document {
        lead: lead.to_string(),
        authority: evidence.authority,
        claims: evidence.claims,
    };
    document.validate()?;
    let yaml = project::fs::yaml(&document)?;
    let path = layout.slice_dir(slice).join("evidence").join(format!("{source}.yaml"));
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
        adapter: adapter.name,
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

/// Append one journal event, propagating the write failure — the
/// source-axis emits are fallible (strict [`journal::append_one`]),
/// unlike the best-effort build/merge brackets.
fn emit(layout: Layout<'_>, now: Timestamp, kind: EventKind) -> Result<(), Error> {
    journal::append_one(layout, &Event::new(now, kind))
}
