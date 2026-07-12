//! Subprocess driver: the canonical workflow steps against the shipped
//! `specify` binary over a composed live deployment.
//!
//! The binary carries no Specify vocabulary — every verb runs in the
//! workflow guest through omnia's own `run` grammar (`specify run
//! --config <manifest> -- <verb …>`), so this driver stages the
//! sandbox, writes the manifest, and forwards each canonical workflow
//! step as guest argv, capturing per-step results for grading.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{Context as _, Result, anyhow, ensure};
use scenario::grade::StepResult;

use crate::manifest;

/// The canonical scenario this driver executes.
pub const SCENARIO: &str = "guest-execute-loop";

/// Default pinned HTTP trigger address — the runtime serves the
/// `/mcp/<name>` routes here for the agents the live cursor backend
/// spawns; pinning keeps it from colliding with other deployments.
const DEFAULT_HTTP_ADDR: &str = "127.0.0.1:8094";

/// One wasm-live drive: sibling checkout roots plus the trial sandbox.
#[derive(Debug, Clone)]
pub struct Config {
    /// The `specify` checkout (workflow guest + shipped binary).
    pub framework: PathBuf,
    /// The `specify-adapters` checkout (release-built components).
    pub adapters: PathBuf,
    /// Trial project root (created when absent).
    pub sandbox: PathBuf,
}

/// Drive the full loop, returning every captured step in order.
///
/// Steps are `init` first, then the scenario workflow ids. Driving
/// stops at the first failing step; the failure stays in the returned
/// steps for grading.
///
/// # Errors
///
/// Returns setup errors only — missing release components, a failing
/// build, or an unusable sandbox. Step failures are data, not errors.
pub fn drive(config: &Config) -> Result<Vec<(String, StepResult)>> {
    let scenario = scenario::catalog::load(SCENARIO)
        .map_err(|error| anyhow!("loading the canonical scenario: {error}"))?;
    let release = config.adapters.join("target/wasm32-wasip2/release");
    ensure!(
        release.join("omnia.wasm").is_file(),
        "release-built adapter components not found at {}; run `cargo make release` in the \
         sibling specify-adapters checkout",
        release.display()
    );

    // The binary under test and a fresh workflow guest: the manifest
    // points at the debug guest so the loop under test is the branch
    // head, not a published core.
    build(&config.framework, &["build", "-q", "-p", "specify"])?;
    build(
        &config.framework,
        &["build", "-q", "-p", "specify", "--lib", "--target", "wasm32-wasip2"],
    )?;
    let target = env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| config.framework.join("target"), PathBuf::from);
    let specify = target.join("debug/specify");
    let workflow_wasm = target.join("wasm32-wasip2/debug/specify.wasm");
    ensure!(specify.is_file(), "shipped binary not found at {}", specify.display());
    ensure!(workflow_wasm.is_file(), "workflow guest not found at {}", workflow_wasm.display());

    // Stage the sandbox: the project mount, the component-cache mount,
    // the omnia component readable through the `"."` preopen (init
    // mirrors it into the cache), and the deployment manifest.
    let cache = config.sandbox.join(".specify-cache");
    fs::create_dir_all(&cache)
        .with_context(|| format!("creating the sandbox at {}", config.sandbox.display()))?;
    let sandbox = config.sandbox.canonicalize().context("resolving the sandbox root")?;
    let cache = cache.canonicalize().context("resolving the cache root")?;
    fs::copy(release.join("omnia.wasm"), sandbox.join("omnia.wasm"))
        .context("staging the omnia component into the project mount")?;
    let manifest_path = sandbox.join("omnia.toml");
    fs::write(&manifest_path, manifest::omnia_toml(&workflow_wasm, &release, &sandbox, &cache))
        .context("writing the deployment manifest")?;
    let http_addr = env::var("HTTP_ADDR").unwrap_or_else(|_| DEFAULT_HTTP_ADDR.to_owned());

    let run = |argv: &[String]| -> Result<StepResult> {
        eprintln!("==> specify {}", argv.join(" "));
        let output = Command::new(&specify)
            .arg("run")
            .arg("--config")
            .arg(&manifest_path)
            .arg("--")
            .args(argv)
            .current_dir(&sandbox)
            .env("HTTP_ADDR", &http_addr)
            .output()
            .context("spawning the specify binary")?;
        let step = StepResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        };
        eprint!("{}", step.stdout);
        eprint!("{}", step.stderr);
        Ok(step)
    };

    let mut steps = Vec::new();
    // The clerical seed: the guest-supported scaffold leg against the
    // staged omnia component, read through the `"."` preopen — the
    // same pattern the composed replay tests prove. Adapter dispatch
    // rides the manifest's `target:omnia` guest, not the cache.
    let init: Vec<String> =
        ["init", "./omnia.wasm", "--name", "demo", "--scaffold-only"].map(str::to_owned).to_vec();
    let seeded = run(&init)?;
    let failed = seeded.exit_code != 0;
    steps.push(("init".to_owned(), seeded));
    if failed {
        return Ok(steps);
    }

    for step in &scenario.workflow {
        let argv = step.argv().map_err(|error| anyhow!("{error}"))?;
        let result = run(&argv[1..])?;
        let failed = result.exit_code != 0;
        steps.push((step.id.clone(), result));
        if failed {
            break;
        }
    }
    Ok(steps)
}

fn build(framework: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("cargo")
        .args(args)
        .current_dir(framework)
        .status()
        .context("spawning cargo")?;
    ensure!(status.success(), "`cargo {}` failed with {status}", args.join(" "));
    Ok(())
}
