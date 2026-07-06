//! Guest-owned verb triage: the composed-deployment leg of the binary.
//!
//! The collapsed orchestrator verbs the native handlers refuse (`plan
//! execute`, `plan author`, `slice refine`) run in the workflow guest
//! against the composed deployment — workflow guest + adapter guests +
//! the spawning `cursor-agent` model backend — awaited through
//! `specify_runtime::drive` in command mode with the HTTP trigger in
//! the background and the guest exit code passed through to the
//! process exit. This module owns the triage predicate the top-level
//! `run` consults, plus the deployment manifest the guest leg runs
//! against: an `omnia.toml` at the project root wins wholesale; absent
//! one, a transient manifest is assembled from the embedded workflow
//! guest, the committed adapter guests discovered under the manifest
//! cache and the in-repo `adapters/` tree, and the project root as the
//! writable `"."` mount. See DECISIONS.md §"One `specify` binary".

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use specify_dispatch::commands::plan::cli::PlanAction;
use specify_dispatch::commands::slice::cli::SliceAction;
use specify_error::Error;
use specify_workflow::adapter::{Axis, adapter_axis_dir, cache_axis_dir};

use crate::runtime::cli::{Commands, Format};
use crate::runtime::output::{Exit, report};

/// Committed adapter guest component beside `adapter.yaml`.
const GUEST_WASM_FILENAME: &str = "guest.wasm";

/// Operator-provided deployment manifest at the project root; when
/// present it replaces the transient assembly wholesale.
const MANIFEST_FILENAME: &str = "omnia.toml";

/// Wire code for a composed-deployment failure ahead of or around the
/// guest run: manifest staging, deployment assembly, or backend
/// connect (`cursor-agent` missing from `PATH`). A failure *inside*
/// the guest is not this — the guest renders its own envelope and its
/// exit code passes through.
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
        Commands::Slice { action } => matches!(action, SliceAction::Refine { .. }),
        _ => false,
    }
}

/// Drive one guest-owned verb through the composed deployment and map
/// the outcome onto the process exit: guest exit codes pass through
/// verbatim; host-side failures render an error envelope in `format`.
pub fn run(format: Format) -> Exit {
    match dispatch() {
        Ok(0) => Exit::Success,
        Ok(code) => Exit::Code(code),
        Err(err) => report(format, &err),
    }
}

/// Resolve the deployment manifest (project-root `omnia.toml` or the
/// transient assembly), then block on the composed run, forwarding the
/// process argv (minus `argv[0]` — the runtime core supplies the guest
/// program name) so the guest's clap sees exactly what native clap saw.
fn dispatch() -> Result<u8, Error> {
    let project_dir = std::env::current_dir()
        .map_err(|err| failed(format!("resolving the working directory: {err}")))?;
    let args: Vec<String> = std::env::args().skip(1).collect();

    let committed = project_dir.join(MANIFEST_FILENAME);
    if committed.is_file() {
        return drive(&committed, args);
    }

    let staged = stage_manifest(&project_dir)?;
    drive(&staged.manifest, args)
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

/// A staged transient deployment: the manifest path plus the temp tree
/// that owns it (and the materialised workflow guest), kept alive for
/// the duration of the run.
struct Staged {
    manifest: PathBuf,
    _dir: tempfile::TempDir,
}

/// Materialise the embedded workflow guest and write the transient
/// manifest beside it: one workflow guest linking the source + target
/// seams, every discovered adapter guest with its `/mcp/<name>` route,
/// and the project root as the writable `"."` mount — the same shape
/// the composed integration tests deploy.
fn stage_manifest(project_dir: &Path) -> Result<Staged, Error> {
    let dir = tempfile::TempDir::new()
        .map_err(|err| failed(format!("creating the deployment staging directory: {err}")))?;
    let workflow = dir.path().join("workflow.wasm");
    fs::write(&workflow, specify_runtime::WORKFLOW_GUEST_WASM).map_err(|err| {
        failed(format!("staging the workflow guest at {}: {err}", workflow.display()))
    })?;

    let doc = manifest_document(&workflow, project_dir, &discover_adapters(project_dir))
        .map_err(|err| failed(format!("assembling the deployment manifest: {err}")))?;
    let manifest = dir.path().join(MANIFEST_FILENAME);
    fs::write(&manifest, doc).map_err(|err| {
        failed(format!("writing the deployment manifest at {}: {err}", manifest.display()))
    })?;

    Ok(Staged { manifest, _dir: dir })
}

/// One discovered adapter guest: the deployment guest id
/// (`<axis>:<name>`), the adapter name (its `/mcp/<name>` route), and
/// the committed component path.
struct AdapterGuest {
    id: String,
    name: String,
    wasm: PathBuf,
}

/// Discover committed adapter guests: for each axis, probe the manifest
/// cache first and the in-repo `adapters/` tree second (the resolver's
/// precedence order), taking every `<name>/guest.wasm` and letting the
/// first hit per `(axis, name)` win. Discovery is manifest-driven — no
/// adapter names are special-cased.
fn discover_adapters(project_dir: &Path) -> Vec<AdapterGuest> {
    let mut guests = Vec::new();
    for axis in [Axis::Source, Axis::Target] {
        let mut seen = BTreeSet::new();
        for root in [cache_axis_dir(project_dir, axis), adapter_axis_dir(project_dir, axis)] {
            for (name, wasm) in committed_guests(&root) {
                if seen.insert(name.clone()) {
                    guests.push(AdapterGuest {
                        id: format!("{axis}:{name}"),
                        name,
                        wasm,
                    });
                }
            }
        }
    }
    guests
}

/// The `(name, guest.wasm)` pairs under one axis root, name-sorted for
/// a deterministic manifest. An absent or unreadable root is simply
/// empty — adapter guests are optional per verb.
fn committed_guests(root: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let wasm = entry.path().join(GUEST_WASM_FILENAME);
            wasm.is_file().then_some((name, wasm))
        })
        .collect();
    found.sort();
    found
}

/// Render the transient deployment manifest document.
fn manifest_document(
    workflow: &Path, mount: &Path, adapters: &[AdapterGuest],
) -> Result<String, std::fmt::Error> {
    let mut doc = format!(
        "[[guest]]\n\
         id = \"workflow\"\n\
         source.path = \"{workflow}\"\n\
         link = [\"augentic:specify/source@0.1.0\", \"augentic:specify/target@0.1.0\"]\n\n",
        workflow = workflow.display(),
    );
    for adapter in adapters {
        writeln!(
            doc,
            "[[guest]]\nid = \"{id}\"\nsource.path = \"{wasm}\"\n",
            id = adapter.id,
            wasm = adapter.wasm.display(),
        )?;
    }
    writeln!(doc, "[[mount]]\nname = \".\"\npath = \"{}\"\nwritable = true\n", mount.display())?;
    for adapter in adapters {
        writeln!(
            doc,
            "[[route.http]]\nprefix = \"/mcp/{name}\"\nguest = \"{id}\"\n",
            name = adapter.name,
            id = adapter.id,
        )?;
    }
    doc.push_str("[transport]\ndefault = \"in-process\"\n");
    Ok(doc)
}
