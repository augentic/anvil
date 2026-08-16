//! The native seam provider: project anchoring, ensure/resolve over
//! the compiled catalog, model access, and adapter dispatch. Native
//! ensure is a static package match — misses fail `adapter-not-linked`.

use adapter::seam::{self as aseam, Context};
use diagnostics::Diagnostic;
use error::Error;
use omnia_guest::Model;
use omnia_guest::model::{Reply, Request};
use project::adapter::catalog::Catalog as Adapters;
use project::adapter::{
    AdapterSelector, Axis, FIRST_PARTY_NAMESPACE, Inventory, Origin, ResolvedSource,
    ResolvedTarget, Resolver,
};
use project::handler::ExecutionPaths;
use project::profile::{self, Profiles};
use project::seam::wire::{BuildReport, PhaseReport, RepairOrigin};
use project::seam::{
    self, Evidence, Input, Shelf, Source, SourceInput, SurveyResult, Target, Workspace,
};
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
    inventory: Option<std::sync::Arc<Adapters>>,
    profiles: Option<std::sync::Arc<profile::Table>>,
    references: References,
    worktree: Option<WorktreeScript>,
    forge: Option<ForgeScript>,
}

/// One scripted export answer: the node-local worktree path plus the
/// idempotency state, or a closed D11 refusal.
pub type WorktreeAnswer = Result<(String, seam::WorktreeState), seam::WorktreeError>;

/// A scripted stand-in for the D11 worktree export — test suites
/// answer export requests without host Git or a real repository.
#[derive(Clone)]
pub struct WorktreeScript(
    std::sync::Arc<dyn Fn(&seam::WorktreeRequest) -> WorktreeAnswer + Send + Sync>,
);

impl WorktreeScript {
    /// Wrap one answer function.
    pub fn new(
        answer: impl Fn(&seam::WorktreeRequest) -> WorktreeAnswer + Send + Sync + 'static,
    ) -> Self {
        Self(std::sync::Arc::new(answer))
    }
}

impl std::fmt::Debug for WorktreeScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorktreeScript")
    }
}

/// One scripted forge answer: the pull requests for one
/// `(repository, branch)` lookup, or a typed forge failure.
pub type ForgeAnswer = Result<Vec<seam::PullRequest>, seam::ForgeError>;

/// The boxed answer function inside a [`ForgeScript`].
type ForgeAnswerFn = dyn Fn(&str, &str) -> ForgeAnswer + Send + Sync;

/// A scripted stand-in for the D10 forge read — test suites answer
/// find requests without GitHub or outgoing HTTP.
#[derive(Clone)]
pub struct ForgeScript(std::sync::Arc<ForgeAnswerFn>);

impl ForgeScript {
    /// Wrap one `(repository, branch)` answer function.
    pub fn new(answer: impl Fn(&str, &str) -> ForgeAnswer + Send + Sync + 'static) -> Self {
        Self(std::sync::Arc::new(answer))
    }
}

