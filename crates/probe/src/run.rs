//! Live-model workflow trial over explicit command-line inputs.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context as _, Result, ensure};
use clap::{Parser, Subcommand};
use native::{CachePlacement, Catalog, DynModel, ExecutionPaths, Locations};
use project::plan::Status;

use crate::telemetry::{self, Telemetry};
use crate::{fs as evalfs, grade, sandbox, scenario};

/// Persistent trial project and scenario scratch root, under the
/// workspace root the composition root supplies.
const SANDBOX: &str = "sandbox";

/// One phase's erased model backend plus the composition root's
/// configured default model id, for effective-model reporting.
#[derive(Clone, Debug)]
pub struct ModelInstance {
    /// The erased live backend rooted at the phase's project tree.
    pub model: DynModel,
    /// The configured default model id, when the composition root
    /// carries one (e.g. `EVAL_MODEL`).
    pub default_model: Option<String>,
}

/// Builds one live model backend per trial phase or scenario run,
/// rooted at that run's project tree.
pub type ModelFactory = Arc<dyn Fn(&Path) -> Result<ModelInstance> + Send + Sync>;

#[derive(Debug, Parser)]
#[command(name = "eval", about = "Run the live-model trial over native adapters")]
struct Args {
    /// Optional tree copied into a fresh trial project.
    #[arg(long)]
    fixture: Option<PathBuf>,
    /// Target adapter passed to `specify init`.
    #[arg(long)]
    target: Option<String>,
    /// Change name passed to plan author and Gate 1.
    #[arg(long)]
    change: Option<String>,
    /// Optional operator intent passed to plan author.
    #[arg(long)]
    intent: Option<String>,
    /// Source binding passed to plan author; repeat for multiple sources.
    #[arg(long = "source")]
    sources: Vec<String>,
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
        /// `<adapter>/<scenario>` under the composition root's scenario root.
        id: Option<String>,
    },
}

struct Trial {
    sandbox: PathBuf,
    fixture: Option<PathBuf>,
    init: Vec<String>,
    author: Vec<String>,
    change: String,
    catalog: Catalog,
    factory: ModelFactory,
}

