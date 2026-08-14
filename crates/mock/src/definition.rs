//! Host-only definition-home fixtures for RFC-88 / RFC-104 tests.
//!
//! Mints a reviewed handoff plus `system.wave.reviewed` envelope so CI
//! never depends on live `emery system` orchestrations.

use std::path::{Path, PathBuf};

use error::Error;
use jiff::Timestamp;
use project::definition::{
    Architecture, Handoff, Home, INTENT, Mapping, Ref, Reviewed, Scope, Target, VERSION, Wave,
};
use project::journal::{DEFAULT_WRITER, Event, EventKind};
use project::snapshot::SnapshotId;

/// Inputs for one reviewed definition home.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    /// Definition identity.
    pub definition: String,
    /// Wave id.
    pub wave: String,
    /// Wave outcome prose.
    pub outcome: String,
    /// Delivery targets.
    pub targets: Vec<Target>,
    /// Evidence scopes (degenerate always includes `intent`).
    pub scopes: Vec<Scope>,
    /// Source-to-target assignments.
    pub mappings: Vec<Mapping>,
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
    /// One-target degenerate definition: inline `intent` evidence scope,
    /// no locator, no CID.
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
                source_cid: None,
                value: Some(intent.into()),
                adapter: Some("emery:intent@1.0.0".into()),
                locator: None,
                lead: INTENT.into(),
                evidence_digest: pin(0xe),
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
                    source_cid: Some(pin(0x1)),
                    value: None,
                    adapter: Some("emery:typescript@1.0.0".into()),
                    locator: None,
                    lead: "orders-api".into(),
                    evidence_digest: pin(0x2),
                },
                Scope {
                    source: "payments-code".into(),
                    source_cid: Some(pin(0x3)),
                    value: None,
                    adapter: Some("emery:typescript@1.0.0".into()),
                    locator: None,
                    lead: "payments-api".into(),
                    evidence_digest: pin(0x4),
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
    lead: impl Into<String>, digit: u8,
) -> Scope {
    Scope {
        source: source.into(),
        source_cid: None,
        value: Some(value.into()),
        adapter: Some(adapter.into()),
        locator: None,
        lead: lead.into(),
        evidence_digest: pin(digit),
    }
}

/// Location-backed evidence scope (CID placeholder; ingest fetches).
#[must_use]
pub fn location_scope(
    source: impl Into<String>, adapter: impl Into<String>, locator: impl Into<String>,
    lead: impl Into<String>, digit: u8,
) -> Scope {
    Scope {
        source: source.into(),
        source_cid: Some(pin(digit & 0xf)),
        value: None,
        adapter: Some(adapter.into()),
        locator: Some(locator.into()),
        lead: lead.into(),
        evidence_digest: pin(digit.wrapping_add(1) & 0xf),
    }
}

impl From<&Spec> for Handoff {
    fn from(spec: &Spec) -> Self {
        Self {
            version: VERSION,
            definition: spec.definition.clone(),
            scope_digest: pin(0x0),
            coverage_digest: pin(0xc),
            sources_digest: pin(0xd),
            system_model_digest: pin(0xe),
            migration_plan_digest: pin(0xf),
            wave: Wave {
                id: spec.wave.clone(),
                digest: pin(0x5),
                outcome: spec.outcome.clone(),
                architecture: Architecture {
                    before: Ref {
                        id: "as-is".into(),
                        digest: pin(0xb),
                    },
                    after: Ref {
                        id: "target".into(),
                        digest: pin(0xa),
                    },
                },
                targets: spec.targets.clone(),
                evidence_scopes: spec.scopes.clone(),
                delivery_mappings: spec.mappings.clone(),
                affected_elements: Vec::new(),
                touched_elements: Vec::new(),
                context_elements: Vec::new(),
                dependencies: Vec::new(),
                preconditions: Vec::new(),
                dispositions: Vec::new(),
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
            },
        }
    }
}

/// Write a reviewed definition home at `root` and resolve it back.
///
/// # Errors
///
/// Handoff validation, filesystem, or resolve failures.
pub fn mint(root: &Path, spec: &Spec) -> Result<Minted, Error> {
    let handoff = Handoff::from(spec);
    handoff.validate()?;
    let digest = handoff.digest()?;
    let home = Home::new(root);
    std::fs::create_dir_all(home.handoffs_dir())?;
    std::fs::create_dir_all(home.events_dir())?;
    artifacts::atomic::yaml_write(&home.handoff_path(&digest), &handoff)?;
    let review = Event {
        timestamp: Timestamp::from_second(1_700_000_000).map_err(|err| Error::Diag {
            code: "definition-fixture-time",
            detail: format!("pinned fixture timestamp is out of range: {err}"),
        })?,
        writer: DEFAULT_WRITER.into(),
        sequence: 1,
        kind: EventKind::SystemWaveReviewed {
            wave: spec.wave.clone(),
            handoff_digest: digest.clone(),
        },
    };
    let line = serde_json::to_string(&review).map_err(|err| Error::Diag {
        code: "journal-event-serialise-failed",
        detail: format!("failed to serialise journal event: {err}"),
    })?;
    std::fs::write(home.events_dir().join(format!("{DEFAULT_WRITER}.jsonl")), format!("{line}\n"))?;
    let reviewed = project::definition::resolve(root, &spec.wave)?;
    Ok(Minted {
        root: root.to_path_buf(),
        digest,
        wave: spec.wave.clone(),
        reviewed,
    })
}

fn pin(digit: u8) -> SnapshotId {
    SnapshotId::from_digest(&format!("{digit:x}").repeat(64))
}
