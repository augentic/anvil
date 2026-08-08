//! The native seam provider: project anchoring, ensure/resolve over
//! the compiled catalog, model access, and adapter dispatch. Native
//! ensure is a static package match — misses fail `adapter-not-linked`.

use adapter::seam::{self as aseam, Context};
use error::Error;
use omnia_guest::Model;
use omnia_guest::model::{Reply, Request};
use project::adapter::{
    AdapterSelector, Axis, FIRST_PARTY_NAMESPACE, Origin, ResolvedSource, ResolvedTarget, Resolver,
};
use project::handler::ExecutionPaths;
use project::seam::wire::BuildReport;
use project::seam::{self, Evidence, Input, Lead, Source, Target, Workspace};
use project::snapshot::{CodePatch, SnapshotId};
use project::workspace::{self as workspace_kernel, Access, Store};

use crate::catalog::{Catalog, Entry};
use crate::convert;
use crate::model::DynModel;

/// Whether a provider serves its adapters' reference documents.
///
/// Offline providers never start a listener — deterministic tests
/// state that explicitly. Online providers start the shared loopback
/// listener lazily, on the first adapter operation that carries
/// non-empty reference documents; a document-free catalog remains a
/// no-op, and a bind failure fails the operation rather than
/// stripping its grants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceMode {
    /// Never serve references or bind a listener.
    Offline,
    /// Serve references on a shared lazy loopback listener.
    Online,
}

/// The native host provider over a validated [`Catalog`] and an
/// erased [`DynModel`] backend.
///
/// Clones share the model, catalog, and reference listener — `Clone`
/// supports router invocation and shared capabilities, not concurrent
/// independent commands. The provider exposes no model accessor;
/// backends with post-run state hand out caller-held clones before
/// erasure.
#[derive(Clone, Debug)]
pub struct Provider {
    paths: ExecutionPaths,
    model: DynModel,
    catalog: Catalog,
    references: References,
}

#[derive(Clone, Debug)]
enum References {
    Offline,
    #[cfg(feature = "cli")]
    Online(std::sync::Arc<crate::references::ReferenceHost>),
    #[cfg(not(feature = "cli"))]
    Online,
}

impl Provider {
    /// A provider anchored at `paths` over the given model backend,
    /// native adapter catalog, and reference mode.
    #[must_use]
    pub fn new(
        paths: ExecutionPaths, model: DynModel, catalog: Catalog, references: ReferenceMode,
    ) -> Self {
        let references = match references {
            ReferenceMode::Offline => References::Offline,
            #[cfg(feature = "cli")]
            ReferenceMode::Online => References::Online(std::sync::Arc::new(
                crate::references::ReferenceHost::new(catalog.clone()),
            )),
            #[cfg(not(feature = "cli"))]
            ReferenceMode::Online => References::Online,
        };
        Self {
            paths,
            model,
            catalog,
            references,
        }
    }

    /// The native adapter catalog.
    #[must_use]
    pub const fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Request graceful reference-listener shutdown and await the
    /// server task; offline and never-started providers are a no-op.
    #[cfg(feature = "cli")]
    pub async fn shutdown(&self) {
        if let References::Online(host) = &self.references {
            host.shutdown().await;
        }
    }

    /// The shelf URL for one routed id, starting the shared listener
    /// when this entry is the first with reference documents.
    async fn mcp_url(&self, id: &str) -> Result<Option<String>, seam::Error> {
        let Some(entry) = self.catalog.find(id) else {
            return Ok(None);
        };
        if entry.docs().is_empty() {
            return Ok(None);
        }
        match &self.references {
            References::Offline => Ok(None),
            #[cfg(feature = "cli")]
            References::Online(host) => {
                let base =
                    host.base().await.map_err(|err| seam::Error::Internal(err.to_string()))?;
                Ok(base.map(|base| format!("{base}/mcp/{}", entry.name())))
            }
            #[cfg(not(feature = "cli"))]
            References::Online => Err(seam::Error::Internal(format!(
                "reference-listener-unavailable: `{id}` carries reference documents but \
                 this native host was built without the `cli` networking stack"
            ))),
        }
    }

