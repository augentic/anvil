//! The live quality orchestrator: repeated trials of a canonical
//! workflow scenario's live profile, graded in Rust and persisted as a
//! [`scenario::bundle`] under `quality/runs/`.
//!
//! ```text
//! cargo make quality -- run native-live [--trials N] [--scenario guest-execute-loop]
//! cargo make quality -- run wasm-live
//! ```
//!
//! `native-live` drives the in-process `specify-dev guest-loop` driver
//! (adapters checkout, engine crates patched to this working tree);
//! `wasm-live` hosts the composed deployment in-process through the
//! [`quality::executor::ComposedExecutor`] over the freshly built
//! workflow guest and the release-built adapter components. Never CI:
//! requires an authenticated cursor-agent on PATH.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use std::{env, fs};

use anyhow::{Context as _, Result, ensure};
use clap::Parser;
use quality::executor::{ComposedExecutor, Executor as _};
use quality::judge::LiveJudge;
use quality::manifest;
use scenario::bundle::Bundle;
use scenario::grade::{Evaluators, Execution, StepResult};
use scenario::{
    AssertionId, ModelBackend, Outcome, RunMetadata, Runtime, Scenario, ScenarioReport,
    ScenarioReportVersion, catalog, evaluate,
};

/// The engine crates the standalone native harness pins by revision;
/// the native driver build overrides each with this checkout's working
/// tree through generated `--config` patch flags (mirrors `dev.rs`).
const ENGINE_CRATES: [&str; 6] =
    ["artifacts", "error", "scenario", "schema", "transport", "workflow"];

/// The git source the native harness pins its engine crates to.
const ENGINE_GIT: &str = "https://github.com/augentic/specify.git";

/// Live quality orchestrator over the canonical workflow scenarios.
#[derive(Debug, Parser)]
#[command(name = "quality", about = "Run live quality trials and write a report bundle")]
enum Cli {
    /// Run repeated live trials of one profile.
    Run {
        /// Live profile id (`native-live` or `wasm-live`).
        profile: String,
        /// Trial count override (defaults to the profile's declared
        /// count; the `TRIALS` environment variable wins over both).
        #[arg(long)]
        trials: Option<usize>,
        /// Canonical scenario id.
        #[arg(long, default_value = "guest-execute-loop")]
        scenario: String,
    },
}

