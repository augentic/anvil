//! The native fixture provider: every capability the workflow
//! operations bind (`Anchor + Model + Resolver + SourceSeam +
//! TargetSeam`), with adapter behaviour delegated to the shared
//! [`fixtures`] core and judgment delegated to a scripted
//! `omnia-testkit` model.
//!
//! The mapping layer between the fixture core's WIT-mirroring types
//! and the workflow seam DTOs lives here in full, deliberately shaped
//! like the guest shim's WIT mapping (`src/provider.rs` at the repo
//! root): the same claim-JSON projection (`payload` / `backing-path`
//! keys) and the same `BuildReport` widening. Nothing mechanical pins
//! the two projections together — the WASM boundary smoke exercises
//! the real shim path over the same fixture component, so drift
//! between them surfaces there rather than here.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use artifacts::evidence::AuthorityClass;
use error::Error;
use omnia_guest::Model;
use omnia_guest::api::invocation::Invocation;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::api::operation::Operation;
use omnia_guest::model::{Reply, Request};
use omnia_testkit::model::{Harness, Scripted};
use project::adapter::metadata::Metadata;
use project::adapter::{
    AdapterRef, Origin, PlatformsCapability, ResolvedSource, ResolvedTarget, Resolver,
};
use project::platform::Platform;
use project::seam::{self, Evidence, Input, Lead, MergePhase, SourceSeam, TargetSeam, WorkingTree};
use slice::{BUILD_VERSION, BuildOutput, BuildReport, BuildStatus};

/// The provider shape every scripted suite runs against.
pub type ScriptedProvider = FixtureProvider<Harness<Scripted>>;

/// An invoker over a scripted fixture provider anchored at `root`.
pub fn scripted_invoker(root: &Path, answers: Vec<String>) -> Invoker<ScriptedProvider> {
    Invoker::new("specify", FixtureProvider::new(root, Harness::new(Scripted::answers(answers))))
}

/// Invoke one operation against the scripted fixture provider.
pub async fn run<R, B>(
    invoker: &Invoker<ScriptedProvider>, input: R::Input,
) -> Result<B, project::handler::Error>
where
    R: Operation<ScriptedProvider, Output = B, Error = project::handler::Error>,
    B: Send,
{
    invoker.invoke::<R>(Invocation::new(input)).await
}

/// A minimal initialised project bound to `target_adapter`, with the
/// out-of-tree cache pinned inside the tempdir. The suites that go
/// through the `Init` operation instead create their own tree.
pub fn scripted_project(target_adapter: &str) -> (tempfile::TempDir, PathBuf, super::CacheGuard) {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical tempdir");
    let cache = super::scoped_cache(&root);
    std::fs::create_dir_all(root.join(".specify")).expect("mkdir .specify");
    std::fs::write(
        root.join(".specify/project.yaml"),
        format!("name: demo\nadapter: {target_adapter}\nrules: {{}}\n"),
    )
    .expect("write project.yaml");
    (tmp, root, cache)
}

/// The native fixture provider over a scripted (or live) model
/// backend, anchored at one throw-away project root.
#[derive(Debug)]
pub struct FixtureProvider<M> {
    /// The project root every project-scoped verb anchors at.
    project_dir: PathBuf,
    /// The judgment backend behind the engine's own legs.
    model: M,
    /// Recorded seam dispatches (`<operation> <adapter-id>`), in call
    /// order — the observable proof that fixture guidance / build ran.
    calls: Mutex<Vec<String>>,
}

impl<M> FixtureProvider<M> {
    /// A provider anchored at `project_dir` over the given model.
    pub fn new(project_dir: impl Into<PathBuf>, model: M) -> Self {
        Self {
            project_dir: project_dir.into(),
            model,
            calls: Mutex::new(Vec::new()),
        }
    }

    /// The configured model backend — read access for suites that
    /// assert on a scripted mock's recorded requests.
    pub const fn model(&self) -> &M {
        &self.model
    }

    /// The recorded seam dispatches, in call order.
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

impl<M: Send + Sync + 'static> project::handler::Anchor for FixtureProvider<M> {
    fn project_root(&self) -> &Path {
        &self.project_dir
    }
}

impl<M: Send + Sync> Resolver for FixtureProvider<M> {
    fn resolve_source(
        &self, adapter_ref: &AdapterRef, _project_dir: &Path,
    ) -> Result<ResolvedSource, Error> {
        project::adapter::resolver::source(adapter_ref, Metadata::default(), origin(adapter_ref))
    }

    fn resolve_target(
        &self, adapter_ref: &AdapterRef, _project_dir: &Path,
    ) -> Result<ResolvedTarget, Error> {
        let metadata = Metadata {
            platforms: fixtures::target_platforms(&adapter_ref.name).map(map_platforms),
            ..Metadata::default()
        };
        project::adapter::resolver::target(adapter_ref, metadata, origin(adapter_ref))
    }
}

impl<M: Send + Sync> project::adapter::Hydrator for FixtureProvider<M> {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, Error> {
        Err(Error::Diag {
            code: "adapter-hydrate-unavailable",
            detail: format!("the fixture provider links adapters directly (requested {url})"),
        })
    }
}

impl<M: Model> Model for FixtureProvider<M> {
    async fn create(&self, request: Request) -> Result<Reply, omnia_guest::model::Error> {
        self.model.create(request).await
    }
}

impl<M: Model> SourceSeam for FixtureProvider<M> {
    async fn survey(&self, id: String) -> Result<Vec<Lead>, seam::Error> {
        self.record("survey", &id);
        let leads = fixtures::survey(&id).map_err(map_error)?;
        Ok(leads.into_iter().map(map_lead).collect())
    }

