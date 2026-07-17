//! Native seam provider: project anchoring, judgment, and adapter dispatch.
//! Maps adapter seam DTOs onto workflow seam DTOs like the wasm guest shim.

use std::path::{Path, PathBuf};

use adapter::seam::{self as aseam, Context};
use error::Error;
use omnia_guest::Model;
use omnia_guest::model::{Reply, Request};
use project::adapter::{AdapterRef, Axis, Origin, ResolvedSource, ResolvedTarget, Resolver};
use project::seam::wire::BuildReport;
use project::seam::{self, Evidence, Input, Lead, Source, Target, WorkingTree};

use crate::catalog::{Catalog, Entry};
use crate::convert;

/// Native shim provider over a linked-adapter [`Catalog`] and a
/// [`Model`] backend.
#[derive(Debug)]
pub struct Provider<M> {
    project_dir: PathBuf,
    model: M,
    catalog: Catalog<M>,
    mcp_base: Option<String>,
}

impl<M: Clone> Clone for Provider<M> {
    fn clone(&self) -> Self {
        Self {
            project_dir: self.project_dir.clone(),
            model: self.model.clone(),
            catalog: self.catalog.clone(),
            mcp_base: self.mcp_base.clone(),
        }
    }
}

impl<M> Provider<M> {
    /// A provider anchored at `project_dir` over the given model backend
    /// and linked-adapter catalog.
    pub fn new(project_dir: impl Into<PathBuf>, model: M, catalog: Catalog<M>) -> Self {
        Self {
            project_dir: project_dir.into(),
            model,
            catalog,
            mcp_base: None,
        }
    }

    /// A provider over `B`'s catalog with its reference shelves served
    /// on an ephemeral background listener (skipped when no port can
    /// be bound); the shelf base URL feeds the per-operation MCP grant
    /// rewrite.
    #[cfg(feature = "cursor")]
    pub async fn bound<B: crate::catalog::Binding>(root: impl Into<PathBuf>, model: M) -> Self
    where
        M: Model,
    {
        let catalog = B::catalog();
        let base = crate::mcp::ephemeral_base(&catalog).await;
        let mut provider = Self::new(root, model, catalog);
        provider.mcp_base = base;
        provider
    }

    /// The configured model backend.
    pub const fn model(&self) -> &M {
        &self.model
    }

    /// The linked-adapter catalog.
    pub const fn catalog(&self) -> &Catalog<M> {
        &self.catalog
    }

    fn mcp_url(&self, id: &str) -> Option<String> {
        let name = id.rsplit(':').next().unwrap_or(id);
        self.mcp_base.as_ref().map(|base| format!("{base}/mcp/{name}"))
    }

    // Every operation method computes the URL first (the ctx borrows it),
    // then assembles the SDK context through this one place.
    fn ctx<'a>(&'a self, id: &'a str, url: Option<&'a str>) -> Context<'a> {
        Context {
            adapter_id: id,
            project_root: &self.project_dir,
            mcp_url: url,
        }
    }
}

impl<M: Send + Sync + 'static> project::handler::Anchor for Provider<M> {
    fn project_root(&self) -> &Path {
        &self.project_dir
    }
}

impl<M: Send + Sync> Resolver for Provider<M> {
    fn resolve_source(
        &self, adapter_ref: &AdapterRef, _project_dir: &Path,
    ) -> Result<ResolvedSource, Error> {
        require_bare(adapter_ref)?;
        let entry = self.catalog.get(Axis::Source, &adapter_ref.name)?;
        project::adapter::resolver::source(adapter_ref, entry.metadata(), origin(&entry))
    }

    fn resolve_target(
        &self, adapter_ref: &AdapterRef, _project_dir: &Path,
    ) -> Result<ResolvedTarget, Error> {
        require_bare(adapter_ref)?;
        let entry = self.catalog.get(Axis::Target, &adapter_ref.name)?;
        project::adapter::resolver::target(adapter_ref, entry.metadata(), origin(&entry))
    }
}

impl<M: Send + Sync> project::adapter::Hydrator for Provider<M> {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, Error> {
        Err(Error::Diag {
            code: "adapter-hydrate-unavailable",
            detail: format!(
                "the native harness links adapters directly and fetches nothing (requested {url})"
            ),
        })
    }
}

impl<M: Model> Model for Provider<M> {
    async fn create(&self, request: Request) -> Result<Reply, omnia_guest::model::Error> {
        self.model.create(request).await
    }
}

impl<M: Model> Source for Provider<M> {
    async fn survey(&self, id: String) -> Result<Vec<Lead>, seam::Error> {
        let url = self.mcp_url(&id);
        let ctx = self.ctx(&id, url.as_deref());
        let leads = self.catalog.survey(&self.model, &ctx, &id).await.map_err(convert::error)?;
        Ok(leads.into_iter().map(convert::lead).collect())
    }

    async fn extract(&self, id: String, lead: Lead) -> Result<Evidence, seam::Error> {
        let url = self.mcp_url(&id);
        let ctx = self.ctx(&id, url.as_deref());
        let lead = convert::narrow_lead(lead);
        let evidence =
            self.catalog.extract(&self.model, &ctx, &id, &lead).await.map_err(convert::error)?;
        Ok(convert::evidence(evidence))
    }
}

impl<M: Model> Target for Provider<M> {
    async fn guidance(&self, id: String) -> Result<String, seam::Error> {
        let url = self.mcp_url(&id);
        let ctx = self.ctx(&id, url.as_deref());
        self.catalog.guidance(&self.model, &ctx, &id).await.map_err(convert::error)
    }

    async fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, tree: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        let url = self.mcp_url(&id);
        let ctx = self.ctx(&id, url.as_deref());
        let inputs: Vec<aseam::Input> = inputs.into_iter().map(convert::narrow_input).collect();
        let tree = convert::narrow_tree(tree);
        let report = self
            .catalog
            .build(&self.model, &ctx, &id, &slice, &inputs, &tree)
            .await
            .map_err(convert::error)?;
        Ok(convert::widen_report(&id, slice, report))
    }

    async fn merge(
        &self, id: String, slice: String, phase: seam::MergePhase, tree: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        let url = self.mcp_url(&id);
        let ctx = self.ctx(&id, url.as_deref());
        let phase = convert::narrow_phase(phase);
        let tree = convert::narrow_tree(tree);
        let report = self
            .catalog
            .merge(&self.model, &ctx, &id, &slice, phase, &tree)
            .await
            .map_err(convert::error)?;
        Ok(convert::widen_report(&id, slice, report))
    }
}

fn require_bare(adapter_ref: &AdapterRef) -> Result<(), Error> {
    if adapter_ref.version.is_none() {
        return Ok(());
    }
    Err(Error::Diag {
        code: "adapter-not-found",
        detail: format!(
            "native adapter resolution accepts bare development identities only; \
             `{}` is pinned and must resolve through the component deployment",
            adapter_ref.name
        ),
    })
}

fn origin<M>(entry: &Entry<M>) -> Origin {
    Origin {
        label: "native".to_string(),
        reference: format!("rust:{}", entry.id()),
    }
}
