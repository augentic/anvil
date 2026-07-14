//! The unified capability provider the integration suites run against.
//!
//! [`Provider`] carries every capability the workflow operations bind
//! (`Anchor + Model + Resolver + Hydrator + SourceSeam + TargetSeam`),
//! with adapter behaviour delegated to the shared
//! [`adapter`] core and judgment delegated to the
//! configured model backend.
//!
//! The seam mapping layer between the fixture core's WIT-mirroring
//! types and the workflow seam DTOs lives here in full, deliberately
//! shaped like the guest shim's WIT mapping (`src/provider.rs` at the
//! repo root): the same claim-JSON projection (`payload` /
//! `backing-path` keys) and the same `BuildReport` widening. Nothing
//! mechanical pins the two projections together — the WASM boundary
//! smoke exercises the real shim path over the same fixture component,
//! so drift between them surfaces there rather than here.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use artifacts::evidence::AuthorityClass;
use error::Error;
use omnia_guest::Model;
use omnia_guest::api::invocation::Invocation;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::api::operation::Operation;
use omnia_guest::model::{Reply, Request};
use omnia_testkit::model::Scripted as ScriptedModel;
use project::adapter::metadata::Metadata;
use project::adapter::{
    AdapterRef, Origin, PlatformsCapability, ResolvedSource, ResolvedTarget, Resolver,
};
use project::platform::Platform;
use project::seam::{self, Evidence, Input, Lead, MergePhase, SourceSeam, TargetSeam, WorkingTree};
use slice::{BUILD_VERSION, BuildOutput, BuildReport, BuildStatus};

use crate::adapter;
use crate::env::{CacheGuard, scoped_cache};

/// The provider shape the scripted suites run against: the fixture
/// adapter behind the seams, `omnia-testkit`'s FIFO script behind the
/// judgment legs.
pub type Scripted = Provider<ScriptedModel>;

// How the provider answers adapter resolution.
#[derive(Clone, Copy, Debug)]
enum Resolution {
    // The shipped `resolver::Component` with the deterministic metadata
    // runner ([`resolver`]) — file probing intact, for the init /
    // resolve / store suites.
    Component,
    // Direct fixture identities — no component file anywhere, for the
    // workflow suites bound to `fixture*` adapters.
    Direct,
}

/// A throw-away project tree plus the full capability set the workflow
/// operations bind.
///
/// Clones share the model backend and the seam call log, so [`run`]
/// can mint a fresh invoker per operation while the suite keeps
/// asserting against its original handle.
#[expect(
    clippy::partial_pub_fields,
    reason = "tests read `root` directly; the strategy, log, and lifetime guards are implementation detail"
)]
pub struct Provider<M> {
    /// The project root every project-scoped verb anchors at.
    pub root: PathBuf,
    model: M,
    resolution: Resolution,
    // Recorded seam dispatches (`<operation> <adapter-id>`), in call
    // order — the observable proof that fixture guidance / build ran.
    calls: Arc<Mutex<Vec<String>>>,
    // Owned tempdir + env pinning for the constructors that mint their
    // own tree; `None` when anchored at a caller-owned root.
    owned: Option<Arc<Owned>>,
}

struct Owned {
    _cache: CacheGuard,
    _tmp: tempfile::TempDir,
}

impl<M: Clone> Clone for Provider<M> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            model: self.model.clone(),
            resolution: self.resolution,
            calls: Arc::clone(&self.calls),
            owned: self.owned.clone(),
        }
    }
}

impl<M> std::fmt::Debug for Provider<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Provider").field("root", &self.root).finish_non_exhaustive()
    }
}

impl Provider<ScriptedModel> {
    /// A bare directory — nothing scaffolded (the scaffold-leg input):
    /// owned tempdir, pinned cache, current directory entered, the
    /// shipped component resolver behind adapter resolution, and an
    /// empty model script.
    ///
    /// # Panics
    ///
    /// Panics when the tempdir cannot be created or entered.
    #[must_use]
    pub fn bare() -> Self {
        let (tmp, root, cache) = owned_tree();
        std::env::set_current_dir(&root).expect("enter project root");
        Self {
            root,
            model: ScriptedModel::answers(Vec::<String>::new()),
            resolution: Resolution::Component,
            calls: Arc::new(Mutex::new(Vec::new())),
            owned: Some(Arc::new(Owned {
                _cache: cache,
                _tmp: tmp,
            })),
        }
    }

