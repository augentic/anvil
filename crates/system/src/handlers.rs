//! The `emery system *` operations. A definition home has no
//! `project.yaml`, so handlers anchor at the provider's raw
//! [`Anchor::project_root`] instead of a project `Ctx`.

use std::io::Write;

use omnia_guest::Model;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::Resolver;
use project::handler::{Anchor, Render};
use project::seam::{Source, Trees, Workspaces};
use serde::{Deserialize, Serialize};

use crate::coverage::{SurveyError, SurveyErrorKind};
use crate::layout::Layout;
use crate::orchestrate::plan::{PlanOutcome, WaveHandoff};
use crate::orchestrate::{self, Correlated, SourceReport, SurveyOutcome};
use crate::review::ReviewOutcome;
use crate::status::{NextAction, WaveStanding};
use crate::{review, status};

/// Wire input for `emery system survey`.
///
/// The `--dir` argument never reaches this input: it is consumed by
/// the deployment's anchoring (the launcher mounts it as the
/// invocation's `.`), so the operation reads the anchored root.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct SurveyInput {}

/// `emery system survey` — the coverage-accounted definition survey
/// over the anchored definition home.
#[derive(Clone, Copy, Debug)]
pub struct Survey;

impl<P: Anchor + Source + Resolver + Workspaces + Trees + Model> Operation<P> for Survey {
    type Error = project::handler::Error;
    type Input = SurveyInput;
    type Output = SurveyBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let SurveyInput {} = input;
        let outcome =
            orchestrate::survey(context.provider, context.provider, context.provider.paths())
                .await?;
        Ok(SurveyBody::from(outcome))
    }
}

/// Success envelope for `emery system survey`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SurveyBody {
    /// The declared engagement identity (`scope.yaml.id`).
    pub id: String,
    /// The decision the survey supports (`scope.yaml.decision`).
    pub decision: String,
    /// Declared candidate count across every disposition.
    pub candidates: usize,
    /// Per-included-source run accounting, in operator order.
    pub sources: Vec<SourceBody>,
    /// Evidence documents persisted this run.
    pub evidence: usize,
    /// The persisted `as-is` state's accounting.
    pub as_is: AsIsBody,
}

/// The correlated `as-is` state's wire accounting.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AsIsBody {
    /// Elements in the persisted state.
    pub elements: usize,
    /// Relationships in the persisted state.
    pub relationships: usize,
    /// Claims across the included Evidence corpus.
    pub claims: usize,
}

/// One included source's wire accounting.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum SourceBody {
    /// The source completed survey and extract.
    #[serde(rename_all = "kebab-case")]
    Surveyed {
        /// Coverage-row source key.
        source: String,
        /// The bound adapter that answered.
        adapter: String,
        /// Leads the adapter surfaced (all extracted).
        leads: usize,
        /// RFC-87 identity of the observed tree.
        observed_cid: String,
        /// Git commit when the origin reported one.
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_revision: Option<String>,
    },
    /// The source failed; its row carries `survey-error`.
    #[serde(rename_all = "kebab-case")]
    Failed {
        /// Coverage-row source key.
        source: String,
        /// Which leg failed.
        kind: SurveyErrorKind,
        /// Human-readable failure detail.
        detail: String,
    },
}

impl From<SurveyOutcome> for SurveyBody {
    fn from(outcome: SurveyOutcome) -> Self {
        let sources = outcome
            .sources
            .into_iter()
            .map(|report| match report {
                SourceReport::Surveyed {
                    source,
                    adapter,
                    leads,
                    cid,
                    revision,
                } => SourceBody::Surveyed {
                    source,
                    adapter,
                    leads,
                    observed_cid: cid.as_str().to_string(),
                    observed_revision: revision,
                },
                SourceReport::Failed {
                    source,
                    error: SurveyError { kind, detail },
                } => SourceBody::Failed { source, kind, detail },
            })
            .collect();
        let Correlated {
            elements,
            relationships,
            claims,
        } = outcome.correlated;
        Self {
            id: outcome.id,
            decision: outcome.decision,
            candidates: outcome.candidates,
            sources,
            evidence: outcome.evidence,
            as_is: AsIsBody {
                elements,
                relationships,
                claims,
            },
        }
    }
}

