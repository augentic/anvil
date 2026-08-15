//! `emery system status` — the read-only definition-home projection:
//! coverage accounting, model states, migration waves with their
//! review standing, and the computed next action. Never a writer.

use error::Error;
use project::journal::{self, EventKind, JournalRoot};

use crate::coverage::{Coverage, Disposition};
use crate::layout::Layout;
use crate::migration::Migration;
use crate::model::Model;
use crate::scope::Scope;
use crate::{handoff, review};

/// The projected definition-home status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// The declared engagement identity (`scope.yaml.id`).
    pub id: String,
    /// The decision the definition supports (`scope.yaml.decision`).
    pub decision: String,
    /// Per-disposition coverage accounting.
    pub coverage: CoverageCounts,
    /// Included sources whose last run failed, in operator order.
    pub failed_sources: Vec<String>,
    /// Named states with their sizes; empty until survey correlates.
    pub states: Vec<StateRow>,
    /// Migration waves with their review standing, in plan order;
    /// empty until a plan exists.
    pub waves: Vec<WaveRow>,
    /// The computed next operator action.
    pub next: NextAction,
}

/// Per-disposition coverage row counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CoverageCounts {
    /// Rows with `disposition: included`.
    pub included: usize,
    /// Rows with any other disposition (accounted, never surveyed).
    pub other: usize,
}

/// One named state's accounting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRow {
    /// The state's name (`as-is`, `target`, `transition-*`).
    pub name: String,
    /// Elements in the state.
    pub elements: usize,
    /// Relationships in the state.
    pub relationships: usize,
}

/// One migration wave's review standing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaveRow {
    /// The wave's id in `migration.yaml`.
    pub wave: String,
    /// The wave's current standing.
    pub standing: WaveStanding,
}

/// A wave's standing against the live definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaveStanding {
    /// A current handoff exists and carries a `system.wave.reviewed`
    /// fact — architectural authority is recorded.
    Reviewed {
        /// The reviewed handoff digest (`sha256:…`).
        handoff_digest: String,
    },
    /// A current handoff exists but no review fact covers it.
    AwaitingReview {
        /// The current handoff digest (`sha256:…`).
        handoff_digest: String,
    },
    /// No handoff matches the live files — re-run `emery system plan`.
    Stale,
}

/// The computed next operator action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextAction {
    /// No `system.yaml` yet: run `emery system survey`.
    Survey,
    /// No `target` state yet: run `emery system plan`.
    Plan,
    /// A wave's current handoff awaits review.
    Review {
        /// The wave to review.
        wave: String,
    },
    /// A wave's handoff is stale: re-run `emery system plan`.
    Replan {
        /// The stale wave.
        wave: String,
    },
    /// Every wave's current handoff is reviewed.
    Reviewed,
}

/// Project the definition home's status.
///
/// # Errors
///
/// Fail-closed loads of `scope.yaml` and `coverage.yaml` (the two
/// operator-declared files); `system.yaml` and `migration.yaml` are
/// optional at this stage and project as absent.
pub fn project(layout: &Layout<'_>) -> Result<Status, Error> {
    let scope = Scope::load(&layout.scope_path())?;
    let coverage = Coverage::load(&layout.coverage_path())?;
    let mut counts = CoverageCounts::default();
    let mut failed_sources = Vec::new();
    for row in &coverage.candidates {
        if row.disposition == Disposition::Included {
            counts.included += 1;
            if row.survey_error.is_some() {
                failed_sources.push(row.key.clone());
            }
        } else {
            counts.other += 1;
        }
    }

    let model = match Model::load(&layout.system_path()) {
        Ok(model) => Some(model),
        Err(Error::Diag {
            code: "system-model-missing",
            ..
        }) => None,
        Err(err) => return Err(err),
    };
    let mut states = Vec::new();
    if let Some(model) = &model {
        states.push(state_row("as-is", &model.as_is));
        if let Some(target) = &model.target {
            states.push(state_row("target", target));
        }
        for (name, state) in &model.transitions {
            states.push(state_row(name, state));
        }
    }

    let migration = match Migration::load(&layout.migration_path()) {
        Ok(migration) => Some(migration),
        Err(Error::Diag {
            code: "system-migration-missing",
            ..
        }) => None,
        Err(err) => return Err(err),
    };
    let reviewed = reviewed_facts(layout)?;
    let mut waves = Vec::new();
    if let Some(migration) = &migration {
        // One live-digest pass and one handoff load serve every wave.
        let live = review::Live::load(layout)?;
        let handoffs = handoff::load_all(layout)?;
        for wave in &migration.waves {
            let standing = match review::select(&live, &handoffs, &wave.id) {
                Ok(current) => {
                    let digest = current.digest.as_str().to_string();
                    if reviewed.contains(&(wave.id.clone(), digest.clone())) {
                        WaveStanding::Reviewed {
                            handoff_digest: digest,
                        }
                    } else {
                        WaveStanding::AwaitingReview {
                            handoff_digest: digest,
                        }
                    }
                }
                Err(Error::Diag {
                    code: "system-review-handoff-stale",
                    ..
                }) => WaveStanding::Stale,
                Err(err) => return Err(err),
            };
            waves.push(WaveRow {
                wave: wave.id.clone(),
                standing,
            });
        }
    }

    let next = next_action(model.as_ref(), &waves);
    Ok(Status {
        id: scope.id,
        decision: scope.decision,
        coverage: counts,
        failed_sources,
        states,
        waves,
        next,
    })
}

/// Every `(wave, handoff-digest)` a `system.wave.reviewed` fact covers.
fn reviewed_facts(layout: &Layout<'_>) -> Result<Vec<(String, String)>, Error> {
    let root = JournalRoot::new(layout.events_dir());
    Ok(journal::read_union_at(&root)?
        .into_iter()
        .filter_map(|event| match event.kind {
            EventKind::SystemWaveReviewed { wave, handoff_digest } => {
                Some((wave, handoff_digest.as_str().to_string()))
            }
            _other => None,
        })
        .collect())
}

fn state_row(name: &str, state: &crate::model::State) -> StateRow {
    StateRow {
        name: name.to_string(),
        elements: state.elements.len(),
        relationships: state.relationships.len(),
    }
}

/// The stage ladder: survey → plan → review each wave → reviewed.
fn next_action(model: Option<&Model>, waves: &[WaveRow]) -> NextAction {
    let Some(model) = model else {
        return NextAction::Survey;
    };
    if model.target.is_none() {
        return NextAction::Plan;
    }
    for row in waves {
        match &row.standing {
            WaveStanding::AwaitingReview { .. } => {
                return NextAction::Review {
                    wave: row.wave.clone(),
                };
            }
            WaveStanding::Stale => {
                return NextAction::Replan {
                    wave: row.wave.clone(),
                };
            }
            WaveStanding::Reviewed { .. } => {}
        }
    }
    NextAction::Reviewed
}
