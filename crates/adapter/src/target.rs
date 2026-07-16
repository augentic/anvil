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

pub use generated::exports::specify::adapter::target::*;
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
            specify_floor: metadata.specify_floor,
            inputs: metadata.inputs.into_iter().map(Into::into).collect(),
            platforms: metadata.platforms.map(Into::into),
        }
    }
}

impl From<Input> for crate::seam::Input {
    fn from(input: Input) -> Self {
        match input {
            Input::Proposal(body) => Self::Proposal(body),
            Input::Design(body) => Self::Design(body),
            Input::Tasks(body) => Self::Tasks(body),
            Input::Spec(body) => Self::Spec(body),
            Input::Other(body) => Self::Other(body),
        }
    }
}

impl From<WorkingTree> for crate::seam::WorkingTree {
    fn from(tree: WorkingTree) -> Self {
        Self {
            base: tree.base,
            subpath: tree.subpath,
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

/// Infallible today; WIT marks the operation fallible, so this returns `Result`.
///
/// # Errors
///
/// Never — always `Ok`.
pub fn dispatch_guidance<A: crate::Target>() -> Result<String, Error> {
    Ok(A::guidance().to_string())
}

/// # Errors
///
/// As the implementor's [`build`](crate::Target::build).
pub async fn dispatch_build<A: crate::Target>(
    id: AdapterId, slice: String, inputs: Vec<Input>, tree: WorkingTree,
) -> Result<Report, Error> {
    let inputs: Vec<crate::seam::Input> = inputs.into_iter().map(Into::into).collect();
    let tree = crate::seam::WorkingTree::from(tree);
    let url = crate::references::mcp_url(A::NAME);
    let ctx = crate::seam::Context::guest(&id, url.as_deref());
    A::build(&crate::WasiModel, &ctx, &slice, &inputs, &tree)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

/// # Errors
///
/// As the implementor's [`merge`](crate::Target::merge).
pub async fn dispatch_merge<A: crate::Target>(
    id: AdapterId, slice: String, phase: MergePhase, tree: WorkingTree,
) -> Result<Report, Error> {
    let phase = crate::seam::MergePhase::from(phase);
    let tree = crate::seam::WorkingTree::from(tree);
    let url = crate::references::mcp_url(A::NAME);
    let ctx = crate::seam::Context::guest(&id, url.as_deref());
    A::merge(&crate::WasiModel, &ctx, &slice, phase, &tree)
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
                _id: $crate::target::AdapterId,
            ) -> Result<String, $crate::target::Error> {
                $crate::target::dispatch_guidance::<$adapter>()
            }

            async fn build(
                id: $crate::target::AdapterId,
                slice: String,
                inputs: Vec<$crate::target::Input>,
                tree: $crate::target::WorkingTree,
            ) -> Result<$crate::target::Report, $crate::target::Error> {
                $crate::target::dispatch_build::<$adapter>(id, slice, inputs, tree).await
            }

            async fn merge(
                id: $crate::target::AdapterId,
                slice: String,
                phase: $crate::target::MergePhase,
                tree: $crate::target::WorkingTree,
            ) -> Result<$crate::target::Report, $crate::target::Error> {
                $crate::target::dispatch_merge::<$adapter>(id, slice, phase, tree).await
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
                    <$adapter as $crate::Target>::NAME,
                    env!("CARGO_PKG_VERSION"),
                    <$adapter as $crate::Target>::docs(),
                    request,
                )
                .await
            }
        }
    };
}
