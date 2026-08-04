//! Data-directory eval cases over real `emery` verbs.
//!
//! A case is a directory under the composition root's `cases/` tree
//! holding one `case.toml` (and usually a sibling `fixture/`; a
//! workflow case may instead `clone` an upstream tree into a
//! gitignored `fixture/` cache on first run). Two shapes exist: a [`Workflow`] case drives the operator rhythm
//! (`init` → `plan author` [→ `plan execute` [→ `plan archive`]])
//! and a [`Build`] case invokes
//! `slice build <slice>` once against a committed refined fixture.
//! Every command runs through [`native::command::execute`] — the same
//! public surface operators use — so request assembly, report
//! persistence, journal cadence, and lifecycle transitions are the
//! production paths, never reconstructed here.
//!
//! Each case owns one stable retained sandbox at `<sandbox>/<case>/`.
//! The runner never infers workflow progress from an existing tree:
//! rerun from fresh state with `--restart`, or continue explicitly
//! with `cargo make lab -- --project-dir <sandbox> …`.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use anyhow::{Context as _, Result, bail, ensure};
use native::{CachePlacement, Catalog, DynModel, ExecutionPaths, Locations};
use project::config::Layout;
use project::plan::Status;
use project::slice::{LifecycleStatus, SliceMetadata};
use tracing::Instrument as _;

use crate::telemetry::{self, Telemetry};
use crate::{fs as evalfs, grade, sandbox};

mod spec;

pub use spec::{Build, Case, CloneSpec, Workflow, WorkflowUntil, load, parse};
use spec::{list, validate_entry};

/// Builds one live model backend per case run, rooted at that run's
/// sandbox tree.
pub type ModelFactory = Arc<dyn Fn(&Path) -> Result<DynModel> + Send + Sync>;

/// Run one case by id over the supplied native catalog and model
/// factory, or list every case when `id` is absent.
///
/// `root` is the composition root's `cases/` directory; `sandbox` is
/// the retained per-case sandbox root beside it. `until` overrides a
/// workflow case's configured stop rung and is refused for build
/// cases.
///
/// # Errors
///
/// Returns an unknown or malformed case, an existing sandbox without
/// `--restart`, command failures, and every gate failure.
pub async fn run(
    root: &Path, sandbox: &Path, id: Option<&str>, until: Option<WorkflowUntil>, restart: bool,
    catalog: &Catalog, factory: &ModelFactory,
) -> Result<()> {
    let Some(id) = id else {
        return list(root);
    };
    let case = load(root, id).with_context(|| format!("case `{id}`"))?;
    if matches!(case, Case::Build(_)) {
        ensure!(until.is_none(), "`--until` applies only to workflow cases");
    }

    let span = tracing::info_span!(
        "eval.case",
        case = %id,
        kind = %case.label(),
        until = tracing::field::Empty,
    );
    execute(root, sandbox, id, &case, until, restart, catalog, factory).instrument(span).await
}

// The dispatch behind [`run`]'s `eval.case` span: sandbox policy,
// clone-on-miss fixture population, fixture materialization, then
// the case kind's driver.
#[expect(clippy::too_many_arguments, reason = "internal dispatch kernel; callers use `run`")]
async fn execute(
    root: &Path, sandbox: &Path, id: &str, case: &Case, until: Option<WorkflowUntil>,
    restart: bool, catalog: &Catalog, factory: &ModelFactory,
) -> Result<()> {
    let dir = sandbox.join(id);
    let _lock = sandbox::single_writer(&dir)?;
    if dir.exists() && !restart {
        bail!(
            "sandbox {} already exists; rerun from fresh state with `--restart`, or \
             continue/debug it explicitly with `cargo make lab -- --project-dir {} …`",
            dir.display(),
            dir.display()
        );
    }
    if let Case::Workflow(Workflow {
        clone: Some(clone), ..
    }) = case
    {
        clone_into(&root.join(id).join("fixture"), clone)?;
    }
    let scratch = sandbox::replace(&dir)?;
    if let Some(fixture) = fixture_dir(root, id, case)? {
        evalfs::copy_tree(&fixture, &scratch)
            .with_context(|| format!("materializing the fixture {}", fixture.display()))?;
    }

    let telemetry = Telemetry::new(factory(&scratch)?);
    let model = DynModel::new(telemetry.clone());

    match case {
        Case::Workflow(workflow) => {
            let until = until.unwrap_or(workflow.until);
            tracing::Span::current().record("until", until.label());
            println!(
                "eval case {id}: workflow until {} sandbox {}",
                until.label(),
                scratch.display()
            );
            run_workflow(id, workflow, until, &scratch, &model, catalog, &telemetry).await
        }
        Case::Build(build) => {
            println!("eval case {id}: build slice {} sandbox {}", build.slice, scratch.display());
            run_build(id, build, &scratch, &model, catalog, &telemetry).await
        }
    }
}

