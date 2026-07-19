//! Single-operation prompt scenarios for fast adapter prompt iteration.
//! Each scenario is a data directory under `<root>/<adapter>/<name>/`.

use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, bail, ensure};
use native::{Catalog, DynModel, ExecutionPaths, Provider, ReferenceMode};
use project::seam::wire::{BuildReport, BuildStatus};
use project::seam::{Input, MergePhase, Target as _, WorkingTree};
use serde::Deserialize;

use crate::fs as evalfs;
use crate::run::ModelFactory;
use crate::telemetry::Telemetry;

/// One scenario's machine-readable routing, from `scenario.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Axis-qualified adapter id (`target:contracts`).
    pub adapter: String,
    /// The seam operation the scenario drives.
    pub operation: Operation,
    /// The slice name the operation runs under.
    pub slice: String,
    /// Scratch-relative paths that must exist after a passing report.
    /// Mandatory and non-empty for `build` scenarios.
    #[serde(default)]
    pub expect: Vec<String>,
}

/// The closed operation set a scenario may drive.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Operation {
    /// The target build operation.
    Build,
    /// The merge preflight gate.
    MergePreflight,
    /// The merge postflight gate.
    MergePostflight,
}

impl Operation {
    /// Kebab-case operation label for run output.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::MergePreflight => "merge-preflight",
            Self::MergePostflight => "merge-postflight",
        }
    }
}

/// Run one scenario by `<adapter>/<name>` id over the supplied native
/// catalog and model factory, or list them all.
///
/// # Errors
///
/// Returns an unknown or malformed scenario, seeding failures, a failing adapter
/// report, and a missing `expect` artifact.
pub async fn run(
    root: &Path, sandbox: &Path, id: Option<&str>, catalog: &Catalog, factory: &ModelFactory,
) -> Result<()> {
    let Some(id) = id else {
        return list(root);
    };
    let dir = root.join(id);
    let config = load(root, &dir, catalog).with_context(|| format!("scenario `{id}`"))?;

    let scratch = materialize_fixture(sandbox, id, &dir)?;
    println!(
        "eval scenario {id}: {} `{}` slice={} scratch={}",
        config.operation.label(),
        config.adapter,
        config.slice,
        scratch.display()
    );

    let instance = (factory)(&scratch)?;
    let telemetry = Telemetry::new(instance.model);
    let model = DynModel::new(telemetry.clone());
    let paths = ExecutionPaths::isolated(&*scratch, scratch.join("project-cache"));
    let provider = Provider::new(paths, model, catalog.clone(), ReferenceMode::Online);
    let inputs = inputs(&dir.join("inputs"))?;
    let report = dispatch(&provider, &config, inputs).await;
    provider.shutdown().await;
    let effective = telemetry.effective_model(instance.default_model.as_deref());
    conclude(id, &scratch, &report?, &config.expect, effective.as_deref())
}

/// Parse and validate the scenario config at `dir/scenario.toml`.
///
/// # Errors
///
/// Returns a missing or unparseable `scenario.toml` and any validation failure.
pub fn load(root: &Path, dir: &Path, catalog: &Catalog) -> Result<Config> {
    let path = dir.join("scenario.toml");
    if !path.is_file() {
        bail!(
            "no scenario.toml at {}; known scenarios: {}",
            path.display(),
            ids(root).unwrap_or_default().join(", ")
        );
    }
    let body = fs::read_to_string(&path)?;
    let config: Config =
        toml::from_str(&body).with_context(|| format!("parsing {}", path.display()))?;
    validate(&config, catalog)?;
    Ok(config)
}

fn validate(config: &Config, catalog: &Catalog) -> Result<()> {
    ensure!(
        catalog.entries().iter().any(|entry| entry.id() == config.adapter),
        "adapter `{}` is not linked into this host",
        config.adapter
    );
    ensure!(!config.slice.trim().is_empty(), "empty slice name");
    if config.operation == Operation::Build {
        ensure!(
            !config.expect.is_empty(),
            "build scenarios must declare at least one `expect` artifact — a success \
             report that produced nothing would otherwise pass as a silent no-op"
        );
    }
    for rel in &config.expect {
        validate_entry(rel).with_context(|| format!("expect entry `{rel}`"))?;
    }
    Ok(())
}

fn validate_entry(rel: &str) -> Result<()> {
    ensure!(!rel.trim().is_empty(), "empty expect entry");
    let path = Path::new(rel);
    ensure!(path.is_relative(), "absolute paths are not allowed");
    ensure!(
        path.components().all(|component| matches!(component, Component::Normal(_))),
        "path components must be plain names (no `..` or `.`)"
    );
    Ok(())
}

