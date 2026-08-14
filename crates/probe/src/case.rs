//! Data-directory eval cases over real `emery` verbs.
//!
//! Every command runs through [`native::command::execute`] — production
//! paths, never reconstructed here; rerun from fresh state with `--restart`.

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
use project::slice::SliceMetadata;
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
    // Ingest on the native provider pushes this future past clippy's 16KiB cap.
    Box::pin(execute(root, sandbox, id, &case, until, restart, catalog, factory).instrument(span))
        .await
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
             continue/debug it explicitly with `cargo make lab -- --change-dir {} …`",
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
    if let Some(home) = case_definition_dir(root, id, case) {
        evalfs::copy_tree(&home, &scratch.join("definition"))
            .with_context(|| format!("materializing the definition home {}", home.display()))?;
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
            Box::pin(run_build(id, build, &scratch, &model, catalog, &telemetry)).await
        }
    }
}

async fn run_workflow(
    id: &str, case: &Workflow, until: WorkflowUntil, root: &Path, model: &DynModel,
    catalog: &Catalog, telemetry: &Telemetry<DynModel>,
) -> Result<()> {
    let supplied = definition_home(root, case);
    if supplied.is_none() {
        ensure!(
            !case.target.trim().is_empty(),
            "in-place workflow mint needs `target` for `emery init`"
        );
        invoke(root, model, catalog, &["init", &case.target]).await?;
    }
    let (from, wave) = seed_definition(root, case)?;
    if supplied.is_some() {
        ensure_target_trees(root, &from, &wave, model, catalog).await?;
    }
    invoke(
        root,
        model,
        catalog,
        &["plan", "author", &case.change, "--from", &from.to_string_lossy(), "--wave", &wave],
    )
    .await?;

    let authored = case_layout(root);
    let plan = sandbox::read_plan(root)?;
    ensure!(!plan.entries.is_empty(), "plan author produced no entries");
    let events = project::plan::collect_events(authored)?;
    let ladders = project::plan::project_ladders(&plan, &events);
    ensure!(
        ladders.values().all(|status| *status == Status::Pending),
        "plan author must leave every entry pending: ladders={ladders:?}; entries={:?}",
        plan.entries
    );
    let slices_dir = authored.slices_dir();
    let no_slices = fs::read_dir(&slices_dir).map_or(true, |mut entries| entries.next().is_none());
    ensure!(no_slices, "plan author must not create slices — refinement belongs to `plan refine`");

    if until == WorkflowUntil::Plan {
        telemetry::report(&telemetry.counts(), plan.entries.len());
        println!(
            "eval case {id}: stopped after plan author; continue with \
             `cargo make lab -- --change-dir {} plan refine`",
            root.display()
        );
        return Ok(());
    }

    invoke(root, model, catalog, &["plan", "refine"]).await?;
    if until == WorkflowUntil::Refine {
        telemetry::report(&telemetry.counts(), plan.entries.len());
        println!(
            "eval case {id}: stopped after plan refine; continue with \
             `cargo make lab -- --change-dir {} plan execute`",
            root.display()
        );
        return Ok(());
    }

    invoke(root, model, catalog, &["plan", "execute"]).await?;

    let executed = case_layout(root);
    let plan = sandbox::read_plan(root)?;
    let events = project::plan::collect_events(executed)?;
    let ladders = project::plan::project_ladders(&plan, &events);
    ensure!(
        ladders.values().all(|status| *status == Status::Done),
        "execute must drain the plan (projected done): ladders={ladders:?}"
    );
    grade_accepted(root).await?;
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
    Box::pin(build_phase(root, model, catalog, &case.slice)).await?;

    let slice_dir = Layout::new(root).slice_dir(&case.slice);
    let metadata =
        SliceMetadata::load(&slice_dir).context("loading the slice metadata after the build")?;
    let report = slice_dir.join("build").join("report.yaml");
    ensure!(report.is_file(), "no authoritative build report at {}", report.display());

    // Build writes land in the captured result snapshot, never the
    // sandbox product tree (code reaches it only at merge); materialize
    // it beside the sandbox for the artifact gate and inspection.
    ensure!(
        project::build_record::BuildRecord::present(&slice_dir) || metadata.completed_at.is_some(),
        "slice `{}` has no builds/<digest>.yaml (or completed_at) after the build",
        case.slice,
    );
    let record = project::build_record::BuildRecord::load_latest(&slice_dir)
        .context("loading the fact-substrate build record")?;
    let result_dir = root.join("build-result");
    project::workspace::Store::new(paths(root).locations().snapshots_root())
        .materialize(&record.result, &result_dir)
        .await
        .context("materializing the captured result snapshot")?;
    enforce_expected(id, &result_dir, &case.expect)?;

    telemetry::report(&telemetry.counts(), 1);
    println!("eval case {id}: pass (sandbox {}, report {})", root.display(), report.display());
    Ok(())
}

