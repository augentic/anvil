//! WIT-backed capabilities used by workflow orchestrators; mappings
//! live here so engine code remains wasm-free. Compact build reports
//! are widened with caller-owned envelope fields before validation.

use std::future::Future;
use std::path::Path;
use std::sync::LazyLock;

use artifacts::evidence::AuthorityClass;
use diagnostics::{Diagnostic, Severity};
use error::Error;
use project::adapter::metadata::{Metadata, Request};
use project::adapter::{
    AdapterSelector, Axis, BuildInputDeclaration, PlatformsCapability, ResolvedSource,
    ResolvedTarget, Resolver,
};
use project::handler::{Anchor, ExecutionPaths, GUEST_WORKSPACES_MOUNT, PROJECT_ROOT_ENV};
use project::seam::wire::build_finding;
use project::seam::{
    self, BuildContext, Evidence, Input, Lead, MergePhase, Source, Target, Workspace,
};
use project::snapshot::{CodePatch, SnapshotId};
use slice::{BuildOutput, BuildReport, BuildStatus, UiSurface};

use crate::bindings::emery::adapter::{source, target, types};

/// Workflow capabilities backed by the world's WIT imports.
#[derive(Clone, Copy, Debug)]
pub struct Provider;

/// The guest's execution paths: the project-root mount preopen at
/// `.` with the store and cache preopens the deployment manifest
/// grants as the carried locations — no environment reads and no
/// project-id keying in-guest.
static PATHS: LazyLock<ExecutionPaths> = LazyLock::new(ExecutionPaths::guest);

impl omnia_guest::Model for Provider {}

impl Anchor for Provider {
    fn paths(&self) -> &ExecutionPaths {
        &PATHS
    }
}

impl Resolver for Provider {
    fn resolve_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        project::adapter::resolver::Component::new(metadata).resolve_source(selector, paths)
    }

    fn resolve_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        project::adapter::resolver::Component::new(metadata).resolve_target(selector, paths)
    }

    async fn ensure_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        project::adapter::ensure::source(metadata, selector, paths, jiff::Timestamp::now())
    }

    async fn ensure_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        project::adapter::ensure::target(metadata, selector, paths, jiff::Timestamp::now())
    }
}

impl Source for Provider {
    fn survey(&self, id: String) -> impl Future<Output = Result<Vec<Lead>, seam::Error>> + Send {
        async move {
            let leads = source::survey(id).await.map_err(map_error)?;
            Ok(leads.into_iter().map(map_lead).collect())
        }
    }

    fn extract(
        &self, id: String, lead: Lead,
    ) -> impl Future<Output = Result<Evidence, seam::Error>> + Send {
        async move {
            let wire = source::Lead {
                lead: lead.lead,
                synopsis: lead.synopsis,
                topics: lead.topics,
            };
            let evidence = source::extract(id, wire).await.map_err(map_error)?;
            Ok(Evidence {
                authority: map_authority(evidence.authority),
                claims: evidence.claims.into_iter().map(map_claim).collect(),
            })
        }
    }
}

impl Target for Provider {
    fn guidance(&self, id: String) -> impl Future<Output = Result<String, seam::Error>> + Send {
        async move { target::guidance(id).await.map_err(map_error) }
    }

    fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, context: BuildContext,
        workspace: Workspace,
    ) -> impl Future<Output = Result<BuildReport, seam::Error>> + Send {
        async move {
            let wire_inputs = inputs.into_iter().map(map_input).collect();
            let wire_context = target::BuildContext {
                sources: context.sources,
            };
            let wire_workspace = map_workspace(workspace);
            let report =
                target::build(id.clone(), slice.clone(), wire_inputs, wire_context, wire_workspace)
                    .await
                    .map_err(map_error)?;
            Ok(widen_report(&id, slice, report))
        }
    }

    fn merge(
        &self, id: String, slice: String, phase: MergePhase, workspace: Workspace,
    ) -> impl Future<Output = Result<BuildReport, seam::Error>> + Send {
        async move {
            let wire_phase = match phase {
                MergePhase::Preflight => target::MergePhase::Preflight,
                MergePhase::Postflight => target::MergePhase::Postflight,
            };
            let wire_workspace = map_workspace(workspace);
            let report = target::merge(id.clone(), slice.clone(), wire_phase, wire_workspace)
                .await
                .map_err(map_error)?;
            Ok(widen_report(&id, slice, report))
        }
    }
}