/// Gate and persist one run's outcome.
///
/// # Errors
///
/// Returns a failing adapter report, a failed artifact expectation, and
/// report-persistence I/O failures.
pub fn conclude(
    id: &str, scratch: &Path, report: &BuildReport, expect: &[String], model: Option<&str>,
) -> Result<()> {
    for finding in &report.findings {
        eprintln!(
            "finding [{}] {}: {}",
            format!("{:?}", finding.severity).to_lowercase(),
            finding.rule_id.as_deref().unwrap_or("-"),
            finding.title
        );
    }

    let gate = if report.status == BuildStatus::Success {
        enforce_expected(id, scratch, expect)
    } else {
        Ok(())
    };
    let outcome =
        if report.status == BuildStatus::Success && gate.is_ok() { "pass" } else { "fail" };

    let report_path = scratch.join("report.json");
    fs::write(&report_path, serde_json::to_vec_pretty(&envelope(id, outcome, report, model)?)?)?;
    println!("eval scenario {id}: report {}", report_path.display());

    ensure!(
        report.status == BuildStatus::Success,
        "scenario `{id}` failed; report at {}, delta under {}",
        report_path.display(),
        scratch.display()
    );
    gate
}

/// The artifact-exists gate over the scratch tree.
///
/// # Errors
///
/// Returns the first unsatisfied entry, naming the missing path.
pub fn enforce_expected(id: &str, scratch: &Path, expect: &[String]) -> Result<()> {
    let root = scratch.canonicalize().context("canonical scratch root")?;
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
            "scenario `{id}` reported success but produced no `{rel}` under {} — \
             a silent no-op (every sub-flow self-skipped, or the writes landed \
             elsewhere)",
            root.display()
        );
    }
    Ok(())
}

// A symlink escaping the scratch tree never satisfies a gate.
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

/// Atomically allocate a fresh run directory under `base`.
///
/// A process-local counter disambiguates same-second runs, so the
/// `run-<stamp>-<pid>[-<seq>]` name is unique by construction and the
/// single `create_dir` either succeeds or reports a real failure.
///
/// # Errors
///
/// Returns directory-creation failures and clock errors.
pub fn allocate_run_dir(base: &Path) -> Result<PathBuf> {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    fs::create_dir_all(base).with_context(|| format!("creating {}", base.display()))?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let pid = std::process::id();
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name =
        if seq == 0 { format!("run-{stamp}-{pid}") } else { format!("run-{stamp}-{pid}-{seq}") };
    let candidate = base.join(name);
    fs::create_dir(&candidate).with_context(|| format!("creating {}", candidate.display()))?;
    candidate.canonicalize().context("canonical run dir")
}

async fn dispatch(provider: &Provider, config: &Config, inputs: Vec<Input>) -> Result<BuildReport> {
    let tree = WorkingTree {
        base: "eval".to_string(),
        subpath: None,
    };
    let adapter = config.adapter.clone();
    let slice = config.slice.clone();
    let report = match config.operation {
        Operation::Build => provider.build(adapter, slice, inputs, tree).await,
        Operation::MergePreflight => {
            provider.merge(adapter, slice, MergePhase::Preflight, tree).await
        }
        Operation::MergePostflight => {
            provider.merge(adapter, slice, MergePhase::Postflight, tree).await
        }
    };
    report.map_err(|error| anyhow::anyhow!("{} failed: {error:?}", config.operation.label()))
}

fn list(root: &Path) -> Result<()> {
    let mut ids = ids(root)?;
    ids.sort();
    ensure!(!ids.is_empty(), "no scenarios under {}", root.display());
    println!("scenarios (run with `eval scenario <id>`):");
    for id in ids {
        println!("  {id}");
    }
    Ok(())
}

fn ids(root: &Path) -> Result<Vec<String>> {
    let mut ids = Vec::new();
    for adapter in read_dirs(root)? {
        let name = adapter.file_name().unwrap_or_default().to_string_lossy().into_owned();
        for scenario in read_dirs(&adapter)? {
            if scenario.join("scenario.toml").is_file() {
                let id = scenario.file_name().unwrap_or_default().to_string_lossy();
                ids.push(format!("{name}/{id}"));
            }
        }
    }
    Ok(ids)
}

fn read_dirs(dir: &Path) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs)
}

fn materialize_fixture(sandbox: &Path, id: &str, dir: &Path) -> Result<PathBuf> {
    let base = sandbox.join(id);
    let scratch = allocate_run_dir(&base)?;
    let fixture = dir.join("fixture");
    if fixture.is_dir() {
        evalfs::copy_tree(&fixture, &scratch)?;
    }
    Ok(scratch)
}

fn inputs(dir: &Path) -> Result<Vec<Input>> {
    let entries = fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();
    ensure!(!paths.is_empty(), "no `inputs/*.md` under {}", dir.display());

    let mut inputs = Vec::new();
    for path in paths {
        let body =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
        inputs.push(match stem {
            "proposal" => Input::Proposal(body),
            "design" => Input::Design(body),
            "tasks" => Input::Tasks(body),
            stem if stem.starts_with("spec") => Input::Spec(body),
            _ => Input::Other(body),
        });
    }
    Ok(inputs)
}

fn envelope(
    id: &str, outcome: &str, report: &BuildReport, model: Option<&str>,
) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "version": 1,
        "scenario": id,
        "profile": "adapter-live",
        "runtime": "native",
        "model": model.unwrap_or("backend-default"),
        "outcome": outcome,
        "report": serde_json::to_value(report)?,
    }))
}