    /// An initialised project (`.specify/project.yaml` present).
    ///
    /// # Panics
    ///
    /// Panics when the tempdir or project scaffold cannot be written.
    #[must_use]
    pub fn initialised() -> Self {
        let provider = Self::bare();
        write_project_yaml(&provider.root, "demo");
        provider
    }

    /// A minimal initialised project bound to `target_adapter`, with
    /// the fixture adapter behind the seams and `answers` behind the
    /// judgment legs. The suites that go through the `Init` operation
    /// instead anchor at their own tree via [`Provider::scripted_at`].
    ///
    /// # Panics
    ///
    /// Panics when the tempdir or project scaffold cannot be written.
    #[must_use]
    pub fn scripted(target_adapter: &str, answers: Vec<String>) -> Self {
        let (tmp, root, cache) = owned_tree();
        write_project_yaml(&root, target_adapter);
        let mut provider = Self::scripted_at(&root, answers);
        provider.owned = Some(Arc::new(Owned {
            _cache: cache,
            _tmp: tmp,
        }));
        provider
    }

    /// [`Provider::scripted`] over an owned bare tree — nothing
    /// scaffolded, for suites whose first operation is `Init`.
    ///
    /// # Panics
    ///
    /// Panics when the tempdir cannot be created.
    #[must_use]
    pub fn scripted_bare(answers: Vec<String>) -> Self {
        let (tmp, root, cache) = owned_tree();
        let mut provider = Self::scripted_at(&root, answers);
        provider.owned = Some(Arc::new(Owned {
            _cache: cache,
            _tmp: tmp,
        }));
        provider
    }

    /// A scripted fixture provider anchored at a caller-owned `root`.
    #[must_use]
    pub fn scripted_at(root: &Path, answers: Vec<String>) -> Self {
        Self::new(root, ScriptedModel::answers(answers))
    }
}

impl<M> Provider<M> {
    /// A fixture provider anchored at a caller-owned `root` over the
    /// given model backend.
    pub fn new(root: impl Into<PathBuf>, model: M) -> Self {
        Self {
            root: root.into(),
            model,
            resolution: Resolution::Direct,
            calls: Arc::new(Mutex::new(Vec::new())),
            owned: None,
        }
    }

    /// The configured model backend — read access for suites that
    /// assert the script drained (`assert_exhausted`).
    pub const fn model(&self) -> &M {
        &self.model
    }

    /// The recorded seam dispatches, in call order.
    ///
    /// # Panics
    ///
    /// Panics when the call-log lock is poisoned (never in practice).
    #[must_use]
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("the call log is never poisoned").clone()
    }

    fn record(&self, operation: &str, id: &str) {
        self.calls
            .lock()
            .expect("the call log is never poisoned")
            .push(format!("{operation} {id}"));
    }
}

// A fresh tempdir with the out-of-tree project cache pinned inside it,
// so adapter cache writes are hermetic and auto-cleaned.
fn owned_tree() -> (tempfile::TempDir, PathBuf, CacheGuard) {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical tempdir");
    let cache = scoped_cache(&root);
    (tmp, root, cache)
}

fn write_project_yaml(root: &Path, target_adapter: &str) {
    std::fs::create_dir_all(root.join(".specify")).expect("mkdir .specify");
    std::fs::write(
        root.join(".specify/project.yaml"),
        format!("name: demo\nadapter: {target_adapter}\nrules: {{}}\n"),
    )
    .expect("write project.yaml");
}

/// Invoke one operation against the provider. The operation type
/// leads the generics so call sites write `run::<Op, _>(&provider, …)`.
///
/// # Errors
///
/// Propagates the operation's typed failure.
pub async fn run<R, B, M>(
    provider: &Provider<M>, input: R::Input,
) -> Result<B, project::handler::Error>
where
    M: Clone + Send + Sync + 'static,
    R: Operation<Provider<M>, Output = B, Error = project::handler::Error>,
    B: Send,
{
    Invoker::new("specify", provider.clone()).invoke::<R>(Invocation::new(input)).await
}

/// Rule ids carried by a failing validate operation's report.
///
/// # Panics
///
/// Panics when `err` is not a report-carrying failure.
#[must_use]
pub fn report_rule_ids(err: &project::handler::Error) -> Vec<String> {
    let project::handler::Error::Report { body, .. } = err else {
        panic!("expected report error, got {err:?}");
    };
    body.report().findings.iter().filter_map(|finding| finding.rule_id.clone()).collect()
}

/// The shipped component resolver with the deterministic
/// [`adapter::metadata_json`] answers behind its metadata runner —
/// file probing intact for the resolve / install / store suites.
#[must_use]
pub fn resolver() -> project::adapter::resolver::Component {
    fn stub(request: &project::adapter::metadata::Request<'_>) -> Result<Metadata, Error> {
        parse_metadata(request.adapter_id)
    }

    project::adapter::resolver::Component::new(stub)
}

