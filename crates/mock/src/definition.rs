//! Host-only definition-home fixtures for RFC-88 / RFC-104 tests.
//!
//! Mints a minimal valid definition home plus `system.wave.reviewed`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use artifacts::evidence::{AuthorityClass, Document};
use error::Error;
use jiff::Timestamp;
use project::adapter::catalog::INTENT;
use project::journal::{
    DEFAULT_WRITER, Event, EventKind, JournalRoot, append_for_at, read_union_at,
};
use project::snapshot::SnapshotId;
use system::handoff::{self, Handoff, Projected};
use system::migration::{DeliveryMapping, EvidenceScope, TargetRef, Wave, WaveArchitecture};
use system::{Coverage, Disposition, Layout, Migration, Model, Row, Scope as SystemScope, State};

/// Inputs for one reviewed definition home.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    /// Definition identity (`scope.yaml.id`).
    pub definition: String,
    /// Wave id.
    pub wave: String,
    /// Wave outcome prose.
    pub outcome: String,
    /// Delivery targets.
    pub targets: Vec<Target>,
    /// Surface leads (degenerate always includes `intent`).
    pub scopes: Vec<Scope>,
    /// Source-to-target assignments.
    pub mappings: Vec<Mapping>,
}

/// One proposed delivery target (handoff `targets[]` shape).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Logical target id.
    pub id: String,
    /// Mutable origin locator.
    pub locator: String,
    /// Declared adapter identity (bare name or exact pin).
    pub adapter: String,
}

/// One selected surface lead (mint-local; projected into RFC-104 form).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// Coverage-row source key.
    pub source: String,
    /// Declared adapter identity (bare name or exact pin).
    pub adapter: String,
    /// Origin locator. Empty on a value scope — mint writes a file.
    pub location: String,
    /// Lead id within `source`.
    pub lead: String,
    /// Inline value written to a file at mint time (`intent`).
    pub value: Option<String>,
}

/// Reviewed source-to-target assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    /// Source key.
    pub source: String,
    /// Lead id within `source`.
    pub lead: String,
    /// Target id.
    pub target: String,
}

/// Current reviewed handoff projection (author-consume shape).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reviewed {
    /// Validated RFC-104 handoff body.
    pub handoff: Handoff,
    /// Canonical handoff digest (`sha256:<64 hex>`).
    pub digest: SnapshotId,
    /// Matching `system.wave.reviewed` envelope.
    pub review: Event,
    /// Canonical digest of [`Self::review`].
    pub event_digest: SnapshotId,
}

/// Result of [`mint`]: paths plus the identities later steps import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Minted {
    /// Definition-home root that was written.
    pub root: PathBuf,
    /// Canonical handoff digest.
    pub digest: SnapshotId,
    /// Wave id.
    pub wave: String,
    /// Resolved reviewed projection (self-check).
    pub reviewed: Reviewed,
}

impl Spec {
    /// One-target degenerate definition: `intent` coverage row whose
    /// location is a minted file carrying the inline value.
    #[must_use]
    pub fn degenerate(intent: &str) -> Self {
        Self {
            definition: "demo".into(),
            wave: "deliver".into(),
            outcome: "Deliver the reviewed intent".into(),
            targets: vec![Target {
                id: "app".into(),
                locator: "https://github.com/example/app".into(),
                adapter: "emery:omnia@1.0.0".into(),
            }],
            scopes: vec![Scope {
                source: INTENT.into(),
                adapter: "emery:intent@1.0.0".into(),
                location: String::new(),
                lead: INTENT.into(),
                value: Some(intent.into()),
            }],
            mappings: vec![Mapping {
                source: INTENT.into(),
                lead: INTENT.into(),
                target: "app".into(),
            }],
        }
    }

    /// Two-target fixture with location-backed evidence scopes.
    #[must_use]
    pub fn multi_target() -> Self {
        Self {
            definition: "orders".into(),
            wave: "extract-orders".into(),
            outcome: "Move order ownership behind the reviewed orders service".into(),
            targets: vec![
                Target {
                    id: "orders-service".into(),
                    locator: "https://github.com/example/orders-service".into(),
                    adapter: "emery:omnia@1.0.0".into(),
                },
                Target {
                    id: "payments-service".into(),
                    locator: "https://github.com/example/payments-service".into(),
                    adapter: "emery:omnia@1.0.0".into(),
                },
            ],
            scopes: vec![
                Scope {
                    source: "orders-code".into(),
                    adapter: "emery:typescript@1.0.0".into(),
                    location: "https://github.com/example/orders-service".into(),
                    lead: "orders-api".into(),
                    value: None,
                },
                Scope {
                    source: "payments-code".into(),
                    adapter: "emery:typescript@1.0.0".into(),
                    location: "https://github.com/example/payments-service".into(),
                    lead: "payments-api".into(),
                    value: None,
                },
            ],
            mappings: vec![
                Mapping {
                    source: "orders-code".into(),
                    lead: "orders-api".into(),
                    target: "orders-service".into(),
                },
                Mapping {
                    source: "payments-code".into(),
                    lead: "payments-api".into(),
                    target: "payments-service".into(),
                },
            ],
        }
    }
}