    async fn extract(&self, id: String, lead: Lead) -> Result<Evidence, seam::Error> {
        self.record("extract", &id);
        let fixture_lead = fixtures::Lead {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        };
        let evidence = fixtures::extract(&id, &fixture_lead).map_err(map_error)?;
        Ok(Evidence {
            authority: map_authority(evidence.authority),
            claims: evidence.claims.iter().map(claim_json).collect(),
        })
    }
}

impl<M: Model> TargetSeam for FixtureProvider<M> {
    async fn guidance(&self, id: String) -> Result<String, seam::Error> {
        self.record("guidance", &id);
        fixtures::guidance(&id).map_err(map_error)
    }

    async fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, _tree: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        self.record("build", &id);
        let inputs: Vec<fixtures::Input> = inputs.into_iter().map(map_input).collect();
        let report = fixtures::build(&self.project_dir, &id, &slice, &inputs).map_err(map_error)?;
        Ok(widen_report(&id, slice, report))
    }

    async fn merge(
        &self, id: String, slice: String, phase: MergePhase, _tree: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        self.record(&format!("merge-{phase}"), &id);
        let fixture_phase = match phase {
            MergePhase::Preflight => fixtures::MergePhase::Preflight,
            MergePhase::Postflight => fixtures::MergePhase::Postflight,
        };
        let report =
            fixtures::merge(&self.project_dir, &id, &slice, fixture_phase).map_err(map_error)?;
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

/// Fixture [`fixtures::PlatformsCapability`] → the resolver metadata
/// capability — the same widening the guest shim applies.
fn map_platforms(capability: fixtures::PlatformsCapability) -> PlatformsCapability {
    PlatformsCapability {
        required: capability.required,
        allowed: capability.allowed.into_iter().map(map_platform).collect(),
        default: capability.default.into_iter().map(map_platform).collect(),
    }
}

/// Fixture [`fixtures::Platform`] → the workflow [`Platform`] enum.
const fn map_platform(platform: fixtures::Platform) -> Platform {
    match platform {
        fixtures::Platform::Core => Platform::Core,
        fixtures::Platform::Ios => Platform::Ios,
        fixtures::Platform::Android => Platform::Android,
    }
}

/// Fixture error → the workflow seam's typed failure
/// (variant-for-variant; both mirror the WIT `types.error`).
fn map_error(error: fixtures::Error) -> seam::Error {
    match error {
        fixtures::Error::InvalidRequest(detail) => seam::Error::InvalidRequest(detail),
        fixtures::Error::Io(detail) => seam::Error::Io(detail),
        fixtures::Error::Internal(detail) => seam::Error::Internal(detail),
    }
}

/// Fixture [`fixtures::Lead`] → the workflow seam's [`Lead`].
fn map_lead(lead: fixtures::Lead) -> Lead {
    Lead {
        lead: lead.lead,
        synopsis: lead.synopsis,
        topics: lead.topics,
    }
}

/// Fixture [`fixtures::Authority`] → the document-level
/// [`AuthorityClass`].
const fn map_authority(authority: fixtures::Authority) -> AuthorityClass {
    match authority {
        fixtures::Authority::Intent => AuthorityClass::Intent,
        fixtures::Authority::Documentation => AuthorityClass::Documentation,
        fixtures::Authority::Behaviour => AuthorityClass::Behaviour,
    }
}

/// Fixture [`fixtures::Claim`] → the open claim JSON object the
/// composed Evidence document carries — the same projection the guest
/// shim applies to the WIT claim record (`payload` for an inline
/// payload, `backing-path` for a filesystem pointer).
fn claim_json(claim: &fixtures::Claim) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    object.insert("kind".into(), claim_kind_str(claim.kind).into());
    if let Some(id) = &claim.id {
        object.insert("id".into(), id.clone().into());
    }
    if let Some(path) = &claim.path {
        object.insert("path".into(), path.clone().into());
    }
    if let Some(synopsis) = &claim.synopsis {
        object.insert("synopsis".into(), synopsis.clone().into());
    }
    match &claim.backing {
        Some(fixtures::Backing::Payload(payload)) => {
            object.insert("payload".into(), payload.clone().into());
        }
        Some(fixtures::Backing::Path(path)) => {
            object.insert("backing-path".into(), path.clone().into());
        }
        None => {}
    }
    serde_json::Value::Object(object)
}

/// The closed claim-kind enum's schema token.
const fn claim_kind_str(kind: fixtures::ClaimKind) -> &'static str {
    match kind {
        fixtures::ClaimKind::Requirement => "requirement",
        fixtures::ClaimKind::Criterion => "criterion",
        fixtures::ClaimKind::Section => "section",
    }
}

/// Workflow seam [`Input`] → fixture [`fixtures::Input`].
fn map_input(input: Input) -> fixtures::Input {
    match input {
        Input::Proposal(body) => fixtures::Input::Proposal(body),
        Input::Design(body) => fixtures::Input::Design(body),
        Input::Tasks(body) => fixtures::Input::Tasks(body),
        Input::Spec(body) => fixtures::Input::Spec(body),
        Input::Other(body) => fixtures::Input::Other(body),
    }
}

/// Widen the compact fixture [`fixtures::Report`] into the canonical
/// [`BuildReport`] wire shape the orchestrator's finalize tail
/// schema-gates — the same envelope stamping the guest shim applies.
fn widen_report(id: &str, slice: String, report: fixtures::Report) -> BuildReport {
    BuildReport {
        version: BUILD_VERSION,
        slice,
        target: id.strip_prefix("target:").unwrap_or(id).to_string(),
        status: match report.status {
            fixtures::Status::Success => BuildStatus::Success,
            fixtures::Status::Failure => BuildStatus::Failure,
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
