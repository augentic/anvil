//! The `emery system survey` orchestration (RFC-104 D2/D3):
//! materialize → survey → lead gate → extract → surgical coverage
//! persist. Failures are coverage accounting, never lost rows.

use std::collections::BTreeMap;
use std::path::Path;

use artifacts::atomic::bytes_write;
use artifacts::discovery::{Lead as DiscoveryLead, validate_leads};
use error::Error;
use project::adapter::{AdapterSelector, Resolver};
use project::handler::ExecutionPaths;
use project::seam::{Origins, Source, SourceInput, Workspaces, source_id};
use project::snapshot::SnapshotId;

use crate::coverage::{self, Coverage, Row, RowPatch, SurveyError, SurveyErrorKind};
use crate::layout::Layout;
use crate::scope::Scope;
use crate::{MAX_SURVEY_LEADS, materialize};

/// The completed run's accounting, projected by the operation body.
#[derive(Debug)]
pub struct SurveyOutcome {
    /// The declared engagement identity (`scope.yaml.id`).
    pub id: String,
    /// The decision the survey supports (`scope.yaml.decision`).
    pub decision: String,
    /// Declared candidate count across every disposition.
    pub candidates: usize,
    /// Per-included-source accounting, in operator order.
    pub sources: Vec<SourceReport>,
    /// Evidence documents persisted this run.
    pub evidence: usize,
}

/// One included source's run accounting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceReport {
    /// The source completed survey and extract; its coverage row now
    /// carries the observed tree.
    Surveyed {
        /// Coverage-row source key.
        source: String,
        /// The bound adapter that answered.
        adapter: String,
        /// Leads the adapter surfaced (all extracted).
        leads: usize,
        /// RFC-87 identity of the observed tree.
        cid: SnapshotId,
        /// Git commit when the origin reported one.
        revision: Option<String>,
    },
    /// The source failed; its coverage row carries `survey-error` and
    /// keeps any prior observed tree.
    Failed {
        /// Coverage-row source key.
        source: String,
        /// The recorded failure.
        error: SurveyError,
    },
}

/// One included source that completed its survey leg: the lent
/// workspace stays open until its extract leg settles.
struct Surveyed {
    key: String,
    adapter: String,
    id: String,
    input: SourceInput,
    workspace: String,
    cid: SnapshotId,
    revision: Option<String>,
    leads: Vec<project::seam::Lead>,
}

/// Run the coverage-accounted definition survey over the anchored
/// definition home (RFC-104 D2/D3).
///
/// Every included row is materialized and surveyed; the summed lead
/// count gates before any extract; each fully extracted source's
/// Evidence replaces `evidence/<source>/` and its row gains the
/// observed tree. A failed row records `survey-error` and the run
/// continues — coverage accounting never drops a candidate.
///
/// # Errors
///
/// Fail-closed loads (`system-scope-missing`,
/// `system-coverage-missing`, validation), the typed
/// `system-survey-lead-limit` stop (no extract; the run's coverage
/// accounting still persists), and coverage-persist I/O failures.
pub async fn survey(
    seam: &(impl Source + Workspaces + Origins), resolver: &impl Resolver, paths: &ExecutionPaths,
) -> Result<SurveyOutcome, Error> {
    let root = paths.project_root();
    let layout = Layout::new(root);
    let scope = Scope::load(&layout.scope_path())?;
    let coverage = Coverage::load(&layout.coverage_path())?;

    // Survey phase: materialize and survey every included row,
    // holding each lent workspace for the extract phase.
    let mut completed = Vec::new();
    let mut patches: BTreeMap<String, RowPatch> = BTreeMap::new();
    let mut failures = Vec::new();
    for row in coverage.included() {
        match survey_source(seam, resolver, paths, root, row).await {
            Ok(surveyed) => completed.push(surveyed),
            Err(error) => {
                patches.insert(row.key.clone(), RowPatch::Failed(error.clone()));
                failures.push(SourceReport::Failed {
                    source: row.key.clone(),
                    error,
                });
            }
        }
    }

    // The lead gate runs before any extract: exceeding the engine
    // constant is a typed stop, and surveyed rows keep their observed
    // trees (the survey leg completed; the gate is extract-side).
    let total: usize = completed.iter().map(|source| source.leads.len()).sum();
    if total > MAX_SURVEY_LEADS {
        for source in completed {
            patches.insert(
                source.key,
                RowPatch::Observed {
                    cid: source.cid,
                    revision: source.revision,
                },
            );
            let _dropped = seam.discard(source.workspace).await;
        }
        coverage::persist(&layout.coverage_path(), &patches)?;
        return Err(Error::Diag {
            code: "system-survey-lead-limit",
            detail: format!(
                "survey produced {total} leads across the included sources, over the engine \
                 ceiling of {MAX_SURVEY_LEADS}; narrow coverage (exclude or unresolve rows) or \
                 author another definition home for a narrower decision, then re-run"
            ),
        });
    }

    // Extract phase: a source's Evidence replaces `evidence/<source>/`
    // only after every lead extracted, so a failure preserves the
    // prior corpus alongside its `survey-error`.
    let mut reports = Vec::new();
    let mut persisted = 0_usize;
    for source in completed {
        let extracted = extract_source(seam, &source).await;
        let _dropped = seam.discard(source.workspace).await;
        match extracted {
            Ok(documents) => {
                persisted += documents.len();
                replace_evidence(&layout, &source.key, &documents)?;
                patches.insert(
                    source.key.clone(),
                    RowPatch::Observed {
                        cid: source.cid.clone(),
                        revision: source.revision.clone(),
                    },
                );
                reports.push(SourceReport::Surveyed {
                    source: source.key,
                    adapter: source.adapter,
                    leads: source.leads.len(),
                    cid: source.cid,
                    revision: source.revision,
                });
            }
            Err(error) => {
                patches.insert(source.key.clone(), RowPatch::Failed(error.clone()));
                reports.push(SourceReport::Failed {
                    source: source.key,
                    error,
                });
            }
        }
    }
    reports.extend(failures);

    coverage::persist(&layout.coverage_path(), &patches)?;
    // First successful survey grows the generated layout; `as-is` and
    // `architecture/` arrive with correlation.
    std::fs::create_dir_all(layout.events_dir()).map_err(Error::Io)?;

    Ok(SurveyOutcome {
        id: scope.id,
        decision: scope.decision,
        candidates: coverage.candidates.len(),
        sources: reports,
        evidence: persisted,
    })
}