/// Inline-value evidence scope (`intent` or another value-backed source).
#[must_use]
pub fn value_scope(
    source: impl Into<String>, adapter: impl Into<String>, value: impl Into<String>,
    lead: impl Into<String>, _digit: u8,
) -> Scope {
    Scope {
        source: source.into(),
        adapter: adapter.into(),
        location: String::new(),
        lead: lead.into(),
        value: Some(value.into()),
    }
}

/// Location-backed evidence scope (ingest fetches at bind time).
#[must_use]
pub fn location_scope(
    source: impl Into<String>, adapter: impl Into<String>, locator: impl Into<String>,
    lead: impl Into<String>, _digit: u8,
) -> Scope {
    Scope {
        source: source.into(),
        adapter: adapter.into(),
        location: locator.into(),
        lead: lead.into(),
        value: None,
    }
}

/// Write a reviewed RFC-104 definition home at `root` and resolve it back.
///
/// # Errors
///
/// Projection, filesystem, or review-append failures.
pub fn mint(root: &Path, spec: &Spec) -> Result<Minted, Error> {
    std::fs::create_dir_all(root)?;
    let layout = Layout::new(root);
    let scopes = materialize_values(root, &spec.scopes)?;
    write_declared(&layout, spec, &scopes)?;
    write_evidence(&layout, &scopes)?;

    let scope = SystemScope::load(&layout.scope_path())?;
    let coverage = Coverage::load(&layout.coverage_path())?;
    let model = Model::load(&layout.system_path())?;
    let migration = Migration::load(&layout.migration_path())?;
    let wave = migration.wave(&spec.wave).ok_or_else(|| Error::Diag {
        code: "definition-fixture-wave",
        detail: format!("minted migration has no wave `{}`", spec.wave),
    })?;
    let projected = handoff::project(&layout, &scope, &coverage, &model, &migration, &[], wave)?;
    handoff::write(&layout, &projected)?;
    append_review(&layout, spec, &projected)?;
    let reviewed = load_reviewed(root, &spec.wave)?;
    Ok(Minted {
        root: root.to_path_buf(),
        digest: projected.digest,
        wave: spec.wave.clone(),
        reviewed,
    })
}

/// Load the current reviewed handoff the same way `plan author` does.
///
/// # Errors
///
/// `current_handoff` failures, `system-handoff-corrupt`, or a missing
/// matching `system.wave.reviewed` fact.
pub fn load_reviewed(root: &Path, wave: &str) -> Result<Reviewed, Error> {
    let layout = Layout::new(root);
    let projected = system::review::current_handoff(&layout, wave)?;
    let verified = handoff::load(&layout.handoff_path(projected.digest.digest()))?;
    let review = matching_review(&layout, wave, &verified.digest)?;
    let event_digest = review.digest()?;
    Ok(Reviewed {
        handoff: verified.handoff,
        digest: verified.digest,
        review,
        event_digest,
    })
}

fn materialize_values(root: &Path, scopes: &[Scope]) -> Result<Vec<Scope>, Error> {
    let mut out = Vec::with_capacity(scopes.len());
    for scope in scopes {
        let mut scope = scope.clone();
        if let Some(value) = &scope.value {
            let path = if scope.location.is_empty() {
                root.join(format!("{}.txt", scope.source))
            } else if Path::new(&scope.location).is_absolute() {
                PathBuf::from(&scope.location)
            } else {
                root.join(&scope.location)
            };
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, value)?;
            scope.location = path.display().to_string();
        }
        if scope.location.trim().is_empty() {
            return Err(Error::Diag {
                code: "definition-fixture-location",
                detail: format!("scope `{}` needs a location or a value", scope.source),
            });
        }
        out.push(scope);
    }
    Ok(out)
}

