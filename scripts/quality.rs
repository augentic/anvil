#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
edition = "2024"

[dependencies]
anyhow = "1"
jiff = { version = "0.2", default-features = false, features = ["serde", "std"] }
scenario = { path = "../crates/scenario" }
schema = { path = "../crates/schema" }
serde_json = "1"
specify-live-harness = { path = "../harness/live" }
---

//! The live quality orchestrator: repeated trials of a canonical
//! workflow scenario's live profile, graded in Rust and written as a
//! structured `ScenarioReport` bundle under `quality/runs/`.
//!
//!   cargo make quality -- run native-live [--trials N] [--scenario guest-execute-loop]
//!   cargo make quality -- run wasm-live
//!
//! `native-live` drives the in-process `specify-dev guest-loop` driver
//! (adapters checkout, engine crates patched to this working tree);
//! `wasm-live` links `harness/live`'s subprocess driver over the
//! shipped `specify` binary and the release-built adapter components.
//! Never CI: requires an authenticated cursor-agent on PATH.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use std::{env, fs};

use anyhow::{bail, ensure, Context as _, Result};
use scenario::grade::{Execution, StepResult};
use scenario::{
    catalog, evaluate, grade, Grading, ModelBackend, Outcome, Profile, RunMetadata, Runtime,
    Scenario, ScenarioReport, ScenarioReportVersion, TrialMetrics, TrialResult,
};

/// The engine crates the standalone native harness pins by revision;
/// the native driver build overrides each with this checkout's working
/// tree through generated `--config` patch flags (mirrors `dev.rs`).
const ENGINE_CRATES: [&str; 6] =
    ["artifacts", "error", "scenario", "schema", "transport", "workflow"];

/// The git source the native harness pins its engine crates to.
const ENGINE_GIT: &str = "https://github.com/augentic/specify.git";

struct Quality {
    framework: PathBuf,
    adapters: PathBuf,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("quality: {error:#}");
        std::process::exit(2);
    }
}

fn run() -> Result<()> {
    let current = env::current_dir().context("reading the current directory")?;
    let framework = env_path("SPECIFY_FRAMEWORK")
        .map_or_else(|| current.clone(), |path| absolute(&current, &path));
    let adapters = env_path("SPECIFY_ADAPTERS")
        .map_or_else(|| framework.join("../specify-adapters"), |path| absolute(&current, &path));
    let quality = Quality { framework, adapters };

    let mut args = env::args_os().skip(1);
    let first = args.next();
    let command = match first {
        Some(separator) if separator == "--" => args.next(),
        command => command,
    }
    .and_then(|value| value.into_string().ok())
    .unwrap_or_default();
    let rest: Vec<OsString> = args.collect();
    match command.as_str() {
        "run" => quality.run(&rest),
        _ => bail!("unknown command `{command}`; expected `run <profile> [--trials N] [--scenario <id>]`"),
    }
}

