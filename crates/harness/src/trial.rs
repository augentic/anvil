//! The generic live-model trial driver: the operator rhythm
//! (`init → plan → execute → finalize → clean`) over a wrapper's
//! linked-adapter catalog, graded by deterministic hooks only.
//!
//! Repository differences stay in the wrapper's [`Profile`]: sandbox
//! and seed paths, init and author argv, the change name, optional
//! scenario roots, and the deterministic `authored` / `grade` hooks.
//! Everything else — provider-plus-MCP construction, phase sequencing,
//! the generic drained/done invariants, and telemetry reporting — is
//! shared.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, ensure};
use change::plan::handlers::{Execute, ExecuteBody, ExecuteInput};
use change::{Entry as PlanEntry, Plan, Status};
use clap::{Parser, Subcommand};

use crate::catalog::Binding;
use crate::model::DevModel;
use crate::provider::{Provider, run as run_op};
use crate::scenario::{self, Scenarios};
use crate::telemetry::{self, Telemetry};
use crate::{command, env, fs as evalfs, sandbox};

/// Deterministic post-author hook over the freshly authored plan.
pub type AuthoredHook = fn(&Path, &Plan) -> Result<()>;

/// Deterministic grading hook after execute, before finalize
/// (`plan.yaml` still live). Receives the drained plan and the typed
/// execute body, so a wrapper can grade phase order as well as
/// artifacts.
pub type GradeHook = fn(&Path, &Plan, &ExecuteBody) -> Result<()>;

/// One repository's trial declaration: data and deterministic hooks,
/// no adapter dependencies.
#[derive(Debug)]
pub struct Profile {
    /// The sandbox root: the trial project, and the parent of scenario
    /// scratch trees (`sandbox/<adapter>/<name>/run-…`).
    pub sandbox: PathBuf,
    /// Optional seed tree copied into the fresh sandbox at init.
    pub seed: Option<PathBuf>,
    /// The init argv tail (e.g. `["init", "contracts", "--name", "demo"]`).
    pub init: Vec<String>,
    /// The plan-author argv tail (e.g. `["plan", "author", "auth", …]`).
    pub author: Vec<String>,
    /// The change name Gate 1 stamps `approved`.
    pub change: String,
    /// Optional deterministic assertions over the authored plan.
    pub authored: Option<AuthoredHook>,
    /// Deterministic grading after execute.
    pub grade: GradeHook,
    /// Optional single-operation prompt-scenario roots.
    pub scenarios: Option<Scenarios>,
}

#[derive(Debug, Parser)]
#[command(name = "eval", about = "Run the live-model trial over the persistent sandbox")]
struct Args {
    #[command(subcommand)]
    phase: Option<Phase>,
}

/// One operation in the persistent manual evaluation workflow.
#[derive(Clone, Debug, Subcommand)]
pub enum Phase {
    /// Scaffold the fresh sandbox project.
    Init,
    /// Author the change and stamp Gate 1 (`approved`).
    Plan,
    /// Drain the loop: refine → build → merge per slice, then grade.
    Execute,
    /// Archive the drained plan.
    Finalize,
    /// Remove the sandbox project.
    Clean,
    /// Run one prompt scenario, or list them all.
    Scenario {
        /// `<adapter>/<scenario>` under the profile's scenarios root.
        id: Option<String>,
    },
}

/// Run the trial from the CLI: one phase, or the full rhythm.
///
/// # Errors
///
/// Returns verb failures, grading failures, and sandbox I/O failures.
pub async fn run<B: Binding>(profile: &Profile, argv: &[String]) -> Result<ExitCode> {
    let cli = Args::parse_from(argv);
    match cli.phase {
        Some(Phase::Init) => init::<B>(profile).await?,
        Some(Phase::Plan) => plan::<B>(profile).await?,
        Some(Phase::Execute) => execute::<B>(profile).await?,
        Some(Phase::Finalize) => finalize::<B>(profile).await?,
        Some(Phase::Clean) => clean(profile)?,
        Some(Phase::Scenario { id }) => {
            let scenarios =
                profile.scenarios.as_ref().context("this wrapper declares no prompt scenarios")?;
            scenario::run::<B>(scenarios, &profile.sandbox, id.as_deref()).await?;
        }
        None => {
            init::<B>(profile).await?;
            plan::<B>(profile).await?;
            execute::<B>(profile).await?;
            finalize::<B>(profile).await?;
            clean(profile)?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

async fn init<B: Binding>(profile: &Profile) -> Result<()> {
    let root = sandbox::replace(&profile.sandbox)?;
    println!("trial project: {}", root.display());
    let _cache = env::scoped_cache(&root);
    if let Some(seed) = &profile.seed {
        evalfs::copy_tree(seed, &root)?;
    }
    let argv: Vec<&str> = profile.init.iter().map(String::as_str).collect();
    command::invoke(&provider::<B>(&root).await, &argv).await
}

async fn plan<B: Binding>(profile: &Profile) -> Result<()> {
    let root = sandbox::require(&profile.sandbox)?;
    println!("trial project: {}", root.display());
    let _cache = env::scoped_cache(&root);
    let provider = provider::<B>(&root).await;

    let argv: Vec<&str> = profile.author.iter().map(String::as_str).collect();
    command::invoke(&provider, &argv).await?;
    let authored = sandbox::read_plan(&root)?;
    ensure!(!authored.entries.is_empty(), "plan author produced no entries");
    if let Some(hook) = profile.authored {
        hook(&root, &authored)?;
    }

    // Gate 1: the operator stamps `approved`.
    command::invoke(&provider, &["plan", "transition", &profile.change, "approved"]).await?;

    telemetry::report(&provider.model().counts(), authored.entries.len());
    Ok(())
}

async fn execute<B: Binding>(profile: &Profile) -> Result<()> {
    let root = sandbox::require(&profile.sandbox)?;
    println!("trial project: {}", root.display());
    let _cache = env::scoped_cache(&root);
    let provider = provider::<B>(&root).await;

    // Typed execution, so grading hooks can inspect the phase list.
    let executed = run_op::<Execute, ExecuteBody, _>(&provider, ExecuteInput {})
        .await
        .map_err(|err| anyhow::anyhow!("plan execute failed: {err}"))?;
    for phase in &executed.phases {
        eprintln!("executed {} {}", phase.step, phase.slice);
    }
    ensure!(executed.status == "drained", "execute must exit drained, got {}", executed.status);

    let plan = sandbox::read_plan(&root)?;
    ensure!(
        plan.entries.iter().all(|entry: &PlanEntry| entry.status == Status::Done),
        "execute must leave every entry done: {:?}",
        plan.entries
    );

    (profile.grade)(&root, &plan, &executed)?;
    telemetry::report(&provider.model().counts(), plan.entries.len());
    Ok(())
}

async fn finalize<B: Binding>(profile: &Profile) -> Result<()> {
    let root = sandbox::require(&profile.sandbox)?;
    println!("trial project: {}", root.display());
    let _cache = env::scoped_cache(&root);
    command::invoke(&provider::<B>(&root).await, &["plan", "archive"]).await
}

fn clean(profile: &Profile) -> Result<()> {
    if profile.sandbox.exists() {
        fs::remove_dir_all(&profile.sandbox).context("cleaning up the trial project")?;
    }
    Ok(())
}

// The live provider: cursor-agent connects lazily on the first
// judgment leg (`DevModel`), so deterministic phases never demand it.
async fn provider<B: Binding>(root: &Path) -> Provider<Telemetry<DevModel>> {
    Provider::bound::<B>(root, Telemetry::new(DevModel::new(root))).await
}
