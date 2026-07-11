//! `plan author` — the collapsed `/spec:plan` flow exiting at
//! `lifecycle: pending`.

use std::io::Write;

use omnia_guest::Model;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use crate::change::plan::wire::{SourceAssign, source_map};
use crate::adapter::Resolver;
use crate::handler::{Anchor, Ctx, Render};
use crate::orchestrate;
use crate::seam::SourceSeam;

/// Wire input for `plan author`.
///
/// Carries the raw source surface on every transport — the same
/// [`SourceAssign`] list + `intent` sugar `plan create` takes; the
/// desugaring into the structured `plan.yaml.sources` map runs at the
/// operation boundary.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AuthorInput {
    /// Kebab-case change name.
    pub name: String,
    /// Raw source bindings (the `--source` repeat list).
    #[serde(default)]
    pub sources: Vec<SourceAssign>,
    /// Operator intent literal — sugar for
    /// `--source intent=intent:value:<string>`.
    #[serde(default)]
    pub intent: Option<String>,
}

/// `specify plan author <name> [--source ...]` →
/// [`orchestrate::author`] — the collapsed `/spec:plan` flow exiting
/// at `lifecycle: pending`.
#[derive(Clone, Copy, Debug)]
pub struct Author;

impl<P: Anchor + Model + Resolver + SourceSeam> Operation<P> for Author {
    type Error = crate::handler::Error;
    type Input = AuthorInput;
    type Output = AuthorBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let AuthorInput {
            name,
            sources,
            intent,
        } = input;
        let sources = source_map(sources, intent)?;
        let outcome = orchestrate::author(
            context.provider,
            context.provider,
            context.provider,
            cx.layout(),
            cx.now(),
            &name,
            sources,
        )
        .await?;
        Ok(AuthorBody {
            plan: outcome.plan,
            lifecycle: "pending",
            surveyed: outcome
                .surveyed
                .into_iter()
                .map(|surveyed| AuthorSurvey {
                    source: surveyed.source,
                    adapter: surveyed.adapter,
                    leads: surveyed.leads,
                })
                .collect(),
            slices: outcome.slices,
            hint: outcome.hint,
        })
    }
}

/// Success envelope for the `plan author` exit at `pending`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AuthorBody {
    /// Change name.
    pub plan: String,
    /// Always `pending` — Gate 1 stays operator-owned.
    pub lifecycle: &'static str,
    /// Surveyed sources in plan-binding order.
    pub surveyed: Vec<AuthorSurvey>,
    /// Authored slice names.
    pub slices: Vec<String>,
    /// The literal Gate 1 transition command.
    pub hint: String,
}

/// One surveyed source in the authoring fan-out.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AuthorSurvey {
    /// Source key.
    pub source: String,
    /// The bound adapter that answered.
    pub adapter: String,
    /// Lead ids the survey produced.
    pub leads: Vec<String>,
}

impl Render for AuthorBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        for surveyed in &self.surveyed {
            writeln!(
                w,
                "surveyed {} via {} ({} lead(s))",
                surveyed.source,
                surveyed.adapter,
                surveyed.leads.len()
            )?;
        }
        for slice in &self.slices {
            writeln!(w, "slice: {slice}")?;
        }
        writeln!(w, "{}", self.hint)
    }
}
