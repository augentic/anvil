//! Deterministic target-adapter behavior and its WIT-mirroring types.

use std::path::{Path, PathBuf};

use crate::Error;

/// One slice-artifact input to a build (the WIT `input` variant).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    /// The slice's `proposal.md` body.
    Proposal(String),
    /// The slice's `design.md` body.
    Design(String),
    /// The slice's `tasks.md` body.
    Tasks(String),
    /// One behavioural spec body.
    Spec(String),
    /// Any additional artifact body.
    Other(String),
}

/// Build status (the WIT `status` enum).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// The build succeeded.
    Success,
    /// The build failed.
    Failure,
}

/// One build output — the path half of the WIT `build-output` record.
/// The fixture only ever builds for the core platform, so the mapping
/// layers stamp `platform: core` when widening.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    /// Project-root-relative output path.
    pub path: String,
}

/// A build or merge report (the WIT `report` record, minus findings
/// and UI surface — the fixture never emits either).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Terminal status.
    pub status: Status,
    /// Core-platform outputs the build wrote.
    pub outputs: Vec<Output>,
}

/// The platforms the fixture's capability shapes mention (the subset
/// of the WIT `platform` enum the fixture ever declares).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// The mandatory core platform.
    Core,
    /// iOS shell support.
    Ios,
    /// Android shell support.
    Android,
}

/// A target's declared platform capability (the WIT
/// `platforms-capability` record).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformsCapability {
    /// Whether the project must declare a platform set at init.
    pub required: bool,
    /// The platforms the target can build for.
    pub allowed: Vec<Platform>,
    /// The set assumed when the operator declares none.
    pub default: Vec<Platform>,
}

/// The platform capability a target identity declares — deterministic
/// per id, so one artifact stands in for several capability shapes
/// (real adapters compile in one answer):
///
/// - a name containing `limited` requires platforms from
///   `{core, ios}`;
/// - a name containing `platforms` requires platforms from
///   `{core, ios, android}`;
/// - anything else is platform-agnostic (`None`).
#[must_use]
pub fn target_platforms(id: &str) -> Option<PlatformsCapability> {
    if id.contains("limited") {
        Some(PlatformsCapability {
            required: true,
            allowed: vec![Platform::Core, Platform::Ios],
            default: vec![Platform::Core, Platform::Ios],
        })
    } else if id.contains("platforms") {
        Some(PlatformsCapability {
            required: true,
            allowed: vec![Platform::Core, Platform::Ios, Platform::Android],
            default: vec![Platform::Core, Platform::Ios, Platform::Android],
        })
    } else {
        None
    }
}

/// Marker file (project-root-relative) that flips builds to a failed
/// report while it exists.
pub const FAIL_BUILD_MARKER: &str = "fixture-fail-build";

/// Directory (project-root-relative) fixture builds write their
/// observable output into.
pub const BUILD_DIR: &str = "fixture-build";

/// The deterministic guidance brief served to synthesis.
///
/// # Errors
///
/// `Internal` when the id selects the `fail-guidance` profile.
pub fn guidance(id: &str) -> Result<String, Error> {
    if id.contains("fail-guidance") {
        return Err(Error::Internal(format!("fixture guidance failure for `{id}`")));
    }
    Ok(format!(
        "Fixture guidance ({id}): keep specs behavioural, one domain per spec; builds write \
         one markdown artifact per slice under `{BUILD_DIR}/`."
    ))
}

/// Build one slice: write the observable artifact under
/// [`BUILD_DIR`] and report it as a core-platform output.
///
/// # Errors
///
/// - `Internal` when the id selects the `fail-build` profile.
/// - `Io` when the artifact cannot be written.
pub fn build(root: &Path, id: &str, slice: &str, inputs: &[Input]) -> Result<Report, Error> {
    if id.contains("fail-build") {
        return Err(Error::Internal(format!("fixture build failure for `{id}`")));
    }
    if id.contains("missing-output") {
        // A dishonest success: the declared output is never written, so
        // the caller's outputs-exist gate must abort the build.
        return Ok(Report {
            status: Status::Success,
            outputs: vec![Output {
                path: format!("{BUILD_DIR}/{slice}-never-written.md"),
            }],
        });
    }
    if root.join(FAIL_BUILD_MARKER).is_file() {
        return Ok(Report {
            status: Status::Failure,
            outputs: Vec::new(),
        });
    }
    let relative = format!("{BUILD_DIR}/{slice}.md");
    let path = root.join(&relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| Error::Io(err.to_string()))?;
    }
    std::fs::write(&path, build_artifact(id, slice, inputs))
        .map_err(|err| Error::Io(err.to_string()))?;
    Ok(Report {
        status: Status::Success,
        outputs: vec![Output { path: relative }],
    })
}

/// Which side of the engine's deterministic core merge a merge gate
/// runs on (the WIT `merge-phase` enum).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergePhase {
    /// Before the deterministic commit.
    Preflight,
    /// After the commit and archive.
    Postflight,
}

/// Marker file (project-root-relative) that flips the preflight merge
/// gate to a failed report while it exists.
pub const FAIL_MERGE_PREFLIGHT_MARKER: &str = "fixture-fail-merge-preflight";

/// Marker file (project-root-relative) that flips the postflight merge
/// gate to a failed report while it exists.
pub const FAIL_MERGE_POSTFLIGHT_MARKER: &str = "fixture-fail-merge-postflight";

/// One phased merge gate: a success report with no outputs, unless the
/// id selects a failure profile or the matching per-phase marker file
/// exists at the project root.
///
/// # Errors
///
/// `Internal` when the id selects the `fail-merge` profile.
pub fn merge(root: &Path, id: &str, _slice: &str, phase: MergePhase) -> Result<Report, Error> {
    if id.contains("fail-merge") {
        return Err(Error::Internal(format!("fixture merge failure for `{id}`")));
    }
    let marker = match phase {
        MergePhase::Preflight => FAIL_MERGE_PREFLIGHT_MARKER,
        MergePhase::Postflight => FAIL_MERGE_POSTFLIGHT_MARKER,
    };
    let status = if root.join(marker).is_file() { Status::Failure } else { Status::Success };
    Ok(Report {
        status,
        outputs: Vec::new(),
    })
}

/// The written build artifact body: slice identity plus per-variant
/// input counts, so tests can assert the build saw its inputs.
fn build_artifact(id: &str, slice: &str, inputs: &[Input]) -> String {
    let mut proposal = 0_usize;
    let mut design = 0_usize;
    let mut tasks = 0_usize;
    let mut specs = 0_usize;
    let mut other = 0_usize;
    for input in inputs {
        match input {
            Input::Proposal(_) => proposal += 1,
            Input::Design(_) => design += 1,
            Input::Tasks(_) => tasks += 1,
            Input::Spec(_) => specs += 1,
            Input::Other(_) => other += 1,
        }
    }
    format!(
        "# Fixture build — {slice}\n\nBuilt by `{id}`.\n\nInputs: proposal {proposal}, design \
         {design}, tasks {tasks}, specs {specs}, other {other}.\n"
    )
}

/// The absolute path of the build artifact [`build`] writes for
/// `slice` — for test assertions.
#[must_use]
pub fn build_artifact_path(root: &Path, slice: &str) -> PathBuf {
    root.join(BUILD_DIR).join(format!("{slice}.md"))
}