/// Run one phase, or the complete trial when no phase is given.
///
/// `workspace_root` anchors the persistent `sandbox/` tree and any
/// relative scenario root; eval does not consult process
/// current-directory state after entry.
///
/// # Errors
///
/// Returns argument, command, grading, model, sandbox, and
/// sandbox-lock failures.
pub async fn run(
    workspace_root: PathBuf, catalog: Catalog, model: ModelFactory, args: &[String],
    scenarios: Option<&Path>,
) -> Result<ExitCode> {
    let args = Args::parse_from(args);
    let sandbox = workspace_root.join(SANDBOX);
    if let Some(Phase::Scenario { id }) = &args.phase {
        let scenarios = scenarios.context("this eval composition has no prompt scenarios")?;
        let scenarios = anchored(&workspace_root, scenarios);
        scenario::run(&scenarios, &sandbox, id.as_deref(), &catalog, &model).await?;
        return Ok(ExitCode::SUCCESS);
    }

    let trial = Trial::from_args(&args, &workspace_root, sandbox, catalog, model)?;
    let _guard = sandbox::single_writer(&trial.sandbox)?;
    match args.phase {
        Some(Phase::Init) => trial.init().await?,
        Some(Phase::Plan) => trial.plan().await?,
        Some(Phase::Execute) => trial.execute().await?,
        Some(Phase::Finalize) => trial.finalize().await?,
        Some(Phase::Clean) => trial.clean()?,
        Some(Phase::Scenario { .. }) => unreachable!("handled above"),
        None => {
            trial.init().await?;
            trial.plan().await?;
            trial.execute().await?;
            trial.finalize().await?;
            trial.clean()?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn anchored(workspace_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() { path.to_path_buf() } else { workspace_root.join(path) }
}

impl Trial {
    fn from_args(
        args: &Args, workspace_root: &Path, sandbox: PathBuf, catalog: Catalog,
        factory: ModelFactory,
    ) -> Result<Self> {
        let target = args.target.as_deref().context("trial requires --target")?;
        let change = args.change.as_deref().context("trial requires --change")?;
        ensure!(
            args.intent.is_some() || !args.sources.is_empty(),
            "trial requires --intent or at least one --source"
        );

        let init = ["init", target].map(String::from).to_vec();
        let mut author = ["plan", "author", change].map(String::from).to_vec();
        if let Some(intent) = &args.intent {
            author.extend(["--intent".to_string(), intent.clone()]);
        }
        for source in &args.sources {
            author.extend(["--source".to_string(), source.clone()]);
        }

        Ok(Self {
            sandbox,
            // A relative fixture anchors at the workspace root, not the
            // process current directory.
            fixture: args.fixture.as_deref().map(|fixture| anchored(workspace_root, fixture)),
            init,
            author,
            change: change.to_string(),
            catalog,
            factory,
        })
    }

    /// One phase's telemetry-wrapped model over the trial project.
    fn model(&self, root: &Path) -> Result<(DynModel, Telemetry<DynModel>, Option<String>)> {
        let instance = (self.factory)(root)?;
        let telemetry = Telemetry::new(instance.model);
        Ok((DynModel::new(telemetry.clone()), telemetry, instance.default_model))
    }

    async fn invoke(&self, root: &Path, model: &DynModel, argv: &[&str]) -> Result<()> {
        let display = argv.join(" ");
        eprintln!("==> specify {display}");
        let mut full = vec!["specify".to_string()];
        full.extend(argv.iter().map(ToString::to_string));
        let locations = Locations::explicit(
            root.join("adapter-store"),
            CachePlacement::Parent(root.join("project-cache")),
        );
        let paths = ExecutionPaths::new(root, locations);
        let response =
            native::command::execute(paths, model.clone(), self.catalog.clone(), full).await?;
        io::stdout().write_all(&response.stdout)?;
        io::stderr().write_all(&response.stderr)?;
        ensure!(response.exit == 0, "`specify {display}` exited {}", response.exit);
        Ok(())
    }

    async fn init(&self) -> Result<()> {
        let root = sandbox::replace(&self.sandbox)?;
        println!("trial project: {}", root.display());
        if let Some(fixture) = &self.fixture {
            evalfs::copy_tree(fixture, &root)?;
        }
        let (model, _telemetry, _default) = self.model(&root)?;
        let init: Vec<&str> = self.init.iter().map(String::as_str).collect();
        self.invoke(&root, &model, &init).await
    }

    async fn plan(&self) -> Result<()> {
        let root = sandbox::require(&self.sandbox)?;
        println!("trial project: {}", root.display());
        let (model, telemetry, _default) = self.model(&root)?;

        let author: Vec<&str> = self.author.iter().map(String::as_str).collect();
        self.invoke(&root, &model, &author).await?;
        let authored = sandbox::read_plan(&root)?;
        ensure!(!authored.entries.is_empty(), "plan author produced no entries");

        self.invoke(&root, &model, &["plan", "transition", &self.change, "approved"]).await?;
        telemetry::report(&telemetry.counts(), authored.entries.len());
        Ok(())
    }

    async fn execute(&self) -> Result<()> {
        let root = sandbox::require(&self.sandbox)?;
        println!("trial project: {}", root.display());
        let (model, telemetry, _default) = self.model(&root)?;

        self.invoke(&root, &model, &["plan", "execute"]).await?;

        let plan = sandbox::read_plan(&root)?;
        ensure!(
            plan.entries.iter().all(|entry| entry.status == Status::Done),
            "execute must drain the plan, leaving every entry done: {:?}",
            plan.entries
        );
        grade::provenance(&grade::baseline(&root)?)?;
        telemetry::report(&telemetry.counts(), plan.entries.len());
        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        let root = sandbox::require(&self.sandbox)?;
        println!("trial project: {}", root.display());
        let (model, _telemetry, _default) = self.model(&root)?;
        self.invoke(&root, &model, &["plan", "archive"]).await
    }

    fn clean(&self) -> Result<()> {
        if self.sandbox.exists() {
            fs::remove_dir_all(&self.sandbox).context("cleaning up the trial project")?;
        }
        Ok(())
    }
}
