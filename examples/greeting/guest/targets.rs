use std::path::Path;

use testkit::adapter;

use crate::FixtureAdapter;
use crate::bindings::exports::specify::adapter::target;

impl target::Guest for FixtureAdapter {
    fn metadata(id: target::AdapterId) -> target::AdapterMetadata {
        target::AdapterMetadata {
            specify_floor: None,
            inputs: Vec::new(),
            platforms: adapter::target_platforms(&id).map(wire_platforms),
        }
    }

    async fn guidance(id: target::AdapterId) -> Result<String, target::Error> {
        adapter::guidance(&id).map_err(map_error)
    }

    async fn build(
        id: target::AdapterId, slice: String, inputs: Vec<target::Input>,
        _tree: target::WorkingTree,
    ) -> Result<target::Report, target::Error> {
        // Every guest shares the deployment's `[[mount]]` preopens, so
        // the build writes through its own `"."` preopen.
        let inputs: Vec<adapter::Input> = inputs.into_iter().map(core_input).collect();
        let report = adapter::build(Path::new("."), &id, &slice, &inputs).map_err(map_error)?;
        Ok(wire_report(report))
    }

    async fn merge(
        id: target::AdapterId, slice: String, phase: target::MergePhase, _tree: target::WorkingTree,
    ) -> Result<target::Report, target::Error> {
        let core_phase = match phase {
            target::MergePhase::Preflight => adapter::MergePhase::Preflight,
            target::MergePhase::Postflight => adapter::MergePhase::Postflight,
        };
        let report = adapter::merge(Path::new("."), &id, &slice, core_phase).map_err(map_error)?;
        Ok(wire_report(report))
    }
}

fn map_error(error: adapter::Error) -> target::Error {
    match error {
        adapter::Error::InvalidRequest(detail) => target::Error::InvalidRequest(detail),
        adapter::Error::Io(detail) => target::Error::Io(detail),
        adapter::Error::Internal(detail) => target::Error::Internal(detail),
    }
}

fn wire_platforms(capability: adapter::PlatformsCapability) -> target::PlatformsCapability {
    target::PlatformsCapability {
        required: capability.required,
        allowed: capability.allowed.into_iter().map(wire_platform).collect(),
        default: capability.default.into_iter().map(wire_platform).collect(),
    }
}

const fn wire_platform(platform: adapter::Platform) -> target::Platform {
    match platform {
        adapter::Platform::Core => target::Platform::Core,
        adapter::Platform::Ios => target::Platform::Ios,
        adapter::Platform::Android => target::Platform::Android,
    }
}

fn core_input(input: target::Input) -> adapter::Input {
    match input {
        target::Input::Proposal(body) => adapter::Input::Proposal(body),
        target::Input::Design(body) => adapter::Input::Design(body),
        target::Input::Tasks(body) => adapter::Input::Tasks(body),
        target::Input::Spec(body) => adapter::Input::Spec(body),
        target::Input::Other(body) => adapter::Input::Other(body),
    }
}

fn wire_report(report: adapter::Report) -> target::Report {
    target::Report {
        status: match report.status {
            adapter::Status::Success => target::Status::Success,
            adapter::Status::Failure => target::Status::Failure,
        },
        findings: Vec::new(),
        outputs: report
            .outputs
            .into_iter()
            .map(|output| target::BuildOutput {
                platform: target::Platform::Core,
                path: output.path,
            })
            .collect(),
        ui_surface: None,
    }
}
