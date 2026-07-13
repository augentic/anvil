//! `slice refine` — the `/spec:refine` breakout outside the execute
//! loop.

use std::io::Write;

use omnia_guest::Model;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::Resolver;
use project::handler::{Anchor, Ctx, Render};
use project::seam::{SourceSeam, TargetSeam};
use serde::{Deserialize, Serialize};

use crate::orchestrate;

/// Wire input for `slice refine`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefineInput {
    /// Slice name (a `plan.yaml.slices[]` entry).
    pub name: String,
}

/// `specify slice refine <name>` → the internal refine orchestration —
/// the `/spec:refine` breakout outside the execute loop.
#[derive(Clone, Copy, Debug)]
pub struct Refine;

impl<P: Anchor + Model + Resolver + SourceSeam + TargetSeam> Operation<P> for Refine {
    type Error = project::handler::Error;
    type Input = RefineInput;
    type Output = RefineBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let caps = orchestrate::Capabilities::provider(context.provider);
        let outcome =
            orchestrate::refine_breakout(caps, cx.layout(), cx.now(), &input.name).await?;
        Ok(RefineBody {
            slice: outcome.slice,
            artifacts: outcome.artifacts,
            extracted: outcome
                .extracted
                .into_iter()
                .map(|(source, lead)| RefineExtract { source, lead })
                .collect(),
            tags: RefineTags {
                unknown: outcome.tags.unknown,
                conflict: outcome.tags.conflict,
                divergence: outcome.tags.divergence,
            },
        })
    }
}

/// Success envelope for the `slice refine` breakout.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefineBody {
    /// Slice name.
    pub slice: String,
    /// Written artifact names.
    pub artifacts: Vec<String>,
    /// Extracted `(source, lead)` pairs, in binding order.
    pub extracted: Vec<RefineExtract>,
    /// Synthesis-tag counts from the validate sweep — review signals,
    /// never a park.
    pub tags: RefineTags,
}

/// Per-tag requirement counts on the refine envelope.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefineTags {
    /// `[unknown]` requirements.
    pub unknown: usize,
    /// `[conflict]` requirements.
    pub conflict: usize,
    /// `[divergence]` requirements.
    pub divergence: usize,
}

/// One extracted `(source, lead)` pair.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefineExtract {
    /// Source key.
    pub source: String,
    /// Lead id.
    pub lead: String,
}

impl Render for RefineBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "refined {}", self.slice)?;
        for extract in &self.extracted {
            writeln!(w, "extracted: {}/{}", extract.source, extract.lead)?;
        }
        for artifact in &self.artifacts {
            writeln!(w, "artifact: {artifact}")?;
        }
        writeln!(
            w,
            "spec tags: {} unknown, {} conflict, {} divergence",
            self.tags.unknown, self.tags.conflict, self.tags.divergence
        )?;
        Ok(())
    }
}
