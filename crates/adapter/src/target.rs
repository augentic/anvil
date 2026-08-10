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

impl From<crate::seam::TargetMetadata> for AdapterMetadata {
    fn from(metadata: crate::seam::TargetMetadata) -> Self {
        Self {
            emery_floor: metadata.emery_floor,
            inputs: metadata.inputs.into_iter().map(Into::into).collect(),
            platforms: metadata.platforms.map(Into::into),
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

impl From<Workspace> for crate::seam::Workspace {
    fn from(workspace: Workspace) -> Self {
        Self {
            id: workspace.id,
            root: workspace.root,
            artifacts: workspace.artifacts,
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
            covered: report.covered,
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
) -> Result<Report, Error> {
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
            ) -> Result<$crate::target::Report, $crate::target::Error> {
                $crate::target::dispatch_build::<$adapter>(id, slice, inputs, context, workspace)
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
