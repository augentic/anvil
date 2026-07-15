use std::path::Path;

use project::platform;
use project::seam::{self, wire};
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
        let inputs: Vec<seam::Input> = inputs.into_iter().map(seam::Input::from).collect();
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

impl From<project::adapter::PlatformsCapability> for PlatformsCapability {
    fn from(capability: project::adapter::PlatformsCapability) -> Self {
        Self {
            required: capability.required,
            allowed: capability.allowed.into_iter().map(Platform::from).collect(),
            default: capability.default.into_iter().map(Platform::from).collect(),
        }
    }
}

impl From<platform::Platform> for Platform {
    fn from(platform: platform::Platform) -> Self {
        match platform {
            platform::Platform::Core => Self::Core,
            platform::Platform::Ios => Self::Ios,
            platform::Platform::Android => Self::Android,
            platform::Platform::Web => Self::Web,
            platform::Platform::Desktop => Self::Desktop,
        }
    }
}

impl From<Input> for seam::Input {
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

impl From<MergePhase> for seam::MergePhase {
    fn from(phase: MergePhase) -> Self {
        match phase {
            MergePhase::Preflight => Self::Preflight,
            MergePhase::Postflight => Self::Postflight,
        }
    }
}

// Narrow the fixture's stamped `BuildReport` to the WIT report: the
// envelope keys (`version`, `slice`, `target`) stay caller-owned on the
// seam, and the fixture never emits findings or a UI surface.
impl From<wire::BuildReport> for Report {
    fn from(report: wire::BuildReport) -> Self {
        Self {
            status: report.status.into(),
            findings: Vec::new(),
            outputs: report.outputs.into_iter().map(BuildOutput::from).collect(),
            ui_surface: None,
        }
    }
}

impl From<wire::BuildStatus> for Status {
    fn from(status: wire::BuildStatus) -> Self {
        match status {
            wire::BuildStatus::Success => Self::Success,
            wire::BuildStatus::Failure => Self::Failure,
        }
    }
}

impl From<wire::BuildOutput> for BuildOutput {
    fn from(output: wire::BuildOutput) -> Self {
        Self {
            platform: output.platform.into(),
            path: output.path,
        }
    }
}