async fn run_workflow(
    id: &str, case: &Workflow, until: WorkflowUntil, root: &Path, model: &DynModel,
    catalog: &Catalog, telemetry: &Telemetry<DynModel>,
) -> Result<()> {
    invoke(root, model, catalog, &["init", &case.target]).await?;

    let mut author = vec!["plan".to_string(), "author".to_string(), case.change.clone()];
    if let Some(intent) = &case.intent {
        author.extend(["--intent".to_string(), intent.clone()]);
    }
    for (key, binding) in &case.sources {
        author.extend(["--source".to_string(), format!("{key}={binding}")]);
    }
    let author: Vec<&str> = author.iter().map(String::as_str).collect();
    invoke(root, model, catalog, &author).await?;

    let plan = sandbox::read_plan(root)?;
    ensure!(!plan.entries.is_empty(), "plan author produced no entries");
    ensure!(
        plan.entries.iter().all(|entry| entry.status == Status::Pending),
        "plan author must leave every entry pending: {:?}",
        plan.entries
    );
    let slices_dir = Layout::new(root).slices_dir();
    let no_slices = fs::read_dir(&slices_dir).map_or(true, |mut entries| entries.next().is_none());
    ensure!(no_slices, "plan author must not create slices — execution belongs to `plan execute`");

    if until == WorkflowUntil::Plan {
        telemetry::report(&telemetry.counts(), plan.entries.len());
        println!(
            "eval case {id}: stopped after plan author; continue with \
             `cargo make lab -- --project-dir {} plan execute`",
            root.display()
        );
        return Ok(());
    }

    invoke(root, model, catalog, &["plan", "execute"]).await?;

    let plan = sandbox::read_plan(root)?;
    ensure!(
        plan.entries.iter().all(|entry| entry.status == Status::Done),
        "execute must drain the plan, leaving every entry done: {:?}",
        plan.entries
    );
    grade::provenance(&grade::baseline(root)?)?;
    telemetry::report(&telemetry.counts(), plan.entries.len());

    if until == WorkflowUntil::Finalize {
        invoke(root, model, catalog, &["plan", "archive"]).await?;
    }
    println!("eval case {id}: pass (sandbox retained at {})", root.display());
    Ok(())
}

async fn run_build(
    id: &str, case: &Build, root: &Path, model: &DynModel, catalog: &Catalog,
    telemetry: &Telemetry<DynModel>,
) -> Result<()> {
    invoke(root, model, catalog, &["slice", "build", &case.slice]).await?;

    let slice_dir = Layout::new(root).slice_dir(&case.slice);
    let metadata =
        SliceMetadata::load(&slice_dir).context("loading the slice metadata after the build")?;
    ensure!(
        metadata.status == LifecycleStatus::Built,
        "slice `{}` is `{}` after the build, expected `built`",
        case.slice,
        metadata.status
    );
    let report = slice_dir.join("build").join("report.yaml");
    ensure!(report.is_file(), "no authoritative build report at {}", report.display());

    // RFC-87: build writes land in the captured result snapshot, never
    // the sandbox product tree (code reaches it only at merge).
    // Materialize the result beside the sandbox for the artifact gate
    // and operator inspection.
    let patch_path = slice_dir.join("build").join("patch.yaml");
    let patch: project::snapshot::CodePatch = serde_saphyr::from_str(
        &fs::read_to_string(&patch_path)
            .with_context(|| format!("reading the captured patch {}", patch_path.display()))?,
    )
    .context("parsing build/patch.yaml")?;
    let result_dir = root.join("build-result");
    project::workspace::Store::new(paths(root).locations().snapshots_root())
        .materialize(&patch.result, &result_dir)
        .context("materializing the captured result snapshot")?;
    enforce_expected(id, &result_dir, &case.expect)?;

    telemetry::report(&telemetry.counts(), 1);
    println!("eval case {id}: pass (sandbox {}, report {})", root.display(), report.display());
    Ok(())
}

