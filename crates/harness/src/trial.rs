//! Live-model workflow trial over explicit command-line inputs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, ensure};
use change::plan::handlers::{Execute, ExecuteBody, ExecuteInput};
use change::{Entry as PlanEntry, Status};
use clap::{Parser, Subcommand};

use crate::catalog::Binding;
use crate::invoke::run as run_op;
use crate::model::DevModel;
use crate::provider::Provider;
use crate::telemetry::{self, Telemetry};
use crate::{command, env, fs as evalfs, grade, sandbox, scenario};

#[derive(Debug, Parser)]
#[command(name = "eval", about = "Run the live-model trial over linked adapters")]
struct Args {
    /// Persistent trial project and scenario scratch root.
    #[arg(long)]
    sandbox: PathBuf,
    /// Optional tree copied into a fresh trial project.
    #[arg(long)]
    seed: Option<PathBuf>,
    /// Target adapter passed to `specify init`.
    #[arg(long)]
    target: Option<String>,
    /// Project name passed to `specify init`.
    #[arg(long)]
    name: Option<String>,
    /// Change name passed to plan author and Gate 1.
    #[arg(long)]
    change: Option<String>,
    /// Optional operator intent passed to plan author.
    #[arg(long)]
    intent: Option<String>,
    /// Source binding passed to plan author; repeat for multiple sources.
    #[arg(long = "source")]
    sources: Vec<String>,
    /// Root containing single-operation prompt scenarios.
    #[arg(long)]
    scenarios: Option<PathBuf>,
    #[command(subcommand)]
    phase: Option<Phase>,
}

#[derive(Clone, Debug, Subcommand)]
enum Phase {
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
        /// `<adapter>/<scenario>` under `--scenarios`.
        id: Option<String>,
    },
}

struct Trial {
    sandbox: PathBuf,
    seed: Option<PathBuf>,
    init: Vec<String>,
    author: Vec<String>,
    change: String,
}

/// Run one phase, or the complete trial when no phase is given.
///
/// # Errors
///
/// Returns argument, command, grading, model, and sandbox failures.
pub(crate) async fn run<B: Binding>(raw: &[String]) -> Result<ExitCode> {
    let args = Args::parse_from(raw);
    if let Some(Phase::Scenario { id }) = &args.phase {
        let scenarios = args.scenarios.as_deref().context("scenario requires --scenarios")?;
        scenario::run::<B>(scenarios, &args.sandbox, id.as_deref()).await?;
        return Ok(ExitCode::SUCCESS);
    }

    let trial = Trial::from_args(&args)?;
    match args.phase {
        Some(Phase::Init) => init::<B>(&trial).await?,
        Some(Phase::Plan) => plan::<B>(&trial).await?,
        Some(Phase::Execute) => execute::<B>(&trial).await?,
        Some(Phase::Finalize) => finalize::<B>(&trial).await?,
        Some(Phase::Clean) => clean(&trial)?,
        Some(Phase::Scenario { .. }) => unreachable!("handled above"),
        None => {
            init::<B>(&trial).await?;
            plan::<B>(&trial).await?;
            execute::<B>(&trial).await?;
            finalize::<B>(&trial).await?;
            clean(&trial)?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

impl Trial {
    fn from_args(args: &Args) -> Result<Self> {
        let target = args.target.as_deref().context("trial requires --target")?;
        let name = args.name.as_deref().context("trial requires --name")?;
        let change = args.change.as_deref().context("trial requires --change")?;
        ensure!(
            args.intent.is_some() || !args.sources.is_empty(),
            "trial requires --intent or at least one --source"
        );

        let init = strings(&["init", target, "--name", name]);
        let mut author = strings(&["plan", "author", change]);
        if let Some(intent) = &args.intent {
            author.extend(["--intent".to_string(), intent.clone()]);
        }
        for source in &args.sources {
            author.extend(["--source".to_string(), source.clone()]);
        }

        Ok(Self {
            sandbox: args.sandbox.clone(),
            seed: args.seed.clone(),
            init,
            author,
            change: change.to_string(),
        })
    }
}

async fn init<B: Binding>(trial: &Trial) -> Result<()> {
    let root = sandbox::replace(&trial.sandbox)?;
    println!("trial project: {}", root.display());
    let _cache = env::scoped_cache(&root);
    if let Some(seed) = &trial.seed {
        evalfs::copy_tree(seed, &root)?;
    }
    invoke(&provider::<B>(&root).await, &trial.init).await
}

async fn plan<B: Binding>(trial: &Trial) -> Result<()> {
    let root = sandbox::require(&trial.sandbox)?;
    println!("trial project: {}", root.display());
    let _cache = env::scoped_cache(&root);
    let provider = provider::<B>(&root).await;

    invoke(&provider, &trial.author).await?;
    let authored = sandbox::read_plan(&root)?;
    ensure!(!authored.entries.is_empty(), "plan author produced no entries");

    command::invoke(&provider, &["plan", "transition", &trial.change, "approved"]).await?;
    telemetry::report(&provider.model().counts(), authored.entries.len());
    Ok(())
}

async fn execute<B: Binding>(trial: &Trial) -> Result<()> {
    let root = sandbox::require(&trial.sandbox)?;
    println!("trial project: {}", root.display());
    let _cache = env::scoped_cache(&root);
    let provider = provider::<B>(&root).await;

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
    grade::provenance(&grade::baseline(&root)?)?;
    telemetry::report(&provider.model().counts(), plan.entries.len());
    Ok(())
}

async fn finalize<B: Binding>(trial: &Trial) -> Result<()> {
    let root = sandbox::require(&trial.sandbox)?;
    println!("trial project: {}", root.display());
    let _cache = env::scoped_cache(&root);
    command::invoke(&provider::<B>(&root).await, &["plan", "archive"]).await
}

fn clean(trial: &Trial) -> Result<()> {
    if trial.sandbox.exists() {
        fs::remove_dir_all(&trial.sandbox).context("cleaning up the trial project")?;
    }
    Ok(())
}

async fn invoke<P>(provider: &P, argv: &[String]) -> Result<()>
where
    P: omnia_guest::api::Provider
        + project::handler::Anchor
        + omnia_guest::Model
        + project::adapter::Resolver
        + project::adapter::Hydrator
        + project::seam::Source
        + project::seam::Target
        + Clone,
{
    let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
    command::invoke(provider, &argv).await
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

async fn provider<B: Binding>(root: &Path) -> Provider<Telemetry<DevModel>> {
    Provider::bound::<B>(root, Telemetry::new(DevModel::new(root))).await
}
