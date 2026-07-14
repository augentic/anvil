//! `slice build` — the guest-routed target-build verb.

use std::io::Write;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::Resolver;
use project::handler::{Anchor, Ctx, Render};
use project::seam::{TargetSeam, WorkingTree};
use serde::{Deserialize, Serialize};

use crate::{BuildStatus, orchestrate};

/// Wire input for `slice build`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildInput {
    /// Slice name (under `.specify/slices/`).
    pub name: String,
}

/// `specify slice build <name>` → the internal build orchestration.
#[derive(Clone, Copy, Debug)]
pub struct Build;

impl<P: Anchor + Resolver + TargetSeam> Operation<P> for Build {
    type Error = project::handler::Error;
    type Input = BuildInput;
    type Output = BuildBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let adapter = cx.resolve_target_adapter(context.provider)?;
        let outcome = orchestrate::build(
            context.provider,
            cx.layout(),
            cx.now(),
            &input.name,
            &adapter.manifest,
            WorkingTree::live(),
        )
        .await?;
        Ok(BuildBody {
            slice: outcome.slice,
            target: outcome.target,
            status: outcome.status,
            findings: outcome.findings,
        })
    }
}

/// Success envelope for the collapsed `slice build`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildBody {
    /// Slice name.
    pub slice: String,
    /// Target adapter identifier.
    pub target: String,
    /// Report status.
    pub status: BuildStatus,
    /// Finding count on the report.
    pub findings: usize,
}

impl Render for BuildBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "built {} against {} ({} finding(s))", self.slice, self.target, self.findings)
    }
}