impl std::fmt::Debug for ForgeScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ForgeScript")
    }
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
            inventory: None,
            profiles: None,
            references,
            worktree: None,
            forge: None,
        }
    }

    /// Script the D11 worktree export instead of running host Git —
    /// change-suite scaffolding for the publication seam.
    #[must_use]
    pub fn with_worktree_script(mut self, script: WorktreeScript) -> Self {
        self.worktree = Some(script);
        self
    }

    /// Script the D10 forge read instead of speaking GitHub REST —
    /// change-suite scaffolding for archive verification.
    #[must_use]
    pub fn with_forge_script(mut self, script: ForgeScript) -> Self {
        self.forge = Some(script);
        self
    }

    /// Replace the compiled first-party inventory.
    #[must_use]
    pub fn with_inventory(mut self, inventory: Adapters) -> Self {
        self.inventory = Some(std::sync::Arc::new(inventory));
        self
    }

    /// Replace the compiled model-capability profile table.
    #[must_use]
    pub fn with_profiles(mut self, profiles: profile::Table) -> Self {
        self.profiles = Some(std::sync::Arc::new(profiles));
        self
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
                Ok(Some(format!("{base}/mcp/{}", entry.name())))
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
            lend: Some(self.paths.project_root().display().to_string()),
        }
    }

    // Source legs lend the CID view; a value-backed leg lends
    // `scratch` (a fresh empty directory) instead — agent backends
    // need a working tree, and sources get no project or change home.
    fn source_ctx<'a>(
        id: &'a str, url: Option<String>, input: &'a aseam::SourceInput,
        scratch: Option<&'a std::path::Path>,
    ) -> Context<'a> {
        match &input.content {
            aseam::SourceContent::Workspace(view) => Context {
                adapter_id: id,
                project_root: std::path::Path::new(&view.root),
                mcp_url: url,
                lend: Some(view.root.clone()),
            },
            aseam::SourceContent::Value(_) => Context {
                adapter_id: id,
                project_root: scratch.unwrap_or_else(|| std::path::Path::new("")),
                mcp_url: url,
                lend: scratch.map(|path| path.display().to_string()),
            },
        }
    }

    /// A disposable empty scratch directory for a value-backed source
    /// leg, dropped (and removed) after the dispatch returns.
    fn scratch(input: &aseam::SourceInput) -> Result<Option<tempfile::TempDir>, seam::Error> {
        match &input.content {
            aseam::SourceContent::Workspace(_) => Ok(None),
            aseam::SourceContent::Value(_) => tempfile::tempdir()
                .map(Some)
                .map_err(|err| seam::Error::Internal(format!("source scratch dir: {err}"))),
        }
    }

    /// The snapshot store at the carried locations' snapshots root.
    fn store(&self) -> Store<project::workspace::FsObjects> {
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
                // Detached topology records every binding as an exact
                // pin, including native mock identities compiled at
                // `0.0.0`. Match the compiled version exactly.
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

impl Inventory for Provider {
    fn inventory(&self) -> &Adapters {
        static FIRST: std::sync::LazyLock<Adapters> =
            std::sync::LazyLock::new(Adapters::first_party);
        self.inventory.as_deref().unwrap_or(&FIRST)
    }
}

impl Profiles for Provider {
    fn profiles(&self) -> &profile::Table {
        static FIRST: std::sync::LazyLock<profile::Table> =
            std::sync::LazyLock::new(profile::Table::compiled);
        self.profiles.as_deref().unwrap_or(&FIRST)
    }
}

impl Model for Provider {
    async fn create(&self, request: Request) -> Result<Reply, omnia_guest::model::Error> {
        self.model.create(request).await
    }
}

impl Source for Provider {
    async fn survey(&self, id: String, input: SourceInput) -> Result<SurveyResult, seam::Error> {
        let input = convert::narrow_source_input(input);
        let scratch = Self::scratch(&input)?;
        let ctx = Self::source_ctx(
            &id,
            self.mcp_url(&id).await?,
            &input,
            scratch.as_ref().map(tempfile::TempDir::path),
        );
        let result =
            self.catalog.survey(&self.model, &ctx, &id, &input).await.map_err(convert::error)?;
        Ok(convert::survey_result(result))
    }

    async fn extract(&self, id: String, input: SourceInput) -> Result<Evidence, seam::Error> {
        let input = convert::narrow_source_input(input);
        let scratch = Self::scratch(&input)?;
        let ctx = Self::source_ctx(
            &id,
            self.mcp_url(&id).await?,
            &input,
            scratch.as_ref().map(tempfile::TempDir::path),
        );
        let evidence =
            self.catalog.extract(&self.model, &ctx, &id, &input).await.map_err(convert::error)?;
        Ok(convert::evidence(evidence))
    }
}

impl Shelf for Provider {
    /// The engine's synthesis shelf on the shared loopback reference
    /// listener (RFC-96 D9), starting it on first use. Offline
    /// providers grant nothing — deterministic tests keep the full
    /// inline prompt and never bind a socket.
    async fn synthesis_shelf(&self) -> Result<Option<String>, seam::Error> {
        match &self.references {
            References::Offline => Ok(None),
            #[cfg(feature = "cli")]
            References::Online(host) => {
                let base =
                    host.base().await.map_err(|err| seam::Error::Internal(err.to_string()))?;
                Ok(Some(format!("{base}{}", ::slice::shelf::PATH)))
            }
            #[cfg(not(feature = "cli"))]
            References::Online => Err(seam::Error::Internal(
                "reference-listener-unavailable: the synthesis shelf needs the `cli` \
                 networking stack"
                    .to_string(),
            )),
        }
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
    ) -> Result<PhaseReport, seam::Error> {
        let ctx = self.ctx(&id, self.mcp_url(&id).await?).lending(workspace.root.clone());
        let inputs: Vec<aseam::Input> = inputs.into_iter().map(convert::narrow_input).collect();
        let context = convert::narrow_context(context);
        let workspace = convert::narrow_workspace(workspace);
        let report = self
            .catalog
            .build(&self.model, &ctx, &id, &slice, &inputs, &context, &workspace)
            .await
            .map_err(convert::error)?;
        Ok(convert::phase_report(report))
    }

    async fn verify(&self, id: String, workspace: Workspace) -> Result<PhaseReport, seam::Error> {
        let ctx = self.ctx(&id, self.mcp_url(&id).await?).lending(workspace.root.clone());
        let workspace = convert::narrow_workspace(workspace);
        let report = self
            .catalog
            .verify(&self.model, &ctx, &id, &workspace)
            .await
            .map_err(convert::error)?;
        Ok(convert::phase_report(report))
    }

    async fn repair(
        &self, id: String, slice: String, origin: RepairOrigin, findings: Vec<Diagnostic>,
        continuation: Option<Vec<u8>>, workspace: Workspace,
    ) -> Result<PhaseReport, seam::Error> {
        let ctx = self.ctx(&id, self.mcp_url(&id).await?).lending(workspace.root.clone());
        let origin = convert::narrow_origin(origin);
        let findings: Vec<aseam::PhaseFinding> =
            findings.into_iter().map(convert::narrow_finding).collect();
        let workspace = convert::narrow_workspace(workspace);
        let report = self
            .catalog
            .repair(
                &self.model,
                &ctx,
                &id,
                &slice,
                origin,
                &findings,
                continuation.as_deref(),
                &workspace,
            )
            .await
            .map_err(convert::error)?;
        Ok(convert::phase_report(report))
    }

    async fn review(
        &self, id: String, slice: String, continuation: Option<Vec<u8>>, workspace: Workspace,
    ) -> Result<PhaseReport, seam::Error> {
        let ctx = self.ctx(&id, self.mcp_url(&id).await?).lending(workspace.root.clone());
        let workspace = convert::narrow_workspace(workspace);
        let report = self
            .catalog
            .review(&self.model, &ctx, &id, &slice, continuation.as_deref(), &workspace)
            .await
            .map_err(convert::error)?;
        Ok(convert::phase_report(report))
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
    /// `.git` and a nested `.emery/change/` home) into the local
    /// snapshot store.
    async fn freeze(&self) -> Result<SnapshotId, seam::Error> {
        if self.paths.is_detached() {
            return Err(seam::Error::InvalidRequest(
                "target-base-freeze-detached: detached change home is not a product tree".into(),
            ));
        }
        self.store()
            .snapshot(self.paths.project_root())
            .await
            .map_err(|err| workspace_failure(&err))
    }

    async fn snapshot(&self, path: String) -> Result<SnapshotId, seam::Error> {
        self.store()
            .snapshot_path(std::path::Path::new(&path))
            .await
            .map_err(|err| workspace_failure(&err))
    }

    async fn contains(&self, id: SnapshotId) -> Result<bool, seam::Error> {
        Ok(self.store().contains(&id).await)
    }

    async fn prepare(&self, base: SnapshotId, writable: bool) -> Result<Workspace, seam::Error> {
        let prepared = workspace_kernel::prepare(
            &self.store(),
            self.workspaces_root(),
            &base,
            Access { writable },
        )
        .await
        .map_err(|err| workspace_failure(&err))?;
        // The build orchestrator attaches the per-attempt artifact
        // stage; preparation itself lends none.
        Ok(Workspace {
            id: prepared.id,
            root: prepared.root.display().to_string(),
            artifacts: host_absolute(self.paths.project_root()),
            artifact_stage: None,
        })
    }

    async fn capture(&self, id: String) -> Result<CodePatch, seam::Error> {
        workspace_kernel::capture(&self.store(), self.workspaces_root(), &id)
            .await
            .map_err(|err| workspace_failure(&err))
    }

    async fn compose(
        &self, base: SnapshotId, patches: Vec<CodePatch>,
    ) -> Result<CodePatch, seam::Error> {
        workspace_kernel::compose(&self.store(), &base, &patches)
            .await
            .map_err(|err| workspace_failure(&err))
    }

    async fn discard(&self, id: String) -> Result<(), seam::Error> {
        workspace_kernel::discard(self.workspaces_root(), &id)
            .map_err(|err| workspace_failure(&err))
    }

    async fn sweep(
        &self, dead: Vec<SnapshotId>, live: Vec<SnapshotId>,
    ) -> Result<usize, seam::Error> {
        self.store().sweep(&dead, &live).await.map_err(|err| workspace_failure(&err))
    }
}

/// Tree fetch runs in-process (RFC-95 `emery:vcs/trees`): host `git`
/// / HTTPS through the native VCS kernel, trees staged beneath the
/// staging root and reported as host paths.
impl seam::Trees for Provider {
    async fn fetch(
        &self, locator: String, credentials: seam::TreeCredentials, limits: seam::TreeLimits,
    ) -> Result<seam::TreeFetched, seam::TreeError> {
        let staging = self.paths.locations().staging_root();
        let fetched = project::vcs::fetch(staging, &locator, credentials, &limits)?;
        Ok(seam::TreeFetched {
            root: fetched.dir.display().to_string(),
            revision: fetched.revision,
        })
    }

    async fn discard_fetched(&self, root: String) -> Result<(), seam::TreeError> {
        let staging = self.paths.locations().staging_root();
        let name = std::path::Path::new(&root)
            .strip_prefix(staging)
            .ok()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                seam::TreeError::InvalidRequest(format!("`{root}` is not a staged tree"))
            })?;
        project::vcs::discard(staging, name)
    }
}