// The sandbox-relative execution layout every case verb runs under:
// store and cache (and through them the snapshot store and workspaces
// roots) live inside the retained sandbox, so a case leaves one
// self-contained tree behind.
fn paths(root: &Path) -> ExecutionPaths {
    let locations = Locations::explicit(
        root.join("adapter-store"),
        CachePlacement::Parent(root.join("project-cache")),
    );
    ExecutionPaths::new(root, locations)
}

// One `emery` verb through the native command surface, which owns
// the `emery.command` span.
async fn invoke(root: &Path, model: &DynModel, catalog: &Catalog, argv: &[&str]) -> Result<()> {
    let command = argv.join(" ");
    tracing::info!("emery {command}");
    let mut full = vec!["emery".to_string()];
    full.extend(argv.iter().map(ToString::to_string));
    let response =
        native::command::execute(paths(root), model.clone(), catalog.clone(), full).await?;
    io::stdout().write_all(&response.stdout)?;
    io::stderr().write_all(&response.stderr)?;
    ensure!(response.exit == 0, "`emery {command}` exited {}", response.exit);
    Ok(())
}

// Populate the case's gitignored `fixture/` cache on miss: one
// shallow clone with `.git` stripped, reused by every later run.
fn clone_into(cache: &Path, spec: &CloneSpec) -> Result<()> {
    let dest = cache.join(&spec.dest);
    if dest.is_dir() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    tracing::info!("git clone --depth 1 {} {}", spec.url, dest.display());
    let status = Command::new("git")
        .args(["clone", "--depth", "1", &spec.url])
        .arg(&dest)
        .status()
        .context("spawning `git clone`")?;
    ensure!(status.success(), "`git clone {}` failed with {status}", spec.url);
    fs::remove_dir_all(dest.join(".git")).context("stripping the clone's `.git`")?;
    Ok(())
}

// The case's fixture directory: the explicit `fixture` path resolved
// against the case directory, else the sibling `fixture/` when it
// exists. An explicit fixture that is absent fails with a focused
// error (e.g. a shared tree that has moved).
fn fixture_dir(root: &Path, id: &str, case: &Case) -> Result<Option<PathBuf>> {
    let dir = root.join(id);
    let explicit = match case {
        Case::Workflow(workflow) => workflow.fixture.as_ref(),
        Case::Build(build) => build.fixture.as_ref(),
    };
    if let Some(fixture) = explicit {
        let fixture = if fixture.is_absolute() { fixture.clone() } else { dir.join(fixture) };
        ensure!(
            fixture.is_dir(),
            "the case's fixture {} does not exist; prepare it before running this case",
            fixture.display()
        );
        return Ok(Some(fixture));
    }
    let sibling = dir.join("fixture");
    Ok(sibling.is_dir().then_some(sibling))
}

/// The artifact-exists gate over the case sandbox.
///
/// # Errors
///
/// Returns the first unsatisfied entry, naming the missing path.
pub fn enforce_expected(id: &str, scratch: &Path, expect: &[String]) -> Result<()> {
    let root = scratch.canonicalize().context("canonical sandbox root")?;
    for rel in expect {
        validate_entry(rel).with_context(|| format!("expect entry `{rel}`"))?;
        let satisfied = confined(&root, &root.join(rel)).is_some_and(|path| {
            if path.is_dir() {
                holds_a_file(&root, &path, &mut HashSet::new())
            } else {
                path.is_file()
            }
        });
        ensure!(
            satisfied,
            "case `{id}` reported success but produced no `{rel}` under {} — a silent \
             no-op (every sub-flow self-skipped, or the writes landed elsewhere)",
            root.display()
        );
    }
    Ok(())
}

// A symlink escaping the sandbox tree never satisfies a gate.
fn confined(root: &Path, path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    canonical.starts_with(root).then_some(canonical)
}

fn holds_a_file(root: &Path, dir: &Path, visited: &mut HashSet<PathBuf>) -> bool {
    if !visited.insert(dir.to_path_buf()) {
        return false;
    }
    fs::read_dir(dir).into_iter().flatten().flatten().any(|entry| {
        confined(root, &entry.path()).is_some_and(|path| {
            path.is_file() || (path.is_dir() && holds_a_file(root, &path, visited))
        })
    })
}