impl Quality {
    fn run(&self, args: &[OsString]) -> Result<()> {
        let (profile_id, trials_override, scenario_id) = parse_run_args(args)?;
        let scenario = catalog::load(&scenario_id)
            .map_err(|error| anyhow::anyhow!("loading scenario `{scenario_id}`: {error}"))?;
        let profile = scenario
            .profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .with_context(|| format!("scenario `{scenario_id}` declares no `{profile_id}` profile"))?;
        ensure!(
            profile.model == ModelBackend::Live,
            "profile `{profile_id}` is not a live profile; the deterministic profiles run as \
             plain tests (`cargo make dev -- check` / the composed suite)"
        );
        ensure!(
            on_path("cursor-agent"),
            "cursor-agent not found on PATH; install it, then `cursor-agent login` or export \
             CURSOR_API_KEY (`cargo make dev -- doctor --live` verifies command-mode credentials)"
        );
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
        let bundle = env_path("RUN_BUNDLE")
            .unwrap_or_else(|| self.framework.join("quality/runs").join(&run_id));
        fs::create_dir_all(bundle.join("trials")).context("creating the run bundle")?;

        let rubrics = evaluate::semantic::Rubrics::load(&self.rubrics_file())
            .map_err(|error| anyhow::anyhow!("loading the rubric catalog: {error}"))?;

        println!("== {run_id}: {trials} trial(s) ==");
        let mut results = Vec::new();
        for trial in 1..=trials {
            let trial_root = bundle.join("trials").join(trial.to_string());
            fs::create_dir_all(&trial_root).context("creating the trial directory")?;
            println!("== trial {trial}/{trials} ==");
            results.push(self.trial(&scenario, profile, &rubrics, trial, &trial_root)?);
        }

        let outcome = if results.iter().all(|trial| trial.outcome == Outcome::Pass) {
            Outcome::Pass
        } else {
            Outcome::Fail
        };
        let report = ScenarioReport {
            version: ScenarioReportVersion,
            scenario: scenario_id.clone(),
            outcome,
            run: RunMetadata {
                id: run_id,
                runner: format!("scripts/quality.rs {profile_id}"),
                revisions: self.revisions()?,
                model: Some(
                    env::var("SPECIFY_EVAL_MODEL")
                        .ok()
                        .filter(|id| !id.trim().is_empty())
                        .unwrap_or_else(|| "cursor-default".to_owned()),
                ),
                prompt_digest: Some(self.prompt_digest(&scenario_id)?),
                component_digests: if profile.runtime == Runtime::Wasm {
                    self.component_digests()?
                } else {
                    BTreeMap::new()
                },
                started_at,
                completed_at: jiff::Timestamp::now(),
            },
            trials: results,
        };
        let path = bundle.join("report.json");
        let body = serde_json::to_string_pretty(&report).context("serialising the report")?;
        fs::write(&path, format!("{body}\n")).context("writing the report")?;
        println!("{}", path.display());
        ensure!(outcome == Outcome::Pass, "run failed; see the bundle for evidence");
        Ok(())
    }

    /// One isolated trial: drive, grade hard assertions, grade the
    /// semantic rubrics, and persist the per-trial artifacts.
    fn trial(
        &self, scenario: &Scenario, profile: &Profile,
        rubrics: &evaluate::semantic::Rubrics, trial: usize, trial_root: &Path,
    ) -> Result<TrialResult> {
        let sandbox = trial_root.join("workspace");
        let log = trial_root.join("driver.log");
        let started = Instant::now();

        let steps = match profile.runtime {
            Runtime::Native => self.drive_native(&sandbox, &log)?,
            Runtime::Wasm => drive_wasm(&self.framework, &self.adapters, &sandbox, &log)?,
        };

        let execution = Execution::new(&sandbox, steps);
        let mut hard_assertions = grade::hard(scenario, &execution);
        evaluate::guest::guest(&mut hard_assertions, &sandbox);

        let mut semantic_rubrics = Vec::new();
        if profile.grading == Grading::Semantic {
            for rubric in &scenario.semantic_rubrics {
                let graded = evaluate::semantic::grade(rubric, rubrics, &sandbox);
                fs::write(trial_root.join("rubric.json"), &graded.raw)
                    .context("writing the rubric verdict")?;
                if !graded.stderr.is_empty() {
                    append(&log, &graded.stderr)?;
                }
                semantic_rubrics.push(graded.result);
            }
        }

        let passed = hard_assertions.iter().all(|result| result.outcome == Outcome::Pass)
            && semantic_rubrics.iter().all(|result| result.outcome == Outcome::Pass);
        let result = TrialResult {
            trial,
            profile: profile.id.clone(),
            outcome: if passed { Outcome::Pass } else { Outcome::Fail },
            hard_assertions,
            semantic_rubrics,
            metrics: TrialMetrics {
                usage_available: false,
                input_tokens: 0,
                output_tokens: 0,
                reasoning_tokens: 0,
                duration_ms: usize::try_from(started.elapsed().as_millis()).unwrap_or(usize::MAX),
            },
            outputs: vec!["driver.log".into(), "rubric.json".into()],
        };
        let body = serde_json::to_string_pretty(&result).context("serialising the trial result")?;
        fs::write(trial_root.join("result.json"), format!("{body}\n"))
            .context("writing the trial result")?;
        println!("trial {trial}: {:?}", result.outcome);
        Ok(result)
    }