// The one id-keyed metadata convention, parsed into the typed resolver
// metadata.
fn parse_metadata(adapter_id: &str) -> Result<Metadata, Error> {
    serde_json::from_str(&adapter::metadata_json(adapter_id)).map_err(|err| Error::Diag {
        code: "adapter-metadata-failed",
        detail: format!("fixture metadata parse {adapter_id}: {err}"),
    })
}

impl<M: Send + Sync + 'static> project::handler::Anchor for Provider<M> {
    fn project_root(&self) -> &Path {
        &self.root
    }
}

impl<M: Send + Sync> Resolver for Provider<M> {
    fn resolve_source(
        &self, adapter_ref: &AdapterRef, project_dir: &Path,
    ) -> Result<ResolvedSource, Error> {
        match self.resolution {
            Resolution::Component => {
                Resolver::resolve_source(&resolver(), adapter_ref, project_dir)
            }
            Resolution::Direct => project::adapter::resolver::source(
                adapter_ref,
                parse_metadata(&format!("source:{}", adapter_ref.name))?,
                origin(adapter_ref),
            ),
        }
    }

    fn resolve_target(
        &self, adapter_ref: &AdapterRef, project_dir: &Path,
    ) -> Result<ResolvedTarget, Error> {
        match self.resolution {
            Resolution::Component => {
                Resolver::resolve_target(&resolver(), adapter_ref, project_dir)
            }
            Resolution::Direct => project::adapter::resolver::target(
                adapter_ref,
                parse_metadata(&format!("target:{}", adapter_ref.name))?,
                origin(adapter_ref),
            ),
        }
    }
}

// A file-backed registry: a test stages the expected component bytes
// at `<root>/hydrator/<name>@<version>.wasm` and the fetch serves
// them; an unstaged URL refuses, standing in for a fetch failure.
impl<M: Send + Sync> project::adapter::Hydrator for Provider<M> {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, Error> {
        let entry = url.rsplit('/').next().unwrap_or_default();
        let staged = self.root.join("hydrator").join(entry);
        std::fs::read(&staged).map_err(|err| Error::Diag {
            code: "http-fetch",
            detail: format!("no staged registry response for {url}: {err}"),
        })
    }
}

impl<M: Model> Model for Provider<M> {
    async fn create(&self, request: Request) -> Result<Reply, omnia_guest::model::Error> {
        self.model.create(request).await
    }
}

impl<M: Model> SourceSeam for Provider<M> {
    async fn survey(&self, id: String) -> Result<Vec<Lead>, seam::Error> {
        self.record("survey", &id);
        let leads = adapter::survey(&id).map_err(map_error)?;
        Ok(leads.into_iter().map(map_lead).collect())
    }

    async fn extract(&self, id: String, lead: Lead) -> Result<Evidence, seam::Error> {
        self.record("extract", &id);
        let fixture_lead = adapter::Lead {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        };
        let evidence = adapter::extract(&id, &fixture_lead).map_err(map_error)?;
        Ok(Evidence {
            authority: map_authority(evidence.authority),
            claims: evidence.claims.iter().map(map_claim).collect(),
        })
    }
}

impl<M: Model> TargetSeam for Provider<M> {
    async fn guidance(&self, id: String) -> Result<String, seam::Error> {
        self.record("guidance", &id);
        adapter::guidance(&id).map_err(map_error)
    }

    async fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, _tree: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        self.record("build", &id);
        let inputs: Vec<adapter::Input> = inputs.into_iter().map(map_input).collect();
        let report = adapter::build(&self.root, &id, &slice, &inputs).map_err(map_error)?;
        Ok(widen_report(&id, slice, report))
    }

    async fn merge(
        &self, id: String, slice: String, phase: MergePhase, _tree: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        self.record(&format!("merge-{phase}"), &id);
        let fixture_phase = match phase {
            MergePhase::Preflight => adapter::MergePhase::Preflight,
            MergePhase::Postflight => adapter::MergePhase::Postflight,
        };
        let report = adapter::merge(&self.root, &id, &slice, fixture_phase).map_err(map_error)?;
        Ok(widen_report(&id, slice, report))
    }
}

/// The resolution origin every fixture identity reports.
fn origin(adapter_ref: &AdapterRef) -> Origin {
    Origin {
        label: "fixture".to_string(),
        reference: format!("fixture:{}", adapter_ref.name),
    }
}