/// The D11 publication materialize runs in-process (RFC-95
/// `emery:vcs/worktree`): host `git` plus `Store::materialize` over
/// the deployment's publication slot root, honoring the in-place
/// candidate when the anchoring is not detached.
impl seam::Worktree for Provider {
    async fn export(
        &self, req: seam::WorktreeRequest,
    ) -> Result<(String, seam::WorktreeState), seam::WorktreeError> {
        if let Some(script) = &self.worktree {
            return (script.0)(&req);
        }
        let store = self.store();
        let env = project::vcs::worktree::ExportEnv {
            store: &store,
            publication_root: self.paths.locations().publication_root(),
            product_root: (!self.paths.is_detached()).then(|| self.paths.project_root()),
        };
        let (path, state) = project::vcs::worktree::export(&env, &req).await?;
        Ok((path.display().to_string(), state))
    }
}

/// The D10 forge read runs in-process (RFC-95 `emery:vcs/forge`):
/// GitHub REST with the launcher's token order, or the scripted
/// double in test suites.
impl seam::Forge for Provider {
    async fn find(
        &self, repository: String, branch: String,
    ) -> Result<Vec<seam::PullRequest>, seam::ForgeError> {
        if let Some(script) = &self.forge {
            return (script.0)(&repository, &branch);
        }
        let config = project::vcs::forge::Config::github();
        project::vcs::forge::find(&config, &repository, &branch)
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