/// One row's survey leg: resolve the declared adapter, materialize
/// the location, dispatch, and validate the lead set.
async fn survey_source(
    seam: &(impl Source + Workspaces + Origins), resolver: &impl Resolver, paths: &ExecutionPaths,
    home: &Path, row: &Row,
) -> Result<Surveyed, SurveyError> {
    let declared = row.adapter.as_deref().unwrap_or_default();
    let selector = AdapterSelector::parse(declared).map_err(|err| adapter_failed(&err))?;
    let adapter =
        resolver.ensure_source(&selector, paths).await.map_err(|err| adapter_failed(&err))?;
    let adapter = adapter.manifest;

    let observed =
        materialize::materialize(seam, seam, home, &row.location).await.map_err(|err| {
            SurveyError {
                kind: SurveyErrorKind::Access,
                detail: err.to_string(),
            }
        })?;

    let id = source_id(&adapter);
    let input = SourceInput::Workspace(observed.workspace.root.clone());
    let raw = seam.survey(id.clone(), row.key.clone(), input.clone()).await;
    let raw = match raw {
        Ok(raw) => raw,
        Err(err) => {
            let _dropped = seam.discard(observed.workspace.id).await;
            return Err(SurveyError {
                kind: SurveyErrorKind::Adapter,
                detail: format!("survey via `{id}` failed: {err}"),
            });
        }
    };

    // The lead grammar gate mirrors the live survey path: an invalid
    // lead set never reaches extract or the evidence tree.
    let stamped: Vec<DiscoveryLead> = raw
        .iter()
        .map(|lead| DiscoveryLead {
            lead: lead.lead.clone(),
            source: row.key.clone(),
            synopsis: lead.synopsis.clone(),
            topics: lead.topics.clone(),
        })
        .collect();
    if let Err(err) = validate_leads(&stamped) {
        let _dropped = seam.discard(observed.workspace.id).await;
        return Err(adapter_failed(&err));
    }

    Ok(Surveyed {
        key: row.key.clone(),
        adapter: adapter.name,
        id,
        input,
        workspace: observed.workspace.id,
        cid: observed.cid,
        revision: observed.revision,
        leads: raw,
    })
}

/// One source's extract leg: every surveyed lead becomes a validated
/// Evidence document, collected before anything touches disk.
async fn extract_source(
    seam: &impl Source, source: &Surveyed,
) -> Result<Vec<(String, artifacts::evidence::Document)>, SurveyError> {
    let mut documents = Vec::with_capacity(source.leads.len());
    for lead in &source.leads {
        let evidence = seam
            .extract(source.id.clone(), source.key.clone(), source.input.clone(), lead.clone())
            .await
            .map_err(|err| SurveyError {
                kind: SurveyErrorKind::Adapter,
                detail: format!("extract of `{}` via `{}` failed: {err}", lead.lead, source.id),
            })?;
        let document = artifacts::evidence::Document {
            lead: lead.lead.clone(),
            authority: evidence.authority,
            claims: evidence.claims,
        };
        document.validate().map_err(|err| adapter_failed(&err))?;
        documents.push((lead.lead.clone(), document));
    }
    Ok(documents)
}

/// Overwrite `evidence/<source>/` with this run's documents — the
/// per-source half of "survey overwrites `evidence/`".
fn replace_evidence(
    layout: &Layout<'_>, source: &str, documents: &[(String, artifacts::evidence::Document)],
) -> Result<(), Error> {
    let dir = layout.source_evidence_dir(source);
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(Error::Io(err)),
    }
    std::fs::create_dir_all(&dir).map_err(Error::Io)?;
    for (lead, document) in documents {
        let yaml = project::fs::yaml(document)?;
        bytes_write(&layout.evidence_path(source, lead), yaml.as_bytes())?;
    }
    Ok(())
}

/// An adapter-leg failure record (resolve, dispatch, or validation).
fn adapter_failed(err: &dyn std::fmt::Display) -> SurveyError {
    SurveyError {
        kind: SurveyErrorKind::Adapter,
        detail: err.to_string(),
    }
}