/// Resolve the case's definition home: explicit `definition` path,
/// sibling `definition/`, or a mint from `intent` / `[sources]`.
///
/// # Errors
///
/// Missing home with no mint inputs; fixture mint failures.
fn seed_definition(root: &Path, case: &Workflow) -> Result<(PathBuf, String)> {
    if let Some(from) = definition_home(root, case) {
        return Ok((from, case.wave.clone().unwrap_or_else(|| "deliver".into())));
    }
    let from = root.join("definition");
    let spec = mint_spec(root, case)?;
    mock::definition::mint(&from, &spec).context("mint definition home")?;
    Ok((from, spec.wave))
}

fn definition_home(root: &Path, _case: &Workflow) -> Option<PathBuf> {
    let candidate = root.join("definition");
    candidate.join("handoffs").is_dir().then_some(candidate)
}

fn case_definition_dir(cases: &Path, id: &str, case: &Case) -> Option<PathBuf> {
    let Case::Workflow(workflow) = case else {
        return None;
    };
    let dir = cases.join(id);
    if let Some(rel) = &workflow.definition {
        let path = if rel.is_absolute() { rel.clone() } else { dir.join(rel) };
        return path.is_dir().then_some(path);
    }
    let sibling = dir.join("definition");
    sibling.is_dir().then_some(sibling)
}

fn mint_spec(root: &Path, case: &Workflow) -> Result<mock::definition::Spec> {
    let explicit_intent = case.intent.clone();
    let intent = explicit_intent.clone().or_else(|| single_value_source(case));
    let adapter = project::config::ProjectConfig::load(root)
        .ok()
        .and_then(|config| config.adapter)
        .unwrap_or_else(|| case.target.clone());
    ensure!(
        !adapter.trim().is_empty(),
        "workflow case needs `definition`, `intent`, a `[sources]` binding, or `target` to mint"
    );
    let mut spec = mock::definition::Spec::degenerate(intent.as_deref().unwrap_or_default());
    if intent.is_none() {
        spec.scopes.clear();
        spec.mappings.clear();
    }
    spec.targets[0].locator = root.display().to_string();
    spec.targets[0].adapter = format!("emery:{adapter}@0.0.0");
    spec.wave = case.wave.clone().unwrap_or(spec.wave);
    let target = spec.targets[0].id.clone();
    let fold_single_value = explicit_intent.is_none() && case.sources.len() == 1;
    for (index, (key, raw)) in case.sources.iter().enumerate() {
        if fold_single_value {
            continue;
        }
        let Some((adapter, rest)) = raw.split_once(':') else {
            bail!("source `{key}` must be `adapter:value:…` or `adapter:<locator>`");
        };
        if let Some(value) = rest.strip_prefix("value:") {
            if key == project::adapter::catalog::INTENT && intent.is_some() {
                continue;
            }
            spec.scopes.push(mock::definition::value_scope(
                key,
                format!("emery:{adapter}@0.0.0"),
                value,
                key,
                u8::try_from(index).unwrap_or(0xf) % 16,
            ));
        } else {
            spec.scopes.push(mock::definition::location_scope(
                key,
                format!("emery:{adapter}@0.0.0"),
                rest,
                key,
                u8::try_from(index).unwrap_or(0xf) % 16,
            ));
        }
        spec.mappings.push(mock::definition::Mapping {
            source: key.clone(),
            lead: key.clone(),
            target: target.clone(),
        });
    }
    ensure!(
        !spec.scopes.is_empty(),
        "workflow case needs `intent`, a `[sources]` binding, or a `definition/` fixture home"
    );
    Ok(spec)
}