/// The in-guest workspace kernel: tree I/O over the `.` and
/// workspaces preopens, objects through `wasi:blobstore` (Omnia's
/// `BlobStore` capability), exec bits through `emery:exec-bits`.
impl seam::Workspaces for Provider {
    fn freeze(&self) -> impl Future<Output = Result<SnapshotId, seam::Error>> + Send {
        async move {
            let store = crate::workspace::store().await.map_err(|err| workspace_failure(&err))?;
            store.snapshot(PATHS.project_root()).await.map_err(|err| workspace_failure(&err))
        }
    }

    fn prepare(
        &self, base: SnapshotId, writable: bool,
    ) -> impl Future<Output = Result<Workspace, seam::Error>> + Send {
        async move {
            let store = crate::workspace::store().await.map_err(|err| workspace_failure(&err))?;
            let prepared = project::workspace::prepare(
                &store,
                Path::new(GUEST_WORKSPACES_MOUNT),
                &base,
                project::workspace::Access { writable },
            )
            .await
            .map_err(|err| workspace_failure(&err))?;
            Ok(Workspace {
                id: prepared.id,
                root: prepared.root.display().to_string(),
                artifacts: artifacts_root(),
            })
        }
    }

    fn capture(&self, id: String) -> impl Future<Output = Result<CodePatch, seam::Error>> + Send {
        async move {
            let store = crate::workspace::store().await.map_err(|err| workspace_failure(&err))?;
            project::workspace::capture(&store, Path::new(GUEST_WORKSPACES_MOUNT), &id)
                .await
                .map_err(|err| workspace_failure(&err))
        }
    }

    fn discard(&self, id: String) -> impl Future<Output = Result<(), seam::Error>> + Send {
        async move {
            project::workspace::discard(Path::new(GUEST_WORKSPACES_MOUNT), &id)
                .map_err(|err| workspace_failure(&err))
        }
    }

    fn apply(&self, patch: CodePatch) -> impl Future<Output = Result<(), seam::Error>> + Send {
        async move {
            let store = crate::workspace::store().await.map_err(|err| workspace_failure(&err))?;
            store.apply(&patch, PATHS.project_root()).await.map_err(|err| workspace_failure(&err))
        }
    }

    fn sweep(
        &self, dead: Vec<SnapshotId>, live: Vec<SnapshotId>,
    ) -> impl Future<Output = Result<usize, seam::Error>> + Send {
        async move {
            let store = crate::workspace::store().await.map_err(|err| workspace_failure(&err))?;
            store.sweep(&dead, &live).await.map_err(|err| workspace_failure(&err))
        }
    }
}

/// Map a workspace-kernel failure onto the seam error contract.
fn workspace_failure(err: &Error) -> seam::Error {
    seam::Error::Internal(err.to_string())
}

/// The agent-visible artifact root: the host-absolute project root
/// the launcher exports as [`PROJECT_ROOT_ENV`] (guests inherit the
/// host environment), so a spawned agent working inside a lent
/// workspace can still read change-tree artifacts. The deployment
/// always sets it; the `.` fallback keeps ad-hoc harnesses running.
fn artifacts_root() -> String {
    std::env::var(PROJECT_ROOT_ENV).unwrap_or_else(|_absent| ".".to_string())
}

fn map_workspace(workspace: Workspace) -> target::Workspace {
    target::Workspace {
        id: workspace.id,
        root: workspace.root,
        artifacts: workspace.artifacts,
    }
}

/// Resolve metadata through the deployed adapter identified by the request.
///
/// Dispatch is by adapter id rather than component path; deployment
/// assembly uses the same resolver precedence.
///
/// # Errors
///
/// Reserved for the resolver callback contract; WIT metadata has no
/// error channel.
pub fn metadata(request: &Request<'_>) -> Result<Metadata, Error> {
    Ok(match request.axis {
        Axis::Source => {
            let record = source::metadata(request.adapter_id);
            Metadata {
                emery_floor: record.emery_floor,
                inputs: Vec::new(),
                platforms: None,
            }
        }
        Axis::Target => {
            let record = target::metadata(request.adapter_id);
            Metadata {
                emery_floor: record.emery_floor,
                inputs: record
                    .inputs
                    .into_iter()
                    .map(|input| BuildInputDeclaration {
                        path: input.path,
                        required: input.required,
                    })
                    .collect(),
                platforms: record.platforms.map(|capability| PlatformsCapability {
                    required: capability.required,
                    allowed: capability.allowed.into_iter().map(map_platform).collect(),
                    default: capability.default.into_iter().map(map_platform).collect(),
                }),
            }
        }
    })
}

