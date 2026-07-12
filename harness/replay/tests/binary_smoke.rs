//! Shipped-binary subprocess smoke: the `run` CLI shell, config
//! loading, and the `omnia::runtime!` host wiring.
//!
//! The in-process replay tests host the deployment through the
//! composed executor, so nothing else in CI spawns the shipped binary.
//! This one hard-only trial keeps that seam owned: `specify run
//! --config <manifest> -- init …` against the echo target fixture,
//! graded with the canonical `composed-init` assertions. Requires
//! `cargo build -p specify` alongside the guest builds (the composed
//! workflow builds it).

#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};
use std::process::Command;

use quality::manifest::Manifest;
use scenario::grade::{Execution, StepResult};
use scenario::{Outcome, Scenario};

#[test]
fn run_config_init_scaffolds() {
    let scenario = Scenario::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../quality/scenarios/composed-init.yaml"),
    )
    .expect("canonical composed-init scenario");

    let project = tempfile::tempdir().expect("project mount");
    let cache = tempfile::tempdir().expect("cache mount");
    std::fs::copy(echo_target_wasm(), project.path().join("echo-target.wasm"))
        .expect("staging the target fixture");
    let manifest_path = project.path().join("omnia.toml");
    std::fs::write(&manifest_path, manifest(project.path(), cache.path()))
        .expect("writing the deployment manifest");

    let argv = scenario.workflow[0].argv().expect("init step splits");
    let output = Command::new(specify_bin())
        .arg("run")
        .arg("--config")
        .arg(&manifest_path)
        .arg("--")
        .args(&argv[1..])
        .current_dir(project.path())
        .output()
        .expect("spawning the shipped binary");

    let execution = Execution::new(
        project.path(),
        [(
            "init".to_owned(),
            StepResult {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            },
        )],
    );
    let results = scenario::grade::hard(&scenario, &execution);
    for result in &results {
        assert_eq!(
            result.outcome,
            Outcome::Pass,
            "hard assertion `{}` failed: {:?}\nstderr:\n{}",
            result.id,
            result.detail,
            execution.step("init").map(|step| step.stderr.as_str()).unwrap_or_default()
        );
    }
}

fn manifest(project: &Path, cache: &Path) -> String {
    Manifest::workflow(&workflow_wasm())
        .guest("target:echo-target", &echo_target_wasm())
        .mount(".", project, true)
        .mount("/specify-cache", cache, true)
        .render()
}

fn specify_bin() -> PathBuf {
    let path = target_dir().join("debug/specify");
    assert!(
        path.is_file(),
        "shipped binary not found at {}; run `cargo build -p specify`",
        path.display()
    );
    path
}

fn workflow_wasm() -> PathBuf {
    guest_wasm("specify.wasm")
}

fn echo_target_wasm() -> PathBuf {
    guest_wasm("examples/echo_target.wasm")
}

fn guest_wasm(relative: &str) -> PathBuf {
    let path = target_dir().join("wasm32-wasip2/debug").join(relative);
    assert!(
        path.is_file(),
        "guest `{relative}` not found at {}; run `cargo make guests` in harness/",
        path.display()
    );
    path
}

fn target_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("manifest is <workspace>/harness/replay")
        .join("target")
}
