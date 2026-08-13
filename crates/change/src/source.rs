//! The `emery source survey` operation (the plan-scoped half of the
//! source-axis surface; extract lives in the `slice` crate).

use std::io::Write;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::Resolver;
use project::handler::{Anchor, Ctx, Render};
use project::seam::Source;
use serde::{Deserialize, Serialize};

use crate::orchestrate;

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

/// `emery source survey <source>` → the internal survey orchestration.
#[derive(Clone, Copy, Debug)]
pub struct Survey;

impl<P: Anchor + Source + Resolver> Operation<P> for Survey {
    type Error = project::handler::Error;
    type Input = SurveyInput;
    type Output = SurveyBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let outcome = orchestrate::survey(
            context.provider,
            context.provider,
            &cx.paths,
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
    /// Lead ids merged into `leads.md`.
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