fn map_error(error: types::Error) -> seam::Error {
    match error {
        types::Error::InvalidRequest(detail) => seam::Error::InvalidRequest(detail),
        types::Error::Io(detail) => seam::Error::Io(detail),
        types::Error::Internal(detail) => seam::Error::Internal(detail),
    }
}

fn map_lead(lead: source::Lead) -> Lead {
    Lead {
        lead: lead.lead,
        synopsis: lead.synopsis,
        topics: lead.topics,
    }
}

const fn map_authority(authority: source::Authority) -> AuthorityClass {
    match authority {
        source::Authority::Intent => AuthorityClass::Intent,
        source::Authority::Documentation => AuthorityClass::Documentation,
        source::Authority::Behaviour => AuthorityClass::Behaviour,
    }
}

/// Map a WIT claim record onto the typed [`artifacts::evidence::Claim`].
///
/// The backing variant flattens onto the wire shape's `payload` /
/// `backing-path` keys via [`artifacts::evidence::Claim::set_backing`].
fn map_claim(claim: source::Claim) -> artifacts::evidence::Claim {
    let mut typed = artifacts::evidence::Claim::new(map_claim_kind(claim.kind));
    typed.id = claim.id;
    typed.path = claim.path;
    typed.synopsis = claim.synopsis;
    typed.set_backing(claim.backing.map(|backing| match backing {
        source::Backing::Payload(payload) => artifacts::evidence::Backing::Payload(payload),
        source::Backing::Path(path) => artifacts::evidence::Backing::Path(path),
    }));
    typed
}

const fn map_claim_kind(kind: source::ClaimKind) -> artifacts::evidence::ClaimKind {
    use artifacts::evidence::ClaimKind;
    match kind {
        source::ClaimKind::Intent => ClaimKind::Intent,
        source::ClaimKind::Requirement => ClaimKind::Requirement,
        source::ClaimKind::Criterion => ClaimKind::Criterion,
        source::ClaimKind::Decision => ClaimKind::Decision,
        source::ClaimKind::Section => ClaimKind::Section,
        source::ClaimKind::Diagram => ClaimKind::Diagram,
        source::ClaimKind::Contract => ClaimKind::Contract,
        source::ClaimKind::Example => ClaimKind::Example,
        source::ClaimKind::Excerpt => ClaimKind::Excerpt,
        source::ClaimKind::Type => ClaimKind::Type,
        source::ClaimKind::Call => ClaimKind::Call,
        source::ClaimKind::Region => ClaimKind::Region,
        source::ClaimKind::Container => ClaimKind::Container,
        source::ClaimKind::Leaf => ClaimKind::Leaf,
    }
}

fn map_input(input: Input) -> target::Input {
    let payload = |body: seam::Payload| match body {
        seam::Payload::Path(path) => target::Payload::Path(path),
        seam::Payload::Body(text) => target::Payload::Body(text),
    };
    match input {
        Input::Proposal(body) => target::Input::Proposal(payload(body)),
        Input::Design(body) => target::Input::Design(payload(body)),
        Input::Tasks(body) => target::Input::Tasks(payload(body)),
        Input::Spec(body) => target::Input::Spec(payload(body)),
        Input::Other(body) => target::Input::Other(payload(body)),
    }
}

/// Add caller-owned envelope fields required by the build-report schema.
fn widen_report(id: &str, slice: String, report: target::Report) -> BuildReport {
    BuildReport::stamped(
        id,
        slice,
        match report.status {
            target::Status::Success => BuildStatus::Success,
            target::Status::Failure => BuildStatus::Failure,
        },
        report.findings.into_iter().map(widen_finding).collect(),
        report
            .outputs
            .into_iter()
            .map(|output| BuildOutput {
                platform: map_platform(output.platform),
                path: output.path,
            })
            .collect(),
        report.ui_surface.map(|surface| UiSurface {
            screens: surface.screens,
        }),
        report.covered,
    )
}

fn widen_finding(finding: target::Finding) -> Diagnostic {
    build_finding(finding.rule_id, finding.detail, map_severity(finding.severity))
}

const fn map_severity(severity: target::Severity) -> Severity {
    match severity {
        target::Severity::Critical => Severity::Critical,
        target::Severity::Important => Severity::Important,
        target::Severity::Suggestion => Severity::Suggestion,
        target::Severity::Optional => Severity::Optional,
    }
}

const fn map_platform(platform: target::Platform) -> project::platform::Platform {
    use project::platform::Platform;
    match platform {
        target::Platform::Core => Platform::Core,
        target::Platform::Ios => Platform::Ios,
        target::Platform::Android => Platform::Android,
        target::Platform::Web => Platform::Web,
        target::Platform::Desktop => Platform::Desktop,
    }
}
