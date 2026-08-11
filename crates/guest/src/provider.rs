//! WIT-backed capabilities used by workflow orchestrators; mappings
//! live here so engine code remains wasm-free. Compact build reports
//! are widened with caller-owned envelope fields before validation.

use std::future::Future;
use std::path::Path;
use std::sync::LazyLock;

use artifacts::evidence::AuthorityClass;
use diagnostics::{Artifact, Confidence, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use error::Error;
use project::adapter::metadata::{Metadata, Request};
use project::adapter::{
    AdapterSelector, Axis, BuildInputDeclaration, PlatformsCapability, ResolvedSource,
    ResolvedTarget, Resolver, WritableArtifactDeclaration, WritableArtifactKind,
};
use project::handler::{Anchor, ExecutionPaths, GUEST_WORKSPACES_MOUNT, PROJECT_ROOT_ENV};
use project::seam::wire::{
    BuildOutput, BuildReport, BuildStatus, PhaseOutcome, PhaseReport, PhaseRoot, PhaseSource,
    PhaseWrite, RepairOrigin, UiSurface, build_finding,
};
use project::seam::{
    self, ArtifactStage, BuildContext, Evidence, Input, Lead, MergePhase, Source, Target, Workspace,
};
use project::snapshot::{CodePatch, SnapshotId};

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
    ) -> impl Future<Output = Result<PhaseReport, seam::Error>> + Send {
        async move {
            let wire_inputs = inputs.into_iter().map(map_input).collect();
            let wire_context = target::BuildContext {
                sources: context.sources,
            };
            let wire_workspace = map_workspace(workspace);
            let report = target::build(id, slice, wire_inputs, wire_context, wire_workspace)
                .await
                .map_err(map_error)?;
            Ok(map_phase_report(report))
        }
    }

    fn verify(
        &self, id: String, workspace: Workspace,
    ) -> impl Future<Output = Result<PhaseReport, seam::Error>> + Send {
        async move {
            let wire_workspace = map_workspace(workspace);
            let report = target::verify(id, wire_workspace).await.map_err(map_error)?;
            Ok(map_phase_report(report))
        }
    }

    fn repair(
        &self, id: String, slice: String, origin: RepairOrigin, findings: Vec<Diagnostic>,
        continuation: Option<Vec<u8>>, workspace: Workspace,
    ) -> impl Future<Output = Result<PhaseReport, seam::Error>> + Send {
        async move {
            let wire_origin = wire_origin(origin);
            let wire_findings = findings.into_iter().map(wire_finding).collect();
            let wire_workspace = map_workspace(workspace);
            let report =
                target::repair(id, slice, wire_origin, wire_findings, continuation, wire_workspace)
                    .await
                    .map_err(map_error)?;
            Ok(map_phase_report(report))
        }
    }

    fn review(
        &self, id: String, slice: String, continuation: Option<Vec<u8>>, workspace: Workspace,
    ) -> impl Future<Output = Result<PhaseReport, seam::Error>> + Send {
        async move {
            let wire_workspace = map_workspace(workspace);
            let report =
                target::review(id, slice, continuation, wire_workspace).await.map_err(map_error)?;
            Ok(map_phase_report(report))
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
            // The build orchestrator attaches the per-attempt artifact
            // stage; preparation itself lends none.
            Ok(Workspace {
                id: prepared.id,
                root: prepared.root.display().to_string(),
                artifacts: artifacts_root(),
                artifact_stage: None,
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
        artifact_stage: workspace.artifact_stage.map(map_artifact_stage),
    }
}

fn map_artifact_stage(stage: ArtifactStage) -> target::ArtifactStage {
    target::ArtifactStage {
        id: stage.id,
        root: stage.root,
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
                writable_artifacts: Vec::new(),
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
                writable_artifacts: record
                    .writable_artifacts
                    .into_iter()
                    .map(|artifact| WritableArtifactDeclaration {
                        path: artifact.path,
                        kind: match artifact.kind {
                            target::WritableArtifactKind::File => WritableArtifactKind::File,
                            target::WritableArtifactKind::Tree => WritableArtifactKind::Tree,
                        },
                    })
                    .collect(),
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
        report.outputs.into_iter().map(map_output).collect(),
        report.ui_surface.map(|surface| UiSurface {
            screens: surface.screens,
        }),
        report.covered,
    )
}

fn widen_finding(finding: target::Finding) -> Diagnostic {
    build_finding(finding.rule_id, finding.detail, map_severity(finding.severity))
}

fn map_output(output: target::BuildOutput) -> BuildOutput {
    BuildOutput {
        platform: map_platform(output.platform),
        path: output.path,
    }
}

/// Map a WIT phase report onto the engine wire shape — the isomorphic
/// projection of RFC-90 D2: nothing folds at this seam.
fn map_phase_report(report: target::PhaseReport) -> PhaseReport {
    PhaseReport {
        outcome: match report.outcome {
            target::PhaseOutcome::Completed => PhaseOutcome::Completed,
            target::PhaseOutcome::NotApplicable => PhaseOutcome::NotApplicable,
        },
        source: map_phase_source(report.source),
        findings: report.findings.into_iter().map(map_phase_finding).collect(),
        outputs: report.outputs.into_iter().map(map_output).collect(),
        ui_surface: report.ui_surface.map(|surface| UiSurface {
            screens: surface.screens,
        }),
        written: report.written.into_iter().map(map_phase_write).collect(),
        next_continuation: report.next_continuation,
    }
}

const fn map_phase_source(source: target::PhaseSource) -> PhaseSource {
    match source {
        target::PhaseSource::Deterministic => PhaseSource::Deterministic,
        target::PhaseSource::ModelAssisted => PhaseSource::ModelAssisted,
        target::PhaseSource::Hybrid => PhaseSource::Hybrid,
        target::PhaseSource::Tool => PhaseSource::Tool,
    }
}

fn map_phase_write(write: target::PhaseWrite) -> PhaseWrite {
    PhaseWrite {
        root: match write.root {
            target::PhaseRoot::Workspace => PhaseRoot::Workspace,
            target::PhaseRoot::Artifacts => PhaseRoot::Artifacts,
        },
        path: write.path,
    }
}

/// Map a WIT phase finding onto the full [`Diagnostic`] shape. The
/// engine stamps `target_adapter` / `slice` / change identity and
/// recomputes the fingerprint; this projection keeps every field
/// as-given.
fn map_phase_finding(finding: target::PhaseFinding) -> Diagnostic {
    Diagnostic {
        id: finding.id,
        rule_id: finding.rule_id,
        related_rule_ids: (!finding.related_rule_ids.is_empty())
            .then_some(finding.related_rule_ids),
        title: finding.title,
        severity: map_severity(finding.severity),
        source: map_diagnostic_source(finding.source),
        kind: match finding.kind {
            target::FindingKind::Violation => DiagnosticKind::Violation,
            target::FindingKind::Review => DiagnosticKind::Review,
        },
        target_adapter: None,
        source_adapter: None,
        slice: None,
        change: None,
        artifact: map_finding_artifact(finding.artifact),
        location: finding.location.map(map_location),
        evidence: map_evidence(finding.evidence),
        impact: finding.impact,
        remediation: finding.remediation,
        confidence: finding.confidence.map(|confidence| match confidence {
            target::FindingConfidence::High => Confidence::High,
            target::FindingConfidence::Medium => Confidence::Medium,
            target::FindingConfidence::Low => Confidence::Low,
        }),
        fingerprint: finding.fingerprint,
    }
}

const fn map_diagnostic_source(source: target::DiagnosticSource) -> DiagnosticSource {
    match source {
        target::DiagnosticSource::Deterministic => DiagnosticSource::Deterministic,
        target::DiagnosticSource::ModelAssisted => DiagnosticSource::ModelAssisted,
        target::DiagnosticSource::Hybrid => DiagnosticSource::Hybrid,
        target::DiagnosticSource::Human => DiagnosticSource::Human,
        target::DiagnosticSource::Tool => DiagnosticSource::Tool,
    }
}

const fn map_finding_artifact(artifact: target::FindingArtifact) -> Artifact {
    match artifact {
        target::FindingArtifact::Code => Artifact::Code,
        target::FindingArtifact::Tests => Artifact::Tests,
        target::FindingArtifact::Contracts => Artifact::Contracts,
        target::FindingArtifact::Specs => Artifact::Specs,
        target::FindingArtifact::Design => Artifact::Design,
        target::FindingArtifact::Decisions => Artifact::Decisions,
        target::FindingArtifact::Tasks => Artifact::Tasks,
        target::FindingArtifact::Assets => Artifact::Assets,
        target::FindingArtifact::Tokens => Artifact::Tokens,
        target::FindingArtifact::Composition => Artifact::Composition,
        target::FindingArtifact::Plan => Artifact::Plan,
        target::FindingArtifact::Unknown => Artifact::Unknown,
    }
}

fn map_location(location: target::PhaseLocation) -> diagnostics::FindingLocation {
    diagnostics::FindingLocation {
        path: location.path,
        line: location.line,
        column: location.column,
        end_line: location.end_line,
        end_column: location.end_column,
    }
}

fn map_evidence(evidence: target::FindingEvidence) -> diagnostics::FindingEvidence {
    match evidence {
        target::FindingEvidence::Snippet(value) => diagnostics::FindingEvidence::Snippet { value },
        target::FindingEvidence::Digest(digest) => diagnostics::FindingEvidence::Digest {
            sha256: digest.sha256,
            summary: digest.summary,
            locations: digest
                .locations
                .map(|locations| locations.into_iter().map(map_location).collect()),
        },
        target::FindingEvidence::Structured(structured) => {
            diagnostics::FindingEvidence::Structured {
                summary: structured.summary,
                // A payload that fails to reparse survives as a JSON
                // string rather than dropping evidence.
                data: serde_json::from_str(&structured.data)
                    .unwrap_or(serde_json::Value::String(structured.data)),
                locations: structured
                    .locations
                    .map(|locations| locations.into_iter().map(map_location).collect()),
            }
        }
    }
}

const fn wire_origin(origin: RepairOrigin) -> target::RepairOrigin {
    match origin {
        RepairOrigin::Verification => target::RepairOrigin::Verification,
        RepairOrigin::Review => target::RepairOrigin::Review,
    }
}

/// Project a [`Diagnostic`] onto the WIT phase-finding record for a
/// repair brief. The engine-stamped identity fields
/// (`target_adapter` / `source_adapter` / `slice` / `change`) do not
/// exist on the WIT record and are dropped.
fn wire_finding(diagnostic: Diagnostic) -> target::PhaseFinding {
    target::PhaseFinding {
        id: diagnostic.id,
        rule_id: diagnostic.rule_id,
        related_rule_ids: diagnostic.related_rule_ids.unwrap_or_default(),
        title: diagnostic.title,
        severity: wire_severity(diagnostic.severity),
        source: wire_diagnostic_source(diagnostic.source),
        kind: match diagnostic.kind {
            DiagnosticKind::Violation => target::FindingKind::Violation,
            DiagnosticKind::Review => target::FindingKind::Review,
        },
        artifact: wire_artifact(diagnostic.artifact),
        location: diagnostic.location.map(wire_location),
        evidence: wire_evidence(diagnostic.evidence),
        impact: diagnostic.impact,
        remediation: diagnostic.remediation,
        confidence: diagnostic.confidence.map(|confidence| match confidence {
            Confidence::High => target::FindingConfidence::High,
            Confidence::Medium => target::FindingConfidence::Medium,
            Confidence::Low => target::FindingConfidence::Low,
        }),
        fingerprint: diagnostic.fingerprint,
    }
}

const fn wire_severity(severity: Severity) -> target::Severity {
    match severity {
        Severity::Critical => target::Severity::Critical,
        Severity::Important => target::Severity::Important,
        Severity::Suggestion => target::Severity::Suggestion,
        Severity::Optional => target::Severity::Optional,
    }
}

const fn wire_diagnostic_source(source: DiagnosticSource) -> target::DiagnosticSource {
    match source {
        DiagnosticSource::Deterministic => target::DiagnosticSource::Deterministic,
        DiagnosticSource::ModelAssisted => target::DiagnosticSource::ModelAssisted,
        DiagnosticSource::Hybrid => target::DiagnosticSource::Hybrid,
        DiagnosticSource::Human => target::DiagnosticSource::Human,
        DiagnosticSource::Tool => target::DiagnosticSource::Tool,
    }
}

const fn wire_artifact(artifact: Artifact) -> target::FindingArtifact {
    match artifact {
        Artifact::Code => target::FindingArtifact::Code,
        Artifact::Tests => target::FindingArtifact::Tests,
        Artifact::Contracts => target::FindingArtifact::Contracts,
        Artifact::Specs => target::FindingArtifact::Specs,
        Artifact::Design => target::FindingArtifact::Design,
        Artifact::Decisions => target::FindingArtifact::Decisions,
        Artifact::Tasks => target::FindingArtifact::Tasks,
        Artifact::Assets => target::FindingArtifact::Assets,
        Artifact::Tokens => target::FindingArtifact::Tokens,
        Artifact::Composition => target::FindingArtifact::Composition,
        Artifact::Plan => target::FindingArtifact::Plan,
        Artifact::Unknown => target::FindingArtifact::Unknown,
    }
}

fn wire_location(location: diagnostics::FindingLocation) -> target::PhaseLocation {
    target::PhaseLocation {
        path: location.path,
        line: location.line,
        column: location.column,
        end_line: location.end_line,
        end_column: location.end_column,
    }
}

fn wire_evidence(evidence: diagnostics::FindingEvidence) -> target::FindingEvidence {
    match evidence {
        diagnostics::FindingEvidence::Snippet { value } => target::FindingEvidence::Snippet(value),
        diagnostics::FindingEvidence::Digest {
            sha256,
            summary,
            locations,
        } => target::FindingEvidence::Digest(target::DigestEvidence {
            sha256,
            summary,
            locations: locations
                .map(|locations| locations.into_iter().map(wire_location).collect()),
        }),
        diagnostics::FindingEvidence::Structured {
            summary,
            data,
            locations,
        } => target::FindingEvidence::Structured(target::StructuredEvidence {
            summary,
            data: data.to_string(),
            locations: locations
                .map(|locations| locations.into_iter().map(wire_location).collect()),
        }),
    }
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
