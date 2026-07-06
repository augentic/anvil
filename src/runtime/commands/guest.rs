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
//! `run` consults, plus the deployment manifest the guest leg runs
//! against: an `omnia.toml` at the project root wins wholesale; absent
//! one, a transient manifest is assembled from the embedded workflow
//! guest, the adapter components resolved for the project's bound
//! adapters (plus a scan of the project component cache for unbound
//! ones), and the project root as the writable `"."` mount. See
//! DECISIONS.md §"One `specify` binary".

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use specify_dispatch::commands::plan::cli::PlanAction;
use specify_dispatch::commands::slice::cli::{SliceAction, SliceMergeAction};
use specify_dispatch::commands::source::cli::SourceAction;
use specify_error::Error;
use specify_workflow::adapter::{AdapterRef, Axis, SourceAdapter, TargetAdapter};
use specify_workflow::change::Plan;
use specify_workflow::config::{Layout, ProjectConfig};
use specify_workflow::init::adapter_ref_from_value;

use crate::runtime::cli::{Commands, Format};
use crate::runtime::output::{Exit, report};

/// Operator-provided deployment manifest at the project root; when
/// present it replaces the transient assembly wholesale.
const MANIFEST_FILENAME: &str = "omnia.toml";

/// Wire code for a composed-deployment failure ahead of or around the
/// guest run: manifest staging, deployment assembly, or backend
/// connect (`cursor-agent` missing from `PATH`). A failure *inside*
/// the guest is not this — the guest renders its own envelope and its
/// exit code passes through. Adapter-resolution failures during
/// assembly keep their own typed codes (`adapter-not-found`,
/// `adapter-digest-mismatch`, …) so the operator sees the same
/// diagnostics the native resolvers raise.
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
/// transient assembly), then block on the composed run, forwarding the
/// process argv (minus `argv[0]` — the runtime core supplies the guest
/// program name) so the guest's clap sees exactly what native clap saw.
fn dispatch(plan_dir: Option<&Path>) -> Result<u8, Error> {
    let project_dir = std::env::current_dir()
        .map_err(|err| failed(format!("resolving the working directory: {err}")))?;
    check_plan_dir(plan_dir, &project_dir)?;
    let args: Vec<String> = std::env::args().skip(1).collect();

    let committed = project_dir.join(MANIFEST_FILENAME);
    if committed.is_file() {
        return drive(&committed, args);
    }

    let staged = stage_manifest(&project_dir)?;
    drive(&staged.manifest, args)
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

    let doc = manifest_document(&workflow, project_dir, &discover_adapters(project_dir)?)
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

/// Discover adapter components with the resolvers' precedence.
///
/// Two legs, first hit per `(axis, name)` winning:
///
/// 1. **Bound adapters through the axis resolvers.** The target bound
///    in `project.yaml` (`adapter:`) and each source bound in
///    `plan.yaml` (`sources.<key>.adapter`) resolve through
///    [`TargetAdapter::resolve`] / [`SourceAdapter::resolve`] — the
///    single-file store entry for a pinned identity (verify-on-read
///    included), the project component cache or the development
///    release build for a bare name. The resolved location *is* the
///    component file.
/// 2. **Component-cache scan for unbound adapters** (`plan author`
///    runs before `plan.yaml` binds sources; a locally-initialized
///    component may not be bound yet): every `*.wasm` in the project
///    component cache, with the axis sniffed from the component's own
///    exports ([`specify_runtime::describe::sniff_axis`]) — a file
///    exporting neither axis interface is skipped, not an error.
fn discover_adapters(project_dir: &Path) -> Result<Vec<AdapterGuest>, Error> {
    let mut guests = Vec::new();
    let mut seen: BTreeSet<(&'static str, String)> = BTreeSet::new();

    for (axis, adapter_ref) in bound_adapters(project_dir)? {
        if !seen.insert((axis.dir_segment(), adapter_ref.name.clone())) {
            continue;
        }
        let wasm = match axis {
            Axis::Source => {
                SourceAdapter::resolve(&adapter_ref, project_dir)?.location.path().clone()
            }
            Axis::Target => {
                TargetAdapter::resolve(&adapter_ref, project_dir)?.location.path().clone()
            }
        };
        guests.push(adapter_guest(axis, adapter_ref.name, wasm));
    }

    for (name, wasm) in cached_components(project_dir) {
        let Ok(Some(axis)) = specify_runtime::describe::sniff_axis(&wasm).map(|sniffed| {
            sniffed.map(|axis| match axis {
                specify_runtime::describe::DescribeAxis::Source => Axis::Source,
                specify_runtime::describe::DescribeAxis::Target => Axis::Target,
            })
        }) else {
            continue;
        };
        if seen.insert((axis.dir_segment(), name.clone())) {
            guests.push(adapter_guest(axis, name, wasm));
        }
    }
    Ok(guests)
}

/// The `(axis, AdapterRef)` pairs the project binds: the `project.yaml`
/// target adapter (skipped for adapter-less workspaces) and every
/// `plan.yaml` source binding. Both files are optional — a bare
/// directory binds nothing and the guest reports its own
/// `not-initialized`.
fn bound_adapters(project_dir: &Path) -> Result<Vec<(Axis, AdapterRef)>, Error> {
    let layout = Layout::new(project_dir);
    let mut bound = Vec::new();
    if layout.config_path().is_file() {
        let config = ProjectConfig::load(project_dir)?;
        if let Some(value) = config.adapter.as_deref() {
            bound.push((Axis::Target, adapter_ref_from_value(value)));
        }
    }
    let plan_path = layout.plan_path();
    if plan_path.is_file() {
        let plan = Plan::load(&plan_path)?;
        for binding in plan.sources.values() {
            bound.push((
                Axis::Source,
                AdapterRef {
                    name: binding.adapter.clone(),
                    version: binding.version.clone(),
                },
            ));
        }
    }
    Ok(bound)
}

fn adapter_guest(axis: Axis, name: String, wasm: PathBuf) -> AdapterGuest {
    AdapterGuest {
        id: format!("{axis}:{name}"),
        name,
        wasm,
    }
}

/// The `(name, component)` pairs in the project component cache
/// (`<project-cache>/components/<name>.wasm`), name-sorted for a
/// deterministic manifest. An absent or unreadable cache is simply
/// empty — adapter components are optional per verb.
fn cached_components(project_dir: &Path) -> Vec<(String, PathBuf)> {
    let root = specify_workflow::adapter::component_cache_dir(project_dir);
    let Ok(entries) = fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut found: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_stem()?.to_str()?.to_owned();
            (path.extension().is_some_and(|ext| ext == "wasm") && path.is_file())
                .then_some((name, path))
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
         source.path = {workflow}\n\
         link = [\"specify:adapter/source@0.1.0\", \"specify:adapter/target@0.1.0\"]\n\n",
        workflow = toml_string(workflow),
    );
    for adapter in adapters {
        writeln!(
            doc,
            "[[guest]]\nid = \"{id}\"\nsource.path = {wasm}\n",
            id = adapter.id,
            wasm = toml_string(&adapter.wasm),
        )?;
    }
    writeln!(doc, "[[mount]]\nname = \".\"\npath = {}\nwritable = true\n", toml_string(mount))?;
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

/// A host path as a TOML basic string: quoted, with `\` and `"` escaped
/// so Windows separators or quote characters in temp paths cannot break
/// the manifest parse.
fn toml_string(path: &Path) -> String {
    let escaped = path.display().to_string().replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}
