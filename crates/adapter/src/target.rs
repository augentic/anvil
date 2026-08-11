//! `target-adapter` WIT bindings and the `target!` export macro.
//!
//! One `wit_bindgen::generate!` here; leaf crates wire a [`crate::Target`]
//! implementor with `adapter::target!(…)`.

mod generated {
    #![allow(
        missing_docs,
        unsafe_code,
        clippy::pedantic,
        clippy::nursery,
        reason = "wit-bindgen generated bindings are not hand-maintained; the generated code cannot carry this workspace's lint posture"
    )]

    wit_bindgen::generate!({
        world: "target-adapter",
        path: "../../wit",
        // Judgment ops are async; `metadata` is sync.
        generate_all,
        pub_export_macro: true,
    });
}

pub use generated::exports::emery::adapter::target::*;
pub use generated::*;

impl From<crate::seam::BuildInput> for BuildInput {
    fn from(input: crate::seam::BuildInput) -> Self {
        Self {
            path: input.path,
            required: input.required,
        }
    }
}

impl From<crate::seam::PlatformsCapability> for PlatformsCapability {
    fn from(capability: crate::seam::PlatformsCapability) -> Self {
        Self {
            required: capability.required,
            allowed: capability.allowed.into_iter().map(Into::into).collect(),
            default: capability.default.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::seam::WritableArtifactKind> for WritableArtifactKind {
    fn from(kind: crate::seam::WritableArtifactKind) -> Self {
        match kind {
            crate::seam::WritableArtifactKind::File => Self::File,
            crate::seam::WritableArtifactKind::Tree => Self::Tree,
        }
    }
}

impl From<crate::seam::WritableArtifact> for WritableArtifact {
    fn from(artifact: crate::seam::WritableArtifact) -> Self {
        Self {
            path: artifact.path,
            kind: artifact.kind.into(),
        }
    }
}

impl From<crate::seam::TargetMetadata> for AdapterMetadata {
    fn from(metadata: crate::seam::TargetMetadata) -> Self {
        Self {
            emery_floor: metadata.emery_floor,
            inputs: metadata.inputs.into_iter().map(Into::into).collect(),
            platforms: metadata.platforms.map(Into::into),
            writable_artifacts: metadata.writable_artifacts.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Payload> for crate::seam::Payload {
    fn from(payload: Payload) -> Self {
        match payload {
            Payload::Path(path) => Self::Path(path),
            Payload::Body(body) => Self::Body(body),
        }
    }
}

impl From<Input> for crate::seam::Input {
    fn from(input: Input) -> Self {
        match input {
            Input::Proposal(payload) => Self::Proposal(payload.into()),
            Input::Design(payload) => Self::Design(payload.into()),
            Input::Tasks(payload) => Self::Tasks(payload.into()),
            Input::Spec(payload) => Self::Spec(payload.into()),
            Input::Other(payload) => Self::Other(payload.into()),
        }
    }
}

impl From<BuildContext> for crate::seam::BuildContext {
    fn from(context: BuildContext) -> Self {
        Self {
            sources: context.sources,
        }
    }
}

impl From<ArtifactStage> for crate::seam::ArtifactStage {
    fn from(stage: ArtifactStage) -> Self {
        Self {
            id: stage.id,
            root: stage.root,
        }
    }
}

impl From<Workspace> for crate::seam::Workspace {
    fn from(workspace: Workspace) -> Self {
        Self {
            id: workspace.id,
            root: workspace.root,
            artifacts: workspace.artifacts,
            artifact_stage: workspace.artifact_stage.map(Into::into),
        }
    }
}

impl From<MergePhase> for crate::seam::MergePhase {
    fn from(phase: MergePhase) -> Self {
        match phase {
            MergePhase::Preflight => Self::Preflight,
            MergePhase::Postflight => Self::Postflight,
        }
    }
}

impl From<RepairOrigin> for crate::seam::RepairOrigin {
    fn from(origin: RepairOrigin) -> Self {
        match origin {
            RepairOrigin::Verification => Self::Verification,
            RepairOrigin::Review => Self::Review,
        }
    }
}

impl From<Severity> for crate::seam::Severity {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Critical => Self::Critical,
            Severity::Important => Self::Important,
            Severity::Suggestion => Self::Suggestion,
            Severity::Optional => Self::Optional,
        }
    }
}

impl From<DiagnosticSource> for crate::seam::DiagnosticSource {
    fn from(source: DiagnosticSource) -> Self {
        match source {
            DiagnosticSource::Deterministic => Self::Deterministic,
            DiagnosticSource::ModelAssisted => Self::ModelAssisted,
            DiagnosticSource::Hybrid => Self::Hybrid,
            DiagnosticSource::Human => Self::Human,
            DiagnosticSource::Tool => Self::Tool,
        }
    }
}

impl From<crate::seam::DiagnosticSource> for DiagnosticSource {
    fn from(source: crate::seam::DiagnosticSource) -> Self {
        match source {
            crate::seam::DiagnosticSource::Deterministic => Self::Deterministic,
            crate::seam::DiagnosticSource::ModelAssisted => Self::ModelAssisted,
            crate::seam::DiagnosticSource::Hybrid => Self::Hybrid,
            crate::seam::DiagnosticSource::Human => Self::Human,
            crate::seam::DiagnosticSource::Tool => Self::Tool,
        }
    }
}

impl From<FindingKind> for crate::seam::FindingKind {
    fn from(kind: FindingKind) -> Self {
        match kind {
            FindingKind::Violation => Self::Violation,
            FindingKind::Review => Self::Review,
        }
    }
}

impl From<crate::seam::FindingKind> for FindingKind {
    fn from(kind: crate::seam::FindingKind) -> Self {
        match kind {
            crate::seam::FindingKind::Violation => Self::Violation,
            crate::seam::FindingKind::Review => Self::Review,
        }
    }
}

impl From<FindingArtifact> for crate::seam::FindingArtifact {
    fn from(artifact: FindingArtifact) -> Self {
        match artifact {
            FindingArtifact::Code => Self::Code,
            FindingArtifact::Tests => Self::Tests,
            FindingArtifact::Contracts => Self::Contracts,
            FindingArtifact::Specs => Self::Specs,
            FindingArtifact::Design => Self::Design,
            FindingArtifact::Decisions => Self::Decisions,
            FindingArtifact::Tasks => Self::Tasks,
            FindingArtifact::Assets => Self::Assets,
            FindingArtifact::Tokens => Self::Tokens,
            FindingArtifact::Composition => Self::Composition,
            FindingArtifact::Plan => Self::Plan,
            FindingArtifact::Unknown => Self::Unknown,
        }
    }
}

impl From<crate::seam::FindingArtifact> for FindingArtifact {
    fn from(artifact: crate::seam::FindingArtifact) -> Self {
        match artifact {
            crate::seam::FindingArtifact::Code => Self::Code,
            crate::seam::FindingArtifact::Tests => Self::Tests,
            crate::seam::FindingArtifact::Contracts => Self::Contracts,
            crate::seam::FindingArtifact::Specs => Self::Specs,
            crate::seam::FindingArtifact::Design => Self::Design,
            crate::seam::FindingArtifact::Decisions => Self::Decisions,
            crate::seam::FindingArtifact::Tasks => Self::Tasks,
            crate::seam::FindingArtifact::Assets => Self::Assets,
            crate::seam::FindingArtifact::Tokens => Self::Tokens,
            crate::seam::FindingArtifact::Composition => Self::Composition,
            crate::seam::FindingArtifact::Plan => Self::Plan,
            crate::seam::FindingArtifact::Unknown => Self::Unknown,
        }
    }
}

impl From<FindingConfidence> for crate::seam::FindingConfidence {
    fn from(confidence: FindingConfidence) -> Self {
        match confidence {
            FindingConfidence::High => Self::High,
            FindingConfidence::Medium => Self::Medium,
            FindingConfidence::Low => Self::Low,
        }
    }
}

impl From<crate::seam::FindingConfidence> for FindingConfidence {
    fn from(confidence: crate::seam::FindingConfidence) -> Self {
        match confidence {
            crate::seam::FindingConfidence::High => Self::High,
            crate::seam::FindingConfidence::Medium => Self::Medium,
            crate::seam::FindingConfidence::Low => Self::Low,
        }
    }
}

impl From<PhaseLocation> for crate::seam::PhaseLocation {
    fn from(location: PhaseLocation) -> Self {
        Self {
            path: location.path,
            line: location.line,
            column: location.column,
            end_line: location.end_line,
            end_column: location.end_column,
        }
    }
}

impl From<crate::seam::PhaseLocation> for PhaseLocation {
    fn from(location: crate::seam::PhaseLocation) -> Self {
        Self {
            path: location.path,
            line: location.line,
            column: location.column,
            end_line: location.end_line,
            end_column: location.end_column,
        }
    }
}

impl From<FindingEvidence> for crate::seam::FindingEvidence {
    fn from(evidence: FindingEvidence) -> Self {
        match evidence {
            FindingEvidence::Snippet(value) => Self::Snippet { value },
            FindingEvidence::Digest(digest) => Self::Digest {
                sha256: digest.sha256,
                summary: digest.summary,
                locations: digest
                    .locations
                    .map(|locations| locations.into_iter().map(Into::into).collect()),
            },
            FindingEvidence::Structured(structured) => Self::Structured {
                summary: structured.summary,
                // A payload that fails to reparse survives as a JSON
                // string rather than dropping evidence.
                data: serde_json::from_str(&structured.data)
                    .unwrap_or(serde_json::Value::String(structured.data)),
                locations: structured
                    .locations
                    .map(|locations| locations.into_iter().map(Into::into).collect()),
            },
        }
    }
}

impl From<crate::seam::FindingEvidence> for FindingEvidence {
    fn from(evidence: crate::seam::FindingEvidence) -> Self {
        match evidence {
            crate::seam::FindingEvidence::Snippet { value } => Self::Snippet(value),
            crate::seam::FindingEvidence::Digest {
                sha256,
                summary,
                locations,
            } => Self::Digest(DigestEvidence {
                sha256,
                summary,
                locations: locations
                    .map(|locations| locations.into_iter().map(Into::into).collect()),
            }),
            crate::seam::FindingEvidence::Structured {
                summary,
                data,
                locations,
            } => Self::Structured(StructuredEvidence {
                summary,
                data: data.to_string(),
                locations: locations
                    .map(|locations| locations.into_iter().map(Into::into).collect()),
            }),
        }
    }
}

impl From<PhaseFinding> for crate::seam::PhaseFinding {
    fn from(finding: PhaseFinding) -> Self {
        Self {
            id: finding.id,
            rule_id: finding.rule_id,
            related_rule_ids: finding.related_rule_ids,
            title: finding.title,
            severity: finding.severity.into(),
            source: finding.source.into(),
            kind: finding.kind.into(),
            artifact: finding.artifact.into(),
            location: finding.location.map(Into::into),
            evidence: finding.evidence.into(),
            impact: finding.impact,
            remediation: finding.remediation,
            confidence: finding.confidence.map(Into::into),
            fingerprint: finding.fingerprint,
        }
    }
}

impl From<crate::seam::PhaseFinding> for PhaseFinding {
    fn from(finding: crate::seam::PhaseFinding) -> Self {
        Self {
            id: finding.id,
            rule_id: finding.rule_id,
            related_rule_ids: finding.related_rule_ids,
            title: finding.title,
            severity: finding.severity.into(),
            source: finding.source.into(),
            kind: finding.kind.into(),
            artifact: finding.artifact.into(),
            location: finding.location.map(Into::into),
            evidence: finding.evidence.into(),
            impact: finding.impact,
            remediation: finding.remediation,
            confidence: finding.confidence.map(Into::into),
            fingerprint: finding.fingerprint,
        }
    }
}

impl From<crate::seam::PhaseOutcome> for PhaseOutcome {
    fn from(outcome: crate::seam::PhaseOutcome) -> Self {
        match outcome {
            crate::seam::PhaseOutcome::Completed => Self::Completed,
            crate::seam::PhaseOutcome::NotApplicable => Self::NotApplicable,
        }
    }
}

impl From<crate::seam::PhaseSource> for PhaseSource {
    fn from(source: crate::seam::PhaseSource) -> Self {
        match source {
            crate::seam::PhaseSource::Deterministic => Self::Deterministic,
            crate::seam::PhaseSource::ModelAssisted => Self::ModelAssisted,
            crate::seam::PhaseSource::Hybrid => Self::Hybrid,
            crate::seam::PhaseSource::Tool => Self::Tool,
        }
    }
}

impl From<crate::seam::PhaseRoot> for PhaseRoot {
    fn from(root: crate::seam::PhaseRoot) -> Self {
        match root {
            crate::seam::PhaseRoot::Workspace => Self::Workspace,
            crate::seam::PhaseRoot::Artifacts => Self::Artifacts,
        }
    }
}

impl From<crate::seam::PhaseWrite> for PhaseWrite {
    fn from(write: crate::seam::PhaseWrite) -> Self {
        Self {
            root: write.root.into(),
            path: write.path,
        }
    }
}

impl From<crate::seam::PhaseReport> for PhaseReport {
    fn from(report: crate::seam::PhaseReport) -> Self {
        Self {
            outcome: report.outcome.into(),
            source: report.source.into(),
            findings: report.findings.into_iter().map(Into::into).collect(),
            outputs: report.outputs.into_iter().map(Into::into).collect(),
            ui_surface: report.ui_surface.map(Into::into),
            covered: report.covered,
            written: report.written.into_iter().map(Into::into).collect(),
            next_continuation: report.next_continuation,
        }
    }
}

impl From<crate::seam::Status> for Status {
    fn from(status: crate::seam::Status) -> Self {
        match status {
            crate::seam::Status::Success => Self::Success,
            crate::seam::Status::Failure => Self::Failure,
        }
    }
}

impl From<crate::seam::Severity> for Severity {
    fn from(severity: crate::seam::Severity) -> Self {
        match severity {
            crate::seam::Severity::Critical => Self::Critical,
            crate::seam::Severity::Important => Self::Important,
            crate::seam::Severity::Suggestion => Self::Suggestion,
            crate::seam::Severity::Optional => Self::Optional,
        }
    }
}

impl From<crate::seam::Finding> for Finding {
    fn from(finding: crate::seam::Finding) -> Self {
        Self {
            rule_id: finding.rule_id,
            severity: finding.severity.into(),
            detail: finding.detail,
        }
    }
}

impl From<crate::seam::Platform> for Platform {
    fn from(platform: crate::seam::Platform) -> Self {
        match platform {
            crate::seam::Platform::Core => Self::Core,
            crate::seam::Platform::Ios => Self::Ios,
            crate::seam::Platform::Android => Self::Android,
            crate::seam::Platform::Web => Self::Web,
            crate::seam::Platform::Desktop => Self::Desktop,
        }
    }
}

impl From<crate::seam::BuildOutput> for BuildOutput {
    fn from(output: crate::seam::BuildOutput) -> Self {
        Self {
            platform: output.platform.into(),
            path: output.path,
        }
    }
}

impl From<crate::seam::UiSurface> for UiSurface {
    fn from(surface: crate::seam::UiSurface) -> Self {
        Self {
            screens: surface.screens,
        }
    }
}

impl From<crate::seam::Report> for Report {
    fn from(report: crate::seam::Report) -> Self {
        Self {
            status: report.status.into(),
            findings: report.findings.into_iter().map(Into::into).collect(),
            outputs: report.outputs.into_iter().map(Into::into).collect(),
            ui_surface: report.ui_surface.map(Into::into),
        }
    }
}

impl From<crate::seam::Error> for Error {
    fn from(error: crate::seam::Error) -> Self {
        match error {
            crate::seam::Error::InvalidRequest(detail) => Self::InvalidRequest(detail),
            crate::seam::Error::Io(detail) => Self::Io(detail),
            crate::seam::Error::Internal(detail) => Self::Internal(detail),
        }
    }
}

/// Map [`crate::Target::metadata`] onto the WIT record.
#[must_use]
pub fn dispatch_metadata<A: crate::Target>() -> AdapterMetadata {
    A::metadata().into()
}

/// # Errors
///
/// As the implementor's [`guidance`](crate::Target::guidance).
pub async fn dispatch_guidance<A: crate::Target>(id: AdapterId) -> Result<String, Error> {
    let ctx = crate::seam::Context::guest(&id);
    A::guidance(&crate::WasiModel, &ctx).await.map_err(Into::into)
}

/// # Errors
///
/// As the implementor's [`build`](crate::Target::build).
pub async fn dispatch_build<A: crate::Target>(
    id: AdapterId, slice: String, inputs: Vec<Input>, context: BuildContext, workspace: Workspace,
) -> Result<PhaseReport, Error> {
    let inputs: Vec<crate::seam::Input> = inputs.into_iter().map(Into::into).collect();
    let context = crate::seam::BuildContext::from(context);
    let workspace = crate::seam::Workspace::from(workspace);
    let ctx = crate::seam::Context::guest(&id).lending(workspace.root.clone());
    A::build(&crate::WasiModel, &ctx, &slice, &inputs, &context, &workspace)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

/// # Errors
///
/// As the implementor's [`verify`](crate::Target::verify).
pub async fn dispatch_verify<A: crate::Target>(
    id: AdapterId, workspace: Workspace,
) -> Result<PhaseReport, Error> {
    let workspace = crate::seam::Workspace::from(workspace);
    let ctx = crate::seam::Context::guest(&id).lending(workspace.root.clone());
    A::verify(&crate::WasiModel, &ctx, &workspace).await.map(Into::into).map_err(Into::into)
}

/// # Errors
///
/// As the implementor's [`repair`](crate::Target::repair).
pub async fn dispatch_repair<A: crate::Target>(
    id: AdapterId, slice: String, origin: RepairOrigin, findings: Vec<PhaseFinding>,
    continuation: Option<Vec<u8>>, workspace: Workspace,
) -> Result<PhaseReport, Error> {
    let origin = crate::seam::RepairOrigin::from(origin);
    let findings: Vec<crate::seam::PhaseFinding> = findings.into_iter().map(Into::into).collect();
    let workspace = crate::seam::Workspace::from(workspace);
    let ctx = crate::seam::Context::guest(&id).lending(workspace.root.clone());
    A::repair(
        &crate::WasiModel,
        &ctx,
        &slice,
        origin,
        &findings,
        continuation.as_deref(),
        &workspace,
    )
    .await
    .map(Into::into)
    .map_err(Into::into)
}

/// # Errors
///
/// As the implementor's [`review`](crate::Target::review).
pub async fn dispatch_review<A: crate::Target>(
    id: AdapterId, slice: String, continuation: Option<Vec<u8>>, workspace: Workspace,
) -> Result<PhaseReport, Error> {
    let workspace = crate::seam::Workspace::from(workspace);
    let ctx = crate::seam::Context::guest(&id).lending(workspace.root.clone());
    A::review(&crate::WasiModel, &ctx, &slice, continuation.as_deref(), &workspace)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

/// # Errors
///
/// As the implementor's [`merge`](crate::Target::merge).
pub async fn dispatch_merge<A: crate::Target>(
    id: AdapterId, slice: String, phase: MergePhase, workspace: Workspace,
) -> Result<Report, Error> {
    let phase = crate::seam::MergePhase::from(phase);
    let workspace = crate::seam::Workspace::from(workspace);
    let ctx = crate::seam::Context::guest(&id).lending(workspace.root.clone());
    A::merge(&crate::WasiModel, &ctx, &slice, phase, &workspace)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

/// Wire a [`crate::Target`] implementor into the component exports.
///
/// ```ignore
/// adapter::target!(crate::Vectis);
/// ```
#[macro_export]
macro_rules! target {
    ($adapter:ty) => {
        struct Adapter;
        $crate::target::export!(Adapter with_types_in $crate::target);

        impl $crate::target::Guest for Adapter {
            fn metadata(
                _id: $crate::target::AdapterId,
            ) -> $crate::target::AdapterMetadata {
                $crate::target::dispatch_metadata::<$adapter>()
            }

            async fn guidance(
                id: $crate::target::AdapterId,
            ) -> Result<String, $crate::target::Error> {
                $crate::target::dispatch_guidance::<$adapter>(id).await
            }

            async fn build(
                id: $crate::target::AdapterId,
                slice: String,
                inputs: Vec<$crate::target::Input>,
                context: $crate::target::BuildContext,
                workspace: $crate::target::Workspace,
            ) -> Result<$crate::target::PhaseReport, $crate::target::Error> {
                $crate::target::dispatch_build::<$adapter>(id, slice, inputs, context, workspace)
                    .await
            }

            async fn verify(
                id: $crate::target::AdapterId,
                workspace: $crate::target::Workspace,
            ) -> Result<$crate::target::PhaseReport, $crate::target::Error> {
                $crate::target::dispatch_verify::<$adapter>(id, workspace).await
            }

            async fn repair(
                id: $crate::target::AdapterId,
                slice: String,
                origin: $crate::target::RepairOrigin,
                findings: Vec<$crate::target::PhaseFinding>,
                continuation: Option<Vec<u8>>,
                workspace: $crate::target::Workspace,
            ) -> Result<$crate::target::PhaseReport, $crate::target::Error> {
                $crate::target::dispatch_repair::<$adapter>(
                    id,
                    slice,
                    origin,
                    findings,
                    continuation,
                    workspace,
                )
                .await
            }

            async fn review(
                id: $crate::target::AdapterId,
                slice: String,
                continuation: Option<Vec<u8>>,
                workspace: $crate::target::Workspace,
            ) -> Result<$crate::target::PhaseReport, $crate::target::Error> {
                $crate::target::dispatch_review::<$adapter>(id, slice, continuation, workspace)
                    .await
            }

            async fn merge(
                id: $crate::target::AdapterId,
                slice: String,
                phase: $crate::target::MergePhase,
                workspace: $crate::target::Workspace,
            ) -> Result<$crate::target::Report, $crate::target::Error> {
                $crate::target::dispatch_merge::<$adapter>(id, slice, phase, workspace).await
            }
        }

        struct HttpGuest;
        $crate::wasip3::http::service::export!(HttpGuest);

        impl $crate::wasip3::exports::http::handler::Guest for HttpGuest {
            async fn handle(
                request: $crate::wasip3::http::types::Request,
            ) -> Result<
                $crate::wasip3::http::types::Response,
                $crate::wasip3::http::types::ErrorCode,
            > {
                $crate::references::serve(
                    <$adapter as $crate::Target>::IDENTITY.name,
                    <$adapter as $crate::Target>::IDENTITY.version,
                    <$adapter as $crate::Target>::docs(),
                    request,
                )
                .await
            }
        }
    };
}
