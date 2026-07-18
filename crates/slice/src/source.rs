//! The `specify source extract` operation (the slice-scoped half of
//! the source-axis surface; survey lives in the `change` crate).

use std::io::Write;
use std::path::PathBuf;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::Resolver;
use project::handler::{Anchor, Ctx, Render};
use project::seam::Source;
use serde::{Deserialize, Serialize};

use crate::orchestrate;

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

impl<P: Anchor + Source + Resolver> Operation<P> for Extract {
    type Error = project::handler::Error;
    type Input = ExtractInput;
    type Output = ExtractBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let outcome = orchestrate::extract(
            context.provider,
            context.provider,
            &cx.paths,
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