struct Quality {
    framework: PathBuf,
    adapters: PathBuf,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(error) = run(cli).await {
        eprintln!("quality: {error:#}");
        std::process::exit(2);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let current = env::current_dir().context("reading the current directory")?;
    let framework = env_path("SPECIFY_FRAMEWORK")
        .map_or_else(|| current.clone(), |path| absolute(&current, &path));
    let adapters = env_path("SPECIFY_ADAPTERS")
        .map_or_else(|| framework.join("../specify-adapters"), |path| absolute(&current, &path));
    let quality = Quality { framework, adapters };
    let Cli::Run {
        profile,
        trials,
        scenario,
    } = cli;
    quality.run(&profile, trials, &scenario).await
}

impl Quality {
    async fn run(
        &self, profile_id: &str, trials_override: Option<usize>, scenario_id: &str,
    ) -> Result<()> {
        let scenario = catalog::load(scenario_id)
            .map_err(|error| anyhow::anyhow!("loading scenario `{scenario_id}`: {error}"))?;
        let profile =
            scenario.profiles.iter().find(|profile| profile.id == profile_id).with_context(
                || format!("scenario `{scenario_id}` declares no `{profile_id}` profile"),
            )?;
        ensure!(
            profile.model == ModelBackend::Live,
            "profile `{profile_id}` is not a live profile; the deterministic profiles run as \
             plain tests (`cargo make dev -- check` / the replay suite)"
        );
        let judge = LiveJudge::connect().await.context(
            "cursor-agent not runnable; install it, then `cursor-agent login` or export \
             CURSOR_API_KEY (`cargo make dev -- doctor --live` verifies command-mode credentials)",
        )?;
        let trials = env::var("TRIALS")
            .ok()
            .map(|value| value.parse::<usize>().context("TRIALS must be a number"))
            .transpose()?
            .or(trials_override)
            .unwrap_or(profile.trials);
        ensure!(trials > 0, "at least one trial is required");

        let started_at = jiff::Timestamp::now();
        let stamp = started_at.strftime("%Y%m%dT%H%M%SZ").to_string();
        let run_id = format!("{scenario_id}-{profile_id}-{stamp}");
        let bundle = Bundle::new(
            env_path("RUN_BUNDLE")
                .unwrap_or_else(|| self.framework.join("quality/runs").join(&run_id)),
        );

        let rubrics = evaluate::semantic::Rubrics::load(&self.rubrics_file())
            .map_err(|error| anyhow::anyhow!("loading the rubric catalog: {error}"))?;
        let evaluators = Evaluators::default()
            .with(AssertionId::GuestJournalCadence, evaluate::guest::journal_cadence)
            .with(
                AssertionId::GuestGeneratedCrateVerifies,
                quality::verify::generated_crates_verify,
            );
        let executor = if profile.runtime == Runtime::Wasm {
            Some(self.wasm_executor(&scenario)?)
        } else {
            None
        };

        println!("== {run_id}: {trials} trial(s) ==");
        let mut results = Vec::new();
        for trial in 1..=trials {
            bundle.create_trial(trial).context("creating the trial directory")?;
            println!("== trial {trial}/{trials} ==");

            let sandbox = bundle.workspace(trial);
            let log = bundle.driver_log(trial);
            let started = Instant::now();
            let execution = match &executor {
                None => Execution::new(&sandbox, self.drive_native(&sandbox, &log)?),
                Some(executor) => {
                    let execution = executor.execute(&scenario, profile, &sandbox).await?;
                    write_driver_log(&log, &execution)?;
                    execution
                }
            };
            let duration = usize::try_from(started.elapsed().as_millis()).unwrap_or(usize::MAX);
            let setting = quality::trial::Setting {
                scenario: &scenario,
                profile,
                evaluators: &evaluators,
                rubrics: &rubrics,
                bundle: &bundle,
            };
            let result =
                quality::trial::grade(&setting, &execution, &judge, trial, duration).await?;
            println!("trial {trial}: {:?}", result.outcome);
            results.push(result);
        }

        let outcome = if results.iter().all(|trial| trial.outcome == Outcome::Pass) {
            Outcome::Pass
        } else {
            Outcome::Fail
        };
        let report = ScenarioReport {
            version: ScenarioReportVersion,
            scenario: scenario_id.to_owned(),
            outcome,
            run: self.metadata(
                run_id,
                profile_id,
                scenario_id,
                profile.runtime,
                &judge,
                started_at,
            )?,
            trials: results,
        };
        scenario::bundle::validate(&scenario, &report)
            .map_err(|error| anyhow::anyhow!("report completeness: {error}"))?;
        let path = bundle.write_report(&report).context("writing the report")?;
        println!("{}", path.display());
        ensure!(outcome == Outcome::Pass, "run failed; see the bundle for evidence");
        Ok(())
    }

    /// Assemble the run-level provenance and timing record.
    fn metadata(
        &self, run_id: String, profile_id: &str, scenario_id: &str, runtime: Runtime,
        judge: &LiveJudge, started_at: jiff::Timestamp,
    ) -> Result<RunMetadata> {
        Ok(RunMetadata {
            id: run_id,
            runner: format!("quality {profile_id}"),
            revisions: self.revisions()?,
            model: Some(
                env::var("SPECIFY_EVAL_MODEL")
                    .ok()
                    .filter(|id| !id.trim().is_empty())
                    .unwrap_or_else(|| "cursor-default".to_owned()),
            ),
            judge_model: Some(judge.model_identity()),
            prompt_digest: Some(self.prompt_digest(scenario_id)?),
            component_digests: if runtime == Runtime::Wasm {
                self.component_digests()?
            } else {
                BTreeMap::new()
            },
            started_at,
            completed_at: jiff::Timestamp::now(),
        })
    }

    /// Assemble the composed live executor: build the workflow guest
    /// from this working tree, deploy the sibling checkout's
    /// release-built adapter components, and — when the scenario
    /// declares no `init` step of its own — seed the clerical
    /// scaffold leg against the staged omnia component.
    fn wasm_executor(&self, scenario: &Scenario) -> Result<ComposedExecutor> {
        let release = self.adapters.join("target/wasm32-wasip2/release");
        ensure!(
            release.join("omnia.wasm").is_file(),
            "release-built adapter components not found at {}; run `cargo make release` in the \
             sibling specify-adapters checkout",
            release.display()
        );
        let status = Command::new("cargo")
            .args(["build", "-q", "-p", "specify", "--lib", "--target", "wasm32-wasip2"])
            .current_dir(&self.framework)
            .status()
            .context("spawning cargo to build the workflow guest")?;
        ensure!(status.success(), "building the workflow guest failed with {status}");
        let target = env::var_os("CARGO_TARGET_DIR")
            .map_or_else(|| self.framework.join("target"), PathBuf::from);
        let workflow = target.join("wasm32-wasip2/debug/specify.wasm");
        ensure!(workflow.is_file(), "workflow guest not found at {}", workflow.display());

        let mut executor = ComposedExecutor::live(workflow)
            .fixtures_root(&self.framework)
            .stage(release.join("omnia.wasm"), "omnia.wasm");
        for id in manifest::ADAPTERS {
            let name = id.split_once(':').map_or(id, |(_, name)| name);
            executor = executor.adapter(id, release.join(format!("{name}.wasm")));
        }
        if !scenario.workflow.iter().any(|step| step.id == "init") {
            executor = executor.seed(["init", "./omnia.wasm", "--name", "demo", "--scaffold-only"]);
        }
        Ok(executor)
    }