fn write_declared(layout: &Layout<'_>, spec: &Spec, scopes: &[Scope]) -> Result<(), Error> {
    let scope = SystemScope {
        version: 1,
        id: spec.definition.clone(),
        decision: spec.outcome.clone(),
        products: Vec::new(),
        journeys: Vec::new(),
        environments: Vec::new(),
        organizations: Vec::new(),
    };
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();
    for row in scopes {
        if !seen.insert(row.source.clone()) {
            continue;
        }
        candidates.push(Row {
            key: row.source.clone(),
            location: row.location.clone(),
            adapter: Some(row.adapter.clone()),
            disposition: Disposition::Included,
            reason: "included".into(),
            observed_cid: Some(pin(0x1)),
            observed_revision: None,
            survey_error: None,
        });
    }
    let coverage = Coverage {
        version: 1,
        candidates,
    };
    let model = Model {
        version: 1,
        identities: Vec::new(),
        as_is: State::default(),
        target: Some(State::default()),
        transitions: std::collections::BTreeMap::new(),
    };
    let wave = Wave {
        id: spec.wave.clone(),
        outcome: spec.outcome.clone(),
        architecture: WaveArchitecture {
            before: "as-is".into(),
            after: "target".into(),
        },
        predecessors: Vec::new(),
        preconditions: Vec::new(),
        affected_elements: Vec::new(),
        touched_elements: Vec::new(),
        context_elements: Vec::new(),
        dispositions: Vec::new(),
        evidence_scopes: scopes
            .iter()
            .map(|scope| EvidenceScope {
                source: scope.source.clone(),
                lead: scope.lead.clone(),
            })
            .collect(),
        targets: spec
            .targets
            .iter()
            .map(|target| TargetRef {
                id: target.id.clone(),
                locator: target.locator.clone(),
                adapter: target.adapter.clone(),
            })
            .collect(),
        delivery_mappings: spec
            .mappings
            .iter()
            .map(|mapping| DeliveryMapping {
                source: mapping.source.clone(),
                lead: mapping.lead.clone(),
                target: mapping.target.clone(),
            })
            .collect(),
        state_movements: Vec::new(),
        coexistence: Vec::new(),
        cutover: Vec::new(),
        rollback: Vec::new(),
        operational_readiness: Vec::new(),
        acceptance: Vec::new(),
        verification: Vec::new(),
        conservation: Vec::new(),
        gaps: Vec::new(),
        assumptions: Vec::new(),
        decisions: Vec::new(),
    };
    let migration = Migration {
        version: 1,
        dispositions: Vec::new(),
        waves: vec![wave],
    };
    artifacts::atomic::yaml_write(&layout.scope_path(), &scope)?;
    artifacts::atomic::yaml_write(&layout.coverage_path(), &coverage)?;
    artifacts::atomic::yaml_write(&layout.system_path(), &model)?;
    artifacts::atomic::yaml_write(&layout.migration_path(), &migration)?;
    Ok(())
}

fn write_evidence(layout: &Layout<'_>, scopes: &[Scope]) -> Result<(), Error> {
    for scope in scopes {
        std::fs::create_dir_all(layout.source_evidence_dir(&scope.source))?;
        let document = Document {
            lead: scope.lead.clone(),
            authority: if scope.source == INTENT {
                AuthorityClass::Intent
            } else {
                AuthorityClass::Documentation
            },
            claims: Vec::new(),
        };
        artifacts::atomic::yaml_write(
            &layout.evidence_path(&scope.source, &scope.lead),
            &document,
        )?;
    }
    Ok(())
}

fn append_review(layout: &Layout<'_>, spec: &Spec, projected: &Projected) -> Result<(), Error> {
    let event = Event::new(
        Timestamp::from_second(1_700_000_000).map_err(|err| Error::Diag {
            code: "definition-fixture-time",
            detail: format!("pinned fixture timestamp is out of range: {err}"),
        })?,
        EventKind::SystemWaveReviewed {
            wave: spec.wave.clone(),
            handoff_digest: projected.digest.clone(),
        },
    );
    append_for_at(
        &JournalRoot::new(layout.events_dir()),
        DEFAULT_WRITER,
        std::slice::from_ref(&event),
    )
}

fn matching_review(layout: &Layout<'_>, wave: &str, digest: &SnapshotId) -> Result<Event, Error> {
    read_union_at(&JournalRoot::new(layout.events_dir()))?
        .into_iter()
        .find(|event| {
            matches!(
                &event.kind,
                EventKind::SystemWaveReviewed { wave: reviewed, handoff_digest }
                    if reviewed == wave && handoff_digest == digest
            )
        })
        .ok_or_else(|| Error::Diag {
            code: "definition-review-missing",
            detail: format!("no system.wave.reviewed fact names handoff `{digest}`"),
        })
}

fn pin(digit: u8) -> SnapshotId {
    SnapshotId::from_digest(&format!("{digit:x}").repeat(64))
}