/// Harness adapter [`adapter::PlatformsCapability`] → the resolver metadata
/// capability — the same widening the guest shim applies.
#[must_use]
pub fn map_platforms(capability: adapter::PlatformsCapability) -> PlatformsCapability {
    PlatformsCapability {
        required: capability.required,
        allowed: capability.allowed.into_iter().map(map_platform).collect(),
        default: capability.default.into_iter().map(map_platform).collect(),
    }
}

/// Harness adapter [`adapter::Platform`] → the workflow [`Platform`] enum.
const fn map_platform(platform: adapter::Platform) -> Platform {
    match platform {
        adapter::Platform::Core => Platform::Core,
        adapter::Platform::Ios => Platform::Ios,
        adapter::Platform::Android => Platform::Android,
    }
}

/// Harness adapter error → the workflow seam's typed failure
/// (variant-for-variant; both mirror the WIT `types.error`).
fn map_error(error: adapter::Error) -> seam::Error {
    match error {
        adapter::Error::InvalidRequest(detail) => seam::Error::InvalidRequest(detail),
        adapter::Error::Io(detail) => seam::Error::Io(detail),
        adapter::Error::Internal(detail) => seam::Error::Internal(detail),
    }
}

/// Harness adapter [`adapter::Lead`] → the workflow seam's [`Lead`].
fn map_lead(lead: adapter::Lead) -> Lead {
    Lead {
        lead: lead.lead,
        synopsis: lead.synopsis,
        topics: lead.topics,
    }
}

/// Harness adapter [`adapter::Authority`] → the document-level
/// [`AuthorityClass`].
const fn map_authority(authority: adapter::Authority) -> AuthorityClass {
    match authority {
        adapter::Authority::Intent => AuthorityClass::Intent,
        adapter::Authority::Documentation => AuthorityClass::Documentation,
        adapter::Authority::Behaviour => AuthorityClass::Behaviour,
    }
}

/// Harness adapter [`adapter::Claim`] → the typed claim the composed
/// Evidence document carries — the same mapping the guest shim
/// applies to the WIT claim record (`payload` for an inline payload,
/// `backing-path` for a filesystem pointer).
fn map_claim(claim: &adapter::Claim) -> artifacts::evidence::Claim {
    let mut typed = artifacts::evidence::Claim::new(map_claim_kind(claim.kind));
    typed.id.clone_from(&claim.id);
    typed.path.clone_from(&claim.path);
    typed.synopsis.clone_from(&claim.synopsis);
    typed.set_backing(claim.backing.clone().map(|backing| match backing {
        adapter::Backing::Payload(payload) => artifacts::evidence::Backing::Payload(payload),
        adapter::Backing::Path(path) => artifacts::evidence::Backing::Path(path),
    }));
    typed
}

/// The closed claim-kind subset the fixture emits.
const fn map_claim_kind(kind: adapter::ClaimKind) -> artifacts::evidence::ClaimKind {
    match kind {
        adapter::ClaimKind::Requirement => artifacts::evidence::ClaimKind::Requirement,
        adapter::ClaimKind::Criterion => artifacts::evidence::ClaimKind::Criterion,
        adapter::ClaimKind::Section => artifacts::evidence::ClaimKind::Section,
    }
}

/// Workflow seam [`Input`] → harness adapter [`adapter::Input`].
fn map_input(input: Input) -> adapter::Input {
    match input {
        Input::Proposal(body) => adapter::Input::Proposal(body),
        Input::Design(body) => adapter::Input::Design(body),
        Input::Tasks(body) => adapter::Input::Tasks(body),
        Input::Spec(body) => adapter::Input::Spec(body),
        Input::Other(body) => adapter::Input::Other(body),
    }
}

/// Widen the compact harness-adapter [`adapter::Report`] into the canonical
/// [`BuildReport`] wire shape the orchestrator's finalize tail
/// schema-gates — the same envelope stamping the guest shim applies.
fn widen_report(id: &str, slice: String, report: adapter::Report) -> BuildReport {
    BuildReport {
        version: BUILD_VERSION,
        slice,
        target: id.strip_prefix("target:").unwrap_or(id).to_string(),
        status: match report.status {
            adapter::Status::Success => BuildStatus::Success,
            adapter::Status::Failure => BuildStatus::Failure,
        },
        findings: Vec::new(),
        outputs: report
            .outputs
            .into_iter()
            .map(|output| BuildOutput {
                platform: Platform::Core,
                path: output.path,
            })
            .collect(),
        ui_surface: None,
    }
}
