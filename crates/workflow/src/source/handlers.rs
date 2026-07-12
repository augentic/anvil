//! The `specify source survey` / `specify source extract` operations.

use std::io::Write;
use std::path::PathBuf;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use crate::handler::{Anchor, Ctx, Render};
use crate::orchestrate;
use crate::seam::SourceSeam;

// ---------------------------------------------------------------------------
// source survey
// ---------------------------------------------------------------------------

/// Wire input for `source survey`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SurveyInput {
    /// Source key from `plan.yaml.sources.<key>`.
    pub source: String,
    /// Plan name guard; when set, must match `plan.yaml.name`.
    #[serde(default)]
    pub plan: Option<String>,
}

/// `specify source survey <source>` → the internal survey orchestration.
#[derive(Clone, Copy, Debug)]
pub struct Survey;

impl<P: Anchor + SourceSeam> Operation<P> for Survey {
    type Error = crate::handler::Error;
    type Input = SurveyInput;
    type Output = SurveyBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let outcome = orchestrate::survey(
            context.provider,
            cx.layout(),
            cx.now(),
            &input.source,
            input.plan.as_deref(),
        )
        .await?;
        Ok(SurveyBody {
            source: outcome.source,
            adapter: outcome.adapter,
            leads: outcome.leads,
        })
    }
}

/// Success envelope for the collapsed `source survey`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SurveyBody {
    /// Source key.
    pub source: String,
    /// The bound adapter that answered.
    pub adapter: String,
    /// Lead ids merged into `discovery.md`.
    pub leads: Vec<String>,
}

impl Render for SurveyBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "surveyed {} via {}", self.source, self.adapter)?;
        for lead in &self.leads {
            writeln!(w, "lead: {lead}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// source extract
// ---------------------------------------------------------------------------

/// Wire input for `source extract`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExtractInput {
    /// Source key from `plan.yaml.sources.<key>`.
    pub source: String,
    /// Lead id from `discovery.md`.
    pub lead: String,
    /// Slice the Evidence is extracted into.
    pub slice: String,
}

/// `specify source extract <source> <lead> --slice <slice>` →
/// the internal extract orchestration.
#[derive(Clone, Copy, Debug)]
pub struct Extract;

impl<P: Anchor + SourceSeam> Operation<P> for Extract {
    type Error = crate::handler::Error;
    type Input = ExtractInput;
    type Output = ExtractBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let outcome = orchestrate::extract(
            context.provider,
            cx.layout(),
            cx.now(),
            &input.source,
            &input.lead,
            &input.slice,
        )
        .await?;
        Ok(ExtractBody {
            source: outcome.source,
            adapter: outcome.adapter,
            evidence: outcome.evidence,
        })
    }
}

/// Success envelope for the collapsed `source extract`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExtractBody {
    /// Source key.
    pub source: String,
    /// The bound adapter that answered.
    pub adapter: String,
    /// Path of the persisted Evidence document.
    pub evidence: PathBuf,
}

impl Render for ExtractBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "extracted {} via {}", self.source, self.adapter)?;
        writeln!(w, "evidence: {}", self.evidence.display())
    }
}
