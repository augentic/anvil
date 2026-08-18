//! `emery specify` — the one loop (ADR-0008 §2): extract every bound
//! source, reconcile and synthesise under authority precedence, and
//! commit the gated spec set behind the generation pointer.

use std::io::Write;

use omnia_guest::Model;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use crate::extract::{Extract, Receipt, extract_all};
use crate::handler::{Anchor, Render, RequestContext};
use crate::home::{Home, SpecSet};
use crate::resolve::Resolver;
use crate::synthesise::{reconcile, synthesise};

/// Wire input for `emery specify` (no flags — ADR-0008 §3).
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct SpecifyInput;

/// Success body: the committed generation and its reviewable set.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyBody {
    /// The committed generation id the pointer names.
    pub generation: String,
    /// Requirement blocks in the committed `spec.md`.
    pub requirements: usize,
    /// Sources extracted this run.
    pub sources: usize,
}

impl Render for SpecifyBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "committed generation {}", self.generation)?;
        writeln!(w, "  requirements: {}", self.requirements)?;
        writeln!(w, "  sources: {}", self.sources)?;
        Ok(())
    }
}

/// The live `specify` route over the provider seam.
#[derive(Clone, Copy, Debug)]
pub struct Specify;

impl<P: Anchor + Resolver + Extract + Model> Operation<P> for Specify {
    type Error = crate::handler::Error;
    type Input = SpecifyInput;
    type Output = SpecifyBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let SpecifyInput = input;
        let request = RequestContext::load(context.provider)?;
        let paths = request.paths();
        let project_dir = paths.project_root();
        let project = request.project();

        let sets = extract_all(context.provider, project, paths).await?;
        let rows = reconcile(&sets);
        let documents = synthesise(context.provider, &sets, &rows).await?;

        let receipts: Vec<Receipt> = sets.iter().map(Receipt::of).collect();
        let set = SpecSet {
            bindings: artifacts::atomic::serialise_yaml(&project.sources)?,
            receipts: artifacts::atomic::serialise_yaml(&receipts)?,
            spec: documents.spec,
            design: documents.design,
        };
        let committed = Home::new(project_dir).commit(&set)?;
        Ok(SpecifyBody {
            generation: committed.id,
            requirements: rows.len(),
            sources: sets.len(),
        })
    }
}
