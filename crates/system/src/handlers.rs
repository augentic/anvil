//! The `emery system *` operations. A definition home has no
//! `project.yaml`, so handlers anchor at the provider's raw
//! [`Anchor::project_root`] instead of a project `Ctx`.

use std::io::Write;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::Resolver;
use project::handler::{Anchor, Render};
use project::seam::{Origins, Source, Workspaces};
use serde::{Deserialize, Serialize};

use crate::coverage::{SurveyError, SurveyErrorKind};
use crate::orchestrate::{self, SourceReport, SurveyOutcome};

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

impl<P: Anchor + Source + Resolver + Workspaces + Origins> Operation<P> for Survey {
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
        Self {
            id: outcome.id,
            decision: outcome.decision,
            candidates: outcome.candidates,
            sources,
            evidence: outcome.evidence,
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
        writeln!(w, "  evidence: {} documents", self.evidence)
    }
}