impl Render for SurveyBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "System survey — {}", self.id)?;
        writeln!(w, "  decision: {}", self.decision)?;
        writeln!(w, "  candidates: {} ({} included)", self.candidates, self.sources.len())?;
        for source in &self.sources {
            match source {
                SourceBody::Surveyed {
                    source,
                    adapter,
                    leads,
                    observed_cid,
                    ..
                } => {
                    writeln!(w, "    - {source}: {leads} leads via {adapter} ({observed_cid})")?;
                }
                SourceBody::Failed { source, kind, detail } => {
                    let kind = match kind {
                        SurveyErrorKind::Access => "access",
                        SurveyErrorKind::Adapter => "adapter",
                    };
                    writeln!(w, "    - {source}: failed ({kind}) — {detail}")?;
                }
            }
        }
        writeln!(w, "  evidence: {} documents", self.evidence)?;
        writeln!(
            w,
            "  as-is: {} elements, {} relationships ({} claims)",
            self.as_is.elements, self.as_is.relationships, self.as_is.claims
        )
    }
}

/// Wire input for `emery system plan`.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct PlanInput {}

/// `emery system plan` — validate the definition, propose the initial
/// architecture when `target` is absent, reproject every view, and
/// project each wave's canonical handoff.
#[derive(Clone, Copy, Debug)]
pub struct Plan;

impl<P: Anchor + Model> Operation<P> for Plan {
    type Error = project::handler::Error;
    type Input = PlanInput;
    type Output = PlanBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let PlanInput {} = input;
        let outcome = orchestrate::plan::plan(context.provider, context.provider.paths()).await?;
        Ok(PlanBody::from(outcome))
    }
}

/// Success envelope for `emery system plan`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PlanBody {
    /// The declared engagement identity (`scope.yaml.id`).
    pub id: String,
    /// True when this run proposed the initial architecture.
    pub proposed: bool,
    /// The named states whose views were reprojected.
    pub states: Vec<String>,
    /// One entry per projected wave handoff, in plan order.
    pub waves: Vec<WaveBody>,
}

/// One projected wave handoff on the wire.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct WaveBody {
    /// The wave's id in `migration.yaml`.
    pub wave: String,
    /// The canonical handoff digest (`sha256:…`).
    pub handoff_digest: String,
}

impl From<PlanOutcome> for PlanBody {
    fn from(outcome: PlanOutcome) -> Self {
        Self {
            id: outcome.id,
            proposed: outcome.proposed,
            states: outcome.states,
            waves: outcome
                .waves
                .into_iter()
                .map(|WaveHandoff { wave, digest }| WaveBody {
                    wave,
                    handoff_digest: digest.as_str().to_string(),
                })
                .collect(),
        }
    }
}

impl Render for PlanBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "System plan — {}", self.id)?;
        if self.proposed {
            writeln!(w, "  proposed the initial target architecture and migration plan")?;
        }
        writeln!(w, "  projected states: {}", self.states.join(", "))?;
        if self.waves.is_empty() {
            writeln!(w, "  waves: none (no migration.yaml)")?;
        } else {
            writeln!(w, "  waves:")?;
            for wave in &self.waves {
                writeln!(w, "    - {}: handoff {}", wave.wave, wave.handoff_digest)?;
            }
        }
        Ok(())
    }
}

/// Wire input for `emery system review`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ReviewInput {
    /// The wave to review (`migration.yaml` `waves[].id`).
    pub wave: String,
    /// The exact handoff digest the operator reviewed (bare 64-hex or
    /// the full `sha256:…` form).
    pub handoff: String,
}

/// `emery system review` — record architectural authority over one
/// exact wave handoff (`system.wave.reviewed`).
#[derive(Clone, Copy, Debug)]
pub struct Review;

impl<P: Anchor> Operation<P> for Review {
    type Error = project::handler::Error;
    type Input = ReviewInput;
    type Output = ReviewBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let layout = Layout::new(context.provider.paths().project_root());
        // Handler-boundary wall-clock read (architecture §Time
        // injection); the kernel receives the timestamp explicitly.
        let now = jiff::Timestamp::now();
        let outcome = review::review(&layout, &input.wave, &input.handoff, now)?;
        Ok(ReviewBody::from(outcome))
    }
}

/// Success envelope for `emery system review`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ReviewBody {
    /// The reviewed wave's id.
    pub wave: String,
    /// The reviewed handoff digest (`sha256:…`).
    pub handoff_digest: String,
    /// False when the same handoff was already reviewed (no-op).
    pub recorded: bool,
}

impl From<ReviewOutcome> for ReviewBody {
    fn from(outcome: ReviewOutcome) -> Self {
        Self {
            wave: outcome.wave,
            handoff_digest: outcome.handoff_digest,
            recorded: outcome.recorded,
        }
    }
}