    // One assembly point for the SDK context. The default lend is the
    // project root itself (the native stand-in for the guest's `"."`
    // preopen); build and merge re-lend their prepared workspace.
    fn ctx<'a>(&'a self, id: &'a str, url: Option<String>) -> Context<'a> {
        Context {
            adapter_id: id,
            project_root: self.paths.project_root(),
            mcp_url: url,
            lend: self.paths.project_root().display().to_string(),
        }
    }

    /// The snapshot store at the carried locations' snapshots root.
    fn store(&self) -> Store {
        Store::new(self.paths.locations().snapshots_root())
    }

    /// The private-workspace root from the carried locations.
    fn workspaces_root(&self) -> &std::path::Path {
        self.paths.locations().workspaces_root()
    }

    /// Match one selector against the compiled catalog (native
    /// ensure): bare by name, exact pin by `(name, version)`; anything
    /// else refuses as `adapter-not-linked`.
    fn matched(&self, axis: Axis, selector: &AdapterSelector) -> Result<Entry, Error> {
        match selector {
            AdapterSelector::Bare { name } => self.catalog.get(axis, name),
            AdapterSelector::Package {
                namespace,
                name,
                version,
            } => {
                if namespace != FIRST_PARTY_NAMESPACE {
                    return Err(not_linked(format!(
                        "adapter `{selector}` (axis `{axis}`) is not linked into this host: \
                         linked identities publish under the `{FIRST_PARTY_NAMESPACE}` namespace"
                    )));
                }
                let entry = self.catalog.get(axis, name)?;
                let linked = entry_version(&entry)?;
                // Unpublished adapters carry the `0.0.0` development
                // placeholder; they remain bare-only identities for
                // pin matching.
                if linked == PLACEHOLDER {
                    return Err(not_linked(format!(
                        "adapter `{selector}` (axis `{axis}`): the linked `{name}` carries the \
                         development placeholder version {PLACEHOLDER} and matches only a bare \
                         reference"
                    )));
                }
                if linked == *version {
                    Ok(entry)
                } else {
                    Err(not_linked(format!(
                        "adapter `{selector}` (axis `{axis}`) does not match the linked \
                         `{name}@{}`; use a compatible native build or the Wasm deployment",
                        entry.version()
                    )))
                }
            }
            AdapterSelector::Component { path } => Err(not_linked(format!(
                "native execution does not load the supplied component `{}`; linked \
                 identities on axis `{axis}`: [{}]",
                path.display(),
                self.catalog.axis_inventory(axis),
            ))),
        }
    }
}

impl project::handler::Anchor for Provider {
    fn paths(&self) -> &ExecutionPaths {
        &self.paths
    }
}

impl Resolver for Provider {
    fn resolve_source(
        &self, selector: &AdapterSelector, _paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        let entry = self.matched(Axis::Source, selector)?;
        project::adapter::resolver::source(
            entry.name(),
            Some(entry_version(&entry)?),
            entry.metadata(),
            origin(&entry),
        )
    }

    fn resolve_target(
        &self, selector: &AdapterSelector, _paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        let entry = self.matched(Axis::Target, selector)?;
        project::adapter::resolver::target(
            entry.name(),
            Some(entry_version(&entry)?),
            entry.metadata(),
            origin(&entry),
        )
    }
}

impl Model for Provider {
    async fn create(&self, request: Request) -> Result<Reply, omnia_guest::model::Error> {
        self.model.create(request).await
    }
}

impl Source for Provider {
    async fn survey(&self, id: String) -> Result<Vec<Lead>, seam::Error> {
        let ctx = self.ctx(&id, self.mcp_url(&id).await?);
        let leads = self.catalog.survey(&self.model, &ctx, &id).await.map_err(convert::error)?;
        Ok(leads.into_iter().map(convert::lead).collect())
    }

    async fn extract(&self, id: String, lead: Lead) -> Result<Evidence, seam::Error> {
        let ctx = self.ctx(&id, self.mcp_url(&id).await?);
        let lead = convert::narrow_lead(lead);
        let evidence =
            self.catalog.extract(&self.model, &ctx, &id, &lead).await.map_err(convert::error)?;
        Ok(convert::evidence(evidence))
    }
}

