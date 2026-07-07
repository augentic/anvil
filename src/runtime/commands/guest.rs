//! Guest-owned verb triage: the composed-deployment leg of the binary.
//!
//! The collapsed orchestrator verbs the native handlers refuse (`plan
//! execute`, `plan author`, `slice refine`, `slice build`,
//! `slice merge run`, `source survey`, `source extract`) run in the
//! workflow guest against the composed deployment — workflow guest + adapter guests +
//! the spawning `cursor-agent` model backend — awaited through
//! `specify_runtime::drive` in command mode with the HTTP trigger in
//! the background and the guest exit code passed through to the
//! process exit. This module owns the triage predicate the top-level
//! `run` consults, plus the manifest choice the guest leg runs
//! against: an `omnia.toml` at the project root wins wholesale (the
//! developer posture); absent one, the generated deployment manifest
//! is regenerated in the per-project cache (`commands::deploy` — the
//! one manifest-producing code path, RFC-65) and driven directly. See
//! DECISIONS.md §"One `specify` binary".

use std::fs;
use std::path::Path;

use specify_dispatch::commands::plan::cli::PlanAction;
use specify_dispatch::commands::slice::cli::{SliceAction, SliceMergeAction};
use specify_dispatch::commands::source::cli::SourceAction;
use specify_error::Error;

use crate::runtime::cli::{Commands, Format};
use crate::runtime::commands::deploy::{self, BareMiss};
use crate::runtime::output::{Exit, report};

/// Operator-provided deployment manifest at the project root; when
/// present it replaces the generated manifest wholesale.
const MANIFEST_FILENAME: &str = "omnia.toml";

/// Wire code for a composed-deployment failure ahead of or around the
/// guest run: deployment assembly or backend connect (`cursor-agent`
/// missing from `PATH`). A failure *inside* the guest is not this —
/// the guest renders its own envelope and its exit code passes
/// through. Adapter-resolution failures during manifest regeneration
/// keep their own typed codes (`adapter-not-found`,
/// `adapter-not-installed`, `adapter-digest-mismatch`, …) so the
/// operator sees the same diagnostics the native resolvers raise.
const GUEST_RUNTIME_FAILED: &str = "guest-runtime-failed";

/// Whether the parsed verb is guest-owned: one of the collapsed
/// orchestrator verbs the native handler table refuses. Everything
/// else — the native residue and the pure workflow verbs — keeps
/// running in-process through today's handlers.
pub const fn owned(command: &Commands) -> bool {
    match command {
        Commands::Plan { action } => {
            matches!(action, PlanAction::Execute | PlanAction::Author { .. })
        }
        Commands::Slice { action } => matches!(
            action,
            SliceAction::Refine { .. }
                | SliceAction::Build { .. }
                | SliceAction::Merge {
                    action: SliceMergeAction::Run { .. }
                }
        ),
        Commands::Source { action } => {
            matches!(action, SourceAction::Survey { .. } | SourceAction::Extract { .. })
        }
        _ => false,
    }
}

/// Drive one guest-owned verb through the composed deployment and map
/// the outcome onto the process exit: guest exit codes pass through
/// verbatim; host-side failures render an error envelope in `format`.
pub fn run(format: Format, plan_dir: Option<&Path>) -> Exit {
    match dispatch(plan_dir) {
        Ok(0) => Exit::Success,
        Ok(code) => Exit::Code(code),
        Err(err) => report(format, &err),
    }
}

/// Resolve the deployment manifest (project-root `omnia.toml` or the
/// generated manifest, regenerated fresh for this drive), then block
/// on the composed run, forwarding the process argv (minus `argv[0]` —
/// the runtime core supplies the guest program name) so the guest's
/// clap sees exactly what native clap saw. Regeneration is cheap by
/// construction — filesystem probes, digest-cached describe answers,
/// and one content-digest read per pinned entry (the D4 +
/// committed-lock verification, RFC-65 AC8), never a fetch — so the
/// manifest is always fresh and lock-verified at drive time without a
/// staleness heuristic.
fn dispatch(plan_dir: Option<&Path>) -> Result<u8, Error> {
    let project_dir = std::env::current_dir()
        .map_err(|err| failed(format!("resolving the working directory: {err}")))?;
    check_plan_dir(plan_dir, &project_dir)?;
    let args: Vec<String> = std::env::args().skip(1).collect();

    let committed = project_dir.join(MANIFEST_FILENAME);
    if committed.is_file() {
        return drive(&committed, args);
    }

    let generated = deploy::regenerate(&project_dir, BareMiss::Fail)?;
    drive(&generated, args)
}

/// Refuse a `--plan-dir` (or `SPECIFY_PLAN_DIR`) pointing anywhere but
/// the working directory: the guest anchors plan artifacts at the `"."`
/// preopen, so any other plan root would be silently ignored in-guest.
/// A value that resolves to the working directory itself is a no-op and
/// passes.
fn check_plan_dir(plan_dir: Option<&Path>, project_dir: &Path) -> Result<(), Error> {
    let Some(dir) = plan_dir else {
        return Ok(());
    };
    let same = fs::canonicalize(dir)
        .and_then(|requested| fs::canonicalize(project_dir).map(|cwd| requested == cwd))
        .unwrap_or(false);
    if same {
        return Ok(());
    }
    Err(Error::Argument {
        flag: "--plan-dir",
        detail: format!(
            "`--plan-dir` is native-only on guest-routed verbs: the guest anchors plan artifacts \
             at the working directory, so {} would be ignored; run from the plan root instead",
            dir.display()
        ),
    })
}

fn drive(manifest: &Path, args: Vec<String>) -> Result<u8, Error> {
    specify_runtime::drive(manifest, args).map_err(|err| failed(format!("{err:#}")))
}

const fn failed(detail: String) -> Error {
    Error::Diag {
        code: GUEST_RUNTIME_FAILED,
        detail,
    }
}