    /// Drive one native trial through `specify-dev guest-loop` in the
    /// adapters checkout, its revision-pinned engine crates patched to
    /// this working tree (the same generated `--config` overrides
    /// `dev.rs` uses; the pinned lockfile is snapshotted and restored).
    fn drive_native(&self, sandbox: &Path, log: &Path) -> Result<BTreeMap<String, StepResult>> {
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

        let steps: serde_json::Value = serde_json::from_slice(&output.stdout)
            .context("the native driver did not return step JSON (see driver.log)")?;
        let steps = steps.as_array().context("step JSON is not an array")?;
        steps
            .iter()
            .map(|step| {
                let step: BTreeMap<String, serde_json::Value> =
                    serde_json::from_value(step.clone()).context("malformed step")?;
                let id = step.get("id").and_then(serde_json::Value::as_str).context("step id")?;
                let text = |key: &str| {
                    step.get(key).and_then(serde_json::Value::as_str).unwrap_or_default().to_owned()
                };
                let exit_code = step
                    .get("exit-code")
                    .and_then(serde_json::Value::as_i64)
                    .and_then(|code| i32::try_from(code).ok())
                    .context("step exit code")?;
                Ok((
                    id.to_owned(),
                    StepResult { exit_code, stdout: text("stdout"), stderr: text("stderr") },
                ))
            })
            .collect()
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
        let mut bytes = fs::read(&scenario)
            .with_context(|| format!("reading {}", scenario.display()))?;
        bytes.extend(
            fs::read(self.rubrics_file())
                .with_context(|| format!("reading {}", self.rubrics_file().display()))?,
        );
        Ok(format!("sha256:{}", schema::digest::sha256_hex(&bytes)))
    }

    /// SHA-256 per release-built adapter component in the sibling
    /// checkout, keyed by component name.
    fn component_digests(&self) -> Result<BTreeMap<String, String>> {
        let release = self.adapters.join("target/wasm32-wasip2/release");
        let mut digests = BTreeMap::new();
        for entry in fs::read_dir(&release)
            .with_context(|| format!("reading {}", release.display()))?
        {
            let path = entry.context("release entry")?.path();
            if path.extension().is_some_and(|extension| extension == "wasm")
                && let Some(name) = path.file_stem().and_then(|stem| stem.to_str())
            {
                let bytes = fs::read(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                digests.insert(
                    name.to_owned(),
                    format!("sha256:{}", schema::digest::sha256_hex(&bytes)),
                );
            }
        }
        Ok(digests)
    }
}

/// Drive one wasm trial through the `harness/live` subprocess driver
/// and persist the step transcript as the driver log.
fn drive_wasm(
    framework: &Path, adapters: &Path, sandbox: &Path, log: &Path,
) -> Result<BTreeMap<String, StepResult>> {
    let steps = specify_live_harness::driver::drive(&specify_live_harness::driver::Config {
        framework: framework.to_owned(),
        adapters: adapters.to_owned(),
        sandbox: sandbox.to_owned(),
    })?;
    let mut transcript = String::new();
    for (id, step) in &steps {
        transcript.push_str(&format!(
            "==> {id} (exit {})\n{}{}\n",
            step.exit_code, step.stdout, step.stderr
        ));
    }
    fs::write(log, transcript).context("writing the driver log")?;
    Ok(steps.into_iter().collect())
}

fn parse_run_args(args: &[OsString]) -> Result<(String, Option<usize>, String)> {
    let mut profile = None;
    let mut trials = None;
    let mut scenario = "guest-execute-loop".to_owned();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let arg = arg.to_string_lossy();
        match arg.as_ref() {
            "--trials" => {
                let value = iter.next().context("--trials requires a number")?;
                trials = Some(
                    value.to_string_lossy().parse::<usize>().context("--trials must be a number")?,
                );
            }
            "--scenario" => {
                let value = iter.next().context("--scenario requires an id")?;
                scenario = value.to_string_lossy().into_owned();
            }
            value if !value.starts_with('-') && profile.is_none() => {
                profile = Some(value.to_owned());
            }
            other => bail!("unexpected argument `{other}`"),
        }
    }
    let profile = profile.context("usage: run <profile> [--trials N] [--scenario <id>]")?;
    Ok((profile, trials, scenario))
}

fn append(path: &Path, text: &str) -> Result<()> {
    use std::io::Write as _;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .context("opening the driver log")?;
    file.write_all(text.as_bytes()).context("appending to the driver log")?;
    Ok(())
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

fn on_path(program: &str) -> bool {
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| directory.join(program).is_file())
    })
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from)
}

fn absolute(current: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() { path.to_owned() } else { current.join(path) }
}