impl Target for Provider {
    async fn guidance(&self, id: String) -> Result<String, seam::Error> {
        let ctx = self.ctx(&id, self.mcp_url(&id).await?);
        self.catalog.guidance(&self.model, &ctx, &id).await.map_err(convert::error)
    }

    async fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, context: seam::BuildContext,
        workspace: Workspace,
    ) -> Result<BuildReport, seam::Error> {
        let ctx = self.ctx(&id, self.mcp_url(&id).await?).lending(workspace.root.clone());
        let inputs: Vec<aseam::Input> = inputs.into_iter().map(convert::narrow_input).collect();
        let context = convert::narrow_context(context);
        let workspace = convert::narrow_workspace(workspace);
        let report = self
            .catalog
            .build(&self.model, &ctx, &id, &slice, &inputs, &context, &workspace)
            .await
            .map_err(convert::error)?;
        Ok(convert::widen_report(&id, slice, report))
    }

    async fn merge(
        &self, id: String, slice: String, phase: seam::MergePhase, workspace: Workspace,
    ) -> Result<BuildReport, seam::Error> {
        let ctx = self.ctx(&id, self.mcp_url(&id).await?).lending(workspace.root.clone());
        let phase = convert::narrow_phase(phase);
        let workspace = convert::narrow_workspace(workspace);
        let report = self
            .catalog
            .merge(&self.model, &ctx, &id, &slice, phase, &workspace)
            .await
            .map_err(convert::error)?;
        Ok(convert::widen_report(&id, slice, report))
    }
}

impl seam::Workspaces for Provider {
    /// Freeze the project root's product tree (the kernel excludes
    /// `.git` and `.emery`) into the local snapshot store.
    async fn freeze(&self) -> Result<SnapshotId, seam::Error> {
        self.store().snapshot(self.paths.project_root()).map_err(|err| workspace_failure(&err))
    }

    async fn prepare(&self, base: SnapshotId, writable: bool) -> Result<Workspace, seam::Error> {
        let prepared = workspace_kernel::prepare(
            &self.store(),
            self.workspaces_root(),
            &base,
            Access { writable },
        )
        .map_err(|err| workspace_failure(&err))?;
        Ok(Workspace {
            id: prepared.id,
            root: prepared.root.display().to_string(),
            artifacts: host_absolute(self.paths.project_root()),
        })
    }

    async fn capture(&self, id: String) -> Result<CodePatch, seam::Error> {
        workspace_kernel::capture(&self.store(), self.workspaces_root(), &id)
            .map_err(|err| workspace_failure(&err))
    }

    async fn discard(&self, id: String) -> Result<(), seam::Error> {
        workspace_kernel::discard(self.workspaces_root(), &id)
            .map_err(|err| workspace_failure(&err))
    }

    async fn apply(&self, patch: CodePatch) -> Result<(), seam::Error> {
        self.store().apply(&patch, self.paths.project_root()).map_err(|err| workspace_failure(&err))
    }
}

/// Map a workspace-kernel failure onto the seam error contract.
fn workspace_failure(err: &Error) -> seam::Error {
    seam::Error::Internal(err.to_string())
}

/// The agent-visible artifact root: the project tree as a host-absolute
/// path, so a spawned agent working inside a lent workspace can still
/// read change-tree artifacts.
fn host_absolute(path: &std::path::Path) -> String {
    std::path::absolute(path).unwrap_or_else(|_io| path.to_path_buf()).display().to_string()
}

/// The development placeholder version unpublished adapters compile
/// with; placeholder identities match only bare references.
const PLACEHOLDER: semver::Version = semver::Version::new(0, 0, 0);

/// The exact compiled version a catalog entry resolves as.
fn entry_version(entry: &Entry) -> Result<semver::Version, Error> {
    semver::Version::parse(entry.version()).map_err(|err| Error::Diag {
        code: "adapter-not-linked",
        detail: format!(
            "linked adapter `{}` carries an invalid version `{}`: {err}",
            entry.name(),
            entry.version()
        ),
    })
}

const fn not_linked(detail: String) -> Error {
    Error::Diag {
        code: "adapter-not-linked",
        detail,
    }
}

fn origin(entry: &Entry) -> Origin {
    Origin {
        label: "native".to_string(),
        reference: format!("rust:{}", entry.id()),
    }
}
