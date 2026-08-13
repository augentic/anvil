//! `plan author` — bind a reviewed handoff and decompose it.

use std::io::Write;
use std::path::PathBuf;

use omnia_guest::Model;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::{Inventory, Resolver};
use project::handler::{Anchor, Render};
use project::profile::Profiles;
use project::seam::{Ingest, Source, Workspaces};
use serde::{Deserialize, Serialize};

use crate::orchestrate;

/// Wire input for `plan author`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AuthorInput {
    /// Kebab-case change name.
    pub name: String,
    /// Reviewed definition home.
    pub from: PathBuf,
    /// Wave id inside [`Self::from`].
    pub wave: String,
    /// Replace an existing plan unconditionally. Rebind requires the
    /// same reviewed handoff.
    #[serde(default)]
    pub force: bool,
}

/// `emery plan author <name> --from <dir> --wave <id>` — bind and decompose.
#[derive(Clone, Copy, Debug)]
pub struct Author;

impl<P: Anchor + Model + Resolver + Inventory + Profiles + Ingest + Source + Workspaces>
    Operation<P> for Author
{
    type Error = project::handler::Error;
    type Input = AuthorInput;
    type Output = AuthorBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let paths = context.provider.paths();
        let outcome = orchestrate::author(
            context.provider,
            paths,
            jiff::Timestamp::now(),
            &input.name,
            &input.from,
            &input.wave,
            input.force,
        )
        .await?;
        Ok(AuthorBody::from(outcome))
    }
}

/// Success envelope for a completed `plan author`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AuthorBody {
    /// Change name.
    pub plan: String,
    /// Canonical `discovery.yaml` digest.
    pub discovery_digest: String,
    /// Canonical `leads.md` digest.
    pub leads_digest: String,
    /// Canonical `decomposition.yaml` digest.
    pub decomposition_digest: String,
    /// Bound target ids.
    pub targets: Vec<String>,
    /// Bound source keys.
    pub sources: Vec<String>,
    /// Projected slice names, in tree order.
    pub slices: Vec<String>,
}

impl From<orchestrate::AuthorOutcome> for AuthorBody {
    fn from(outcome: orchestrate::AuthorOutcome) -> Self {
        Self {
            plan: outcome.plan,
            discovery_digest: outcome.discovery_digest.to_string(),
            leads_digest: outcome.leads_digest.to_string(),
            decomposition_digest: outcome.decomposition_digest.to_string(),
            targets: outcome.targets,
            sources: outcome.sources,
            slices: outcome.slices,
        }
    }
}

impl Render for AuthorBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "plan: {}", self.plan)?;
        writeln!(w, "discovery-digest: {}", self.discovery_digest)?;
        writeln!(w, "leads-digest: {}", self.leads_digest)?;
        writeln!(w, "decomposition-digest: {}", self.decomposition_digest)?;
        for target in &self.targets {
            writeln!(w, "target: {target}")?;
        }
        for source in &self.sources {
            writeln!(w, "source: {source}")?;
        }
        for slice in &self.slices {
            writeln!(w, "slice: {slice}")?;
        }
        Ok(())
    }
}
