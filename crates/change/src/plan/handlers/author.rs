//! `plan author` — the collapsed `/emery:plan` flow. Exits after
//! authoring; the operator reviews the plan, then starts execution.

use std::io::Write;
use std::path::PathBuf;

use omnia_guest::Model;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::Resolver;
use project::handler::{Anchor, Ctx, Render};
use project::seam::Source;
use serde::{Deserialize, Serialize};

use crate::orchestrate;
use crate::plan::wire::{SourceAssign, source_map};

/// Wire input for `plan author`.
///
/// Carries the raw source surface on every transport — the
/// [`SourceAssign`] list + `intent` sugar — and the `--from`/`--wave`
/// definition-home pair. Binding from a reviewed definition is not
/// implemented yet; that path returns `plan-author-unimplemented`.
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
    /// Reviewed definition home. Requires [`Self::wave`].
    #[serde(default)]
    pub from: Option<PathBuf>,
    /// Wave id inside [`Self::from`]. Requires [`Self::from`].
    #[serde(default)]
    pub wave: Option<String>,
    /// Replace an existing plan unconditionally, whatever its entry
    /// statuses. Without this flag an existing `plan.yaml` refuses
    /// with `plan-already-exists`.
    #[serde(default)]
    pub force: bool,
}

/// `emery plan author <name> [--source ...]` →
/// the internal author orchestration — the collapsed `/emery:plan`
/// flow, exiting with the literal execute hint.
#[derive(Clone, Copy, Debug)]
pub struct Author;

impl<P: Anchor + Model + Resolver + Source> Operation<P> for Author {
    type Error = project::handler::Error;
    type Input = AuthorInput;
    type Output = AuthorBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        if input.from.is_some() || input.wave.is_some() {
            if input.from.is_none() || input.wave.is_none() {
                return Err(error::Error::Argument {
                    flag: "--from",
                    detail: "--from and --wave are required together".into(),
                }
                .into());
            }
            if !input.sources.is_empty() || input.intent.is_some() {
                return Err(error::Error::Argument {
                    flag: "--from",
                    detail: "--from/--wave cannot be combined with --source or --intent".into(),
                }
                .into());
            }
            return Err(error::Error::validation_failed(
                "plan-author-unimplemented",
                "authoring from a reviewed definition is not implemented yet",
                "in-place `emery plan author` without --from still works; --from/--wave binding lands with the discovery phase",
            )
            .into());
        }
        let cx = Ctx::load(context.provider)?;
        let AuthorInput {
            name,
            sources,
            intent,
            force,
            ..
        } = input;
        let sources = source_map(sources, intent)?;
        let caps = orchestrate::Capabilities::provider(context.provider).sans_targets();
        let outcome = orchestrate::author(caps, &cx.paths, cx.now(), &name, sources, force).await?;
        Ok(AuthorBody::from(outcome))
    }
}

/// Success envelope for the `plan author` exit.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AuthorBody {
    /// Change name.
    pub plan: String,
    /// Surveyed sources in plan-binding order.
    pub surveyed: Vec<AuthorSurvey>,
    /// Authored slice names.
    pub slices: Vec<String>,
    /// The literal execute command that starts the plan.
    pub hint: String,
}

impl From<orchestrate::AuthorOutcome> for AuthorBody {
    fn from(outcome: orchestrate::AuthorOutcome) -> Self {
        Self {
            plan: outcome.plan,
            surveyed: outcome.surveyed.into_iter().map(AuthorSurvey::from).collect(),
            slices: outcome.slices,
            hint: outcome.hint,
        }
    }
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

impl From<orchestrate::SurveyedSource> for AuthorSurvey {
    fn from(surveyed: orchestrate::SurveyedSource) -> Self {
        Self {
            source: surveyed.source,
            adapter: surveyed.adapter,
            leads: surveyed.leads,
        }
    }
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
