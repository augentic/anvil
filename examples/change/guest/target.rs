use std::path::Path;

use testkit::adapter;

use crate::Adapter;
use crate::bindings::exports::specify::adapter::target::{
    AdapterId, AdapterMetadata, BuildOutput, Error, Guest, Input, MergePhase, Platform,
    PlatformsCapability, Report, Status, WorkingTree,
};

impl Guest for Adapter {
    fn metadata(id: AdapterId) -> AdapterMetadata {
        AdapterMetadata {
            specify_floor: None,
            inputs: Vec::new(),
            platforms: adapter::target_platforms(&id).map(PlatformsCapability::from),
        }
    }

    async fn guidance(id: AdapterId) -> Result<String, Error> {
        adapter::guidance(&id).map_err(Error::from)
    }

    async fn build(
        id: AdapterId, slice: String, inputs: Vec<Input>, _tree: WorkingTree,
    ) -> Result<Report, Error> {
        // Every guest shares the deployment's `[[mount]]` preopens, so
        // the build writes through its own `"."` preopen.
        let inputs: Vec<adapter::Input> = inputs.into_iter().map(adapter::Input::from).collect();
        let report = adapter::build(Path::new("."), &id, &slice, &inputs).map_err(Error::from)?;
        Ok(report.into())
    }

    async fn merge(
        id: AdapterId, slice: String, phase: MergePhase, _tree: WorkingTree,
    ) -> Result<Report, Error> {
        let report =
            adapter::merge(Path::new("."), &id, &slice, phase.into()).map_err(Error::from)?;
        Ok(report.into())
    }
}

impl From<adapter::PlatformsCapability> for PlatformsCapability {
    fn from(capability: adapter::PlatformsCapability) -> Self {
        Self {
            required: capability.required,
            allowed: capability.allowed.into_iter().map(Platform::from).collect(),
            default: capability.default.into_iter().map(Platform::from).collect(),
        }
    }
}

impl From<adapter::Platform> for Platform {
    fn from(platform: adapter::Platform) -> Self {
        match platform {
            adapter::Platform::Core => Platform::Core,
            adapter::Platform::Ios => Platform::Ios,
            adapter::Platform::Android => Platform::Android,
        }
    }
}

impl From<Input> for adapter::Input {
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

impl From<MergePhase> for adapter::MergePhase {
    fn from(phase: MergePhase) -> Self {
        match phase {
            MergePhase::Preflight => Self::Preflight,
            MergePhase::Postflight => Self::Postflight,
        }
    }
}

impl From<adapter::Report> for Report {
    fn from(report: adapter::Report) -> Self {
        Self {
            status: report.status.into(),
            findings: Vec::new(),
            outputs: report.outputs.into_iter().map(BuildOutput::from).collect(),
            ui_surface: None,
        }
    }
}

impl From<adapter::Status> for Status {
    fn from(status: adapter::Status) -> Self {
        match status {
            adapter::Status::Success => Status::Success,
            adapter::Status::Failure => Status::Failure,
        }
    }
}

impl From<adapter::Output> for BuildOutput {
    fn from(output: adapter::Output) -> Self {
        Self {
            platform: Platform::Core,
            path: output.path,
        }
    }
}