impl Render for ReviewBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.recorded {
            writeln!(w, "Recorded system.wave.reviewed for wave {}", self.wave)?;
        } else {
            writeln!(w, "Wave {} already reviewed at this handoff — nothing recorded", self.wave)?;
        }
        writeln!(w, "  handoff: {}", self.handoff_digest)
    }
}

/// Wire input for `emery system status`.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct StatusInput {}

/// `emery system status` — the read-only definition-home projection.
#[derive(Clone, Copy, Debug)]
pub struct Status;

impl<P: Anchor> Operation<P> for Status {
    type Error = project::handler::Error;
    type Input = StatusInput;
    type Output = StatusBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let StatusInput {} = input;
        let layout = Layout::new(context.provider.paths().project_root());
        let projected = status::project(&layout)?;
        Ok(StatusBody::from(projected))
    }
}

/// Success envelope for `emery system status`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct StatusBody {
    /// The declared engagement identity (`scope.yaml.id`).
    pub id: String,
    /// The decision the definition supports.
    pub decision: String,
    /// Included coverage rows.
    pub included: usize,
    /// Rows with any other disposition.
    pub other: usize,
    /// Included sources whose last run failed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failed_sources: Vec<String>,
    /// Named states with their sizes.
    pub states: Vec<StateBody>,
    /// Migration waves with review standing.
    pub waves: Vec<WaveStatusBody>,
    /// The computed next operator action.
    pub next: String,
}

/// One named state's wire accounting.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct StateBody {
    /// The state's name.
    pub name: String,
    /// Elements in the state.
    pub elements: usize,
    /// Relationships in the state.
    pub relationships: usize,
}

/// One wave's wire review standing.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct WaveStatusBody {
    /// The wave's id.
    pub wave: String,
    /// `reviewed | awaiting-review | stale`.
    pub standing: String,
    /// The current handoff digest, when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff_digest: Option<String>,
}

impl From<status::Status> for StatusBody {
    fn from(projected: status::Status) -> Self {
        let next = match &projected.next {
            NextAction::Survey => "emery system survey".to_string(),
            NextAction::Plan => "emery system plan".to_string(),
            NextAction::Review { wave } => {
                format!("emery system review {wave} --handoff <digest>")
            }
            NextAction::Replan { wave } => {
                format!("emery system plan (wave {wave} handoff is stale)")
            }
            NextAction::Reviewed => "reviewed".to_string(),
        };
        Self {
            id: projected.id,
            decision: projected.decision,
            included: projected.coverage.included,
            other: projected.coverage.other,
            failed_sources: projected.failed_sources,
            states: projected
                .states
                .into_iter()
                .map(|row| StateBody {
                    name: row.name,
                    elements: row.elements,
                    relationships: row.relationships,
                })
                .collect(),
            waves: projected
                .waves
                .into_iter()
                .map(|row| {
                    let (standing, handoff_digest) = match row.standing {
                        WaveStanding::Reviewed { handoff_digest } => {
                            ("reviewed".to_string(), Some(handoff_digest))
                        }
                        WaveStanding::AwaitingReview { handoff_digest } => {
                            ("awaiting-review".to_string(), Some(handoff_digest))
                        }
                        WaveStanding::Stale => ("stale".to_string(), None),
                    };
                    WaveStatusBody {
                        wave: row.wave,
                        standing,
                        handoff_digest,
                    }
                })
                .collect(),
            next,
        }
    }
}

impl Render for StatusBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "System status — {}", self.id)?;
        writeln!(w, "  decision: {}", self.decision)?;
        writeln!(w, "  coverage: {} included, {} other", self.included, self.other)?;
        for source in &self.failed_sources {
            writeln!(w, "    - {source}: last run failed (see coverage.yaml survey-error)")?;
        }
        if self.states.is_empty() {
            writeln!(w, "  model: none (run `emery system survey`)")?;
        } else {
            writeln!(w, "  model:")?;
            for state in &self.states {
                writeln!(
                    w,
                    "    - {}: {} elements, {} relationships",
                    state.name, state.elements, state.relationships
                )?;
            }
        }
        if !self.waves.is_empty() {
            writeln!(w, "  waves:")?;
            for wave in &self.waves {
                match &wave.handoff_digest {
                    Some(digest) => {
                        writeln!(w, "    - {}: {} ({digest})", wave.wave, wave.standing)?;
                    }
                    None => writeln!(w, "    - {}: {}", wave.wave, wave.standing)?,
                }
            }
        }
        writeln!(w, "  next: {}", self.next)
    }
}
