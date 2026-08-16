//! `plan refine` — the serial refinement drain (RFC-91 D1/D7): drain
//! extraction + synthesis for a closed plan and stop before any code
//! work. No epoch, no claims, no target build operations.

use std::io::Write;

use error::Error;
use omnia_guest::Model;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::Resolver;
use project::handler::{Anchor, Ctx, Render};
use project::profile::Profiles;
use project::seam::{Shelf, Source, Target, Workspaces};
use serde::{Deserialize, Serialize};

use crate::orchestrate::{self, RefineOutcome};

/// Wire input for `plan refine`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefineInput {
    /// Repeatable `--slice <name>` selectors; empty targets every
    /// in-scope leaf.
    #[serde(default)]
    pub slice: Vec<String>,
}

/// `emery plan refine` → the serial refinement drain.
#[derive(Clone, Copy, Debug)]
pub struct Refine;

impl<P: Anchor + Model + Profiles + Resolver + Source + Shelf + Target + Workspaces> Operation<P>
    for Refine
{
    type Error = project::handler::Error;
    type Input = RefineInput;
    type Output = RefineBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let caps = orchestrate::Capabilities::provider(context.provider);
        let outcome = orchestrate::refine(caps, &cx.paths, cx.now(), &input.slice).await?;
        match outcome {
            RefineOutcome::Completed {
                plan,
                refined,
                skipped,
                gaps,
            } => Ok(RefineBody {
                status: "refined",
                plan,
                refined,
                skipped,
                gaps,
            }),
            // Typed halt: the plan status card rides stdout while the
            // payload-free `plan-refine-stopped` envelope keeps stderr
            // (exit 2 / 422) — mirrors `plan execute`'s stop shape.
            RefineOutcome::Stopped { slice, detail } => {
                let source = Error::validation_failed(
                    "plan-refine-stopped",
                    "the refinement drain reaches every targeted leaf",
                    format!("refinement stopped at `{slice}`: {detail}"),
                );
                let plan = project::plan::Plan::load(&cx.layout().plan_path())?;
                let status = project::plan::plan_status_body(&plan, cx.layout())?;
                Err(project::handler::Error::stopped(status, source))
            }
        }
    }
}

/// Success envelope for the `plan refine` drained exit.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefineBody {
    /// Always `refined` — a stop surfaces on the error envelope.
    pub status: &'static str,
    /// Plan name from `plan.yaml.name`.
    pub plan: String,
    /// Slices refined by this run, in drain order.
    pub refined: Vec<String>,
    /// Targeted slices skipped as already fresh, in drain order.
    pub skipped: Vec<String>,
    /// Whether the in-scope gap inventory is non-empty.
    pub gaps: bool,
}

impl Render for RefineBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        for slice in &self.refined {
            writeln!(w, "refined {slice}")?;
        }
        for slice in &self.skipped {
            writeln!(w, "fresh {slice} (skipped)")?;
        }
        if self.gaps {
            writeln!(w, "open gaps remain — review with emery plan gaps")?;
        }
        writeln!(
            w,
            "refinement complete — review the slice artifacts under .emery/change/slices/, then run \
             emery plan execute"
        )
    }
}