    /// Drive one native trial through `specify-dev guest-loop` in the
    /// adapters checkout, its revision-pinned engine crates patched to
    /// this working tree (the same generated `--config` overrides
    /// `dev.rs` uses; the pinned lockfile is snapshotted and restored).
    fn drive_native(&self, sandbox: &Path, log: &Path) -> Result<Vec<(String, StepResult)>> {
        let manifest = self.adapters.join("harness/native/Cargo.toml");
        let lock = self.adapters.join("harness/native/Cargo.lock");
        let saved = fs::read(&lock).ok();
        let mut command = Command::new("cargo");
        command.current_dir(&self.adapters);
        for name in ENGINE_CRATES {
            command.arg("--config");
            command.arg(format!(
                "patch.\"{ENGINE_GIT}\".{name}.path=\"{}\"",
                self.framework.join("crates").join(name).display()
            ));
        }
        command
            .arg("run")
            .arg("-q")
            .arg("--manifest-path")
            .arg(&manifest)
            .arg("--")
            .arg("guest-loop")
            .arg("--sandbox")
            .arg(sandbox);
        let output = command.output().context("running the native guest-loop driver")?;
        if let Some(bytes) = saved {
            fs::write(&lock, bytes).context("restoring the pinned native-harness lockfile")?;
        }
        fs::write(log, &output.stderr).context("writing the driver log")?;

        serde_json::from_slice(&output.stdout)
            .context("the native driver did not return step JSON (see driver.log)")
    }

    fn rubrics_file(&self) -> PathBuf {
        self.framework.join("quality/rubrics/semantic.yaml")
    }

    fn revisions(&self) -> Result<BTreeMap<String, String>> {
        Ok(BTreeMap::from([
            ("specify".to_owned(), git_head(&self.framework)?),
            ("specify-adapters".to_owned(), git_head(&self.adapters)?),
        ]))
    }

    /// Digest of the model-facing inputs: the canonical scenario
    /// document and the shared rubric catalog.
    fn prompt_digest(&self, scenario_id: &str) -> Result<String> {
        let scenario = self.framework.join("quality/scenarios").join(format!("{scenario_id}.yaml"));
        let mut bytes =
            fs::read(&scenario).with_context(|| format!("reading {}", scenario.display()))?;
        bytes.extend(
            fs::read(self.rubrics_file())
                .with_context(|| format!("reading {}", self.rubrics_file().display()))?,
        );
        Ok(format!("sha256:{}", schema::digest::sha256_hex(&bytes)))
    }

    /// SHA-256 per adapter component the deployment manifest names,
    /// keyed by dispatch id.
    fn component_digests(&self) -> Result<BTreeMap<String, String>> {
        let release = self.adapters.join("target/wasm32-wasip2/release");
        let mut digests = BTreeMap::new();
        for id in manifest::ADAPTERS {
            let name = id.split_once(':').map_or(id, |(_, name)| name);
            let path = release.join(format!("{name}.wasm"));
            let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            digests.insert(id.to_owned(), format!("sha256:{}", schema::digest::sha256_hex(&bytes)));
        }
        Ok(digests)
    }
}

/// Persist the per-step transcript of a composed trial as the driver
/// log.
fn write_driver_log(log: &Path, execution: &Execution) -> Result<()> {
    let mut transcript = String::new();
    for (id, step) in execution.steps() {
        use std::fmt::Write as _;
        let _ = writeln!(
            transcript,
            "==> {id} (exit {})\n{}{}",
            step.exit_code, step.stdout, step.stderr
        );
    }
    fs::write(log, transcript).context("writing the driver log")
}

fn git_head(repository: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repository)
        .output()
        .context("running git rev-parse")?;
    ensure!(
        output.status.success(),
        "git rev-parse failed in {}: {}",
        repository.display(),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from)
}

fn absolute(current: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() { path.to_owned() } else { current.join(path) }
}