fn single_value_source(case: &Workflow) -> Option<String> {
    if case.sources.len() != 1 {
        return None;
    }
    case.sources.values().next().and_then(|raw| {
        raw.split_once(':').and_then(|(_, rest)| rest.strip_prefix("value:")).map(str::to_string)
    })
}

async fn ensure_target_trees(
    root: &Path, from: &Path, wave: &str, model: &DynModel, catalog: &Catalog,
) -> Result<()> {
    let reviewed =
        mock::definition::load_reviewed(from, wave).context("resolve definition home")?;
    for target in &reviewed.handoff.wave.targets {
        let location = project::binding::Location::parse(&target.locator, None)
            .with_context(|| format!("target `{}` locator", target.id))?;
        let project::binding::Locator::Path(rel) = location.locator else {
            continue;
        };
        let tree = if rel.is_absolute() { rel } else { root.join(rel) };
        if project::config::ProjectConfig::load(&tree).is_ok() {
            continue;
        }
        let pin = project::adapter::catalog::fill(
            &project::adapter::catalog::Catalog::first_party(),
            &target.adapter,
        )
        .or_else(|_| project::adapter::catalog::Pin::parse(&target.adapter))
        .with_context(|| format!("target `{}` adapter pin", target.id))?;
        fs::create_dir_all(&tree)
            .with_context(|| format!("creating target tree {}", tree.display()))?;
        invoke(&tree, model, catalog, &["init", &pin.name, "--name", &target.id]).await?;
    }
    Ok(())
}

async fn grade_accepted(root: &Path) -> Result<()> {
    let paths = paths(root);
    let layout = paths.layout();
    let plan = project::plan::Plan::load(&layout.plan_path()).context("loading plan.yaml")?;
    let events = project::plan::collect_events(layout).context("collecting journal events")?;
    let store = project::workspace::Store::new(paths.locations().snapshots_root());
    let mut requirements = Vec::new();
    for id in plan.targets.keys() {
        let Some(cid) = project::wave::accepted_cid(layout, &events, id)
            .with_context(|| format!("accepted CID for target `{id}`"))?
        else {
            continue;
        };
        let dest = root.join(format!("accepted-{id}"));
        store
            .materialize(&cid, &dest)
            .await
            .with_context(|| format!("materializing accepted CID for target `{id}`"))?;
        requirements.extend(grade::baseline(&dest)?);
    }
    ensure!(!requirements.is_empty(), "execute produced no accepted-CID baseline to grade");
    grade::provenance(&requirements)
}

fn case_layout(root: &Path) -> Layout<'_> {
    if in_place(root) { Layout::new(root) } else { Layout::detached(root) }
}

fn in_place(root: &Path) -> bool {
    root.join(".emery").join("project.yaml").is_file()
}

// One build phase over the case's refined fixture, driven straight
// through the shared build orchestration (the execute loop owns the
// phase in production) — one phase, for fast prompt iteration.
async fn build_phase(root: &Path, model: &DynModel, catalog: &Catalog, slice: &str) -> Result<()> {
    tracing::info!("build phase for slice `{slice}`");
    let paths = paths(root);
    let layout = Layout::new(root);
    let provider = native::Provider::new(
        paths.clone(),
        model.clone(),
        catalog.clone(),
        native::ReferenceMode::Online,
    );
    let config = project::config::ProjectConfig::load(layout.project_dir())?;
    let outcome = match project::target_policy::project_adapter(&provider, &config, &paths) {
        Ok(adapter) => slice::orchestrate::build(
            &provider,
            layout,
            jiff::Timestamp::now(),
            slice,
            &adapter.manifest,
        )
        .await
        .map(drop)
        .map_err(anyhow::Error::from),
        Err(err) => Err(err.into()),
    };
    provider.shutdown().await;
    outcome.with_context(|| format!("build phase for slice `{slice}`"))
}

// The sandbox-relative execution layout every case verb runs under:
// store, cache, snapshot, and workspaces roots all live inside the
// retained sandbox, so a case leaves one self-contained tree behind.
fn paths(root: &Path) -> ExecutionPaths {
    let locations = Locations::explicit(
        root.join("adapter-store"),
        CachePlacement::Parent(root.join("project-cache")),
    );
    if in_place(root) {
        ExecutionPaths::new(root, locations)
    } else {
        ExecutionPaths::detached(root, locations)
    }
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
// exists. An absent explicit fixture fails with a focused error.
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
