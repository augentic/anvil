//! Command-mode tests over the composed walking-skeleton deployment.
//!
//! In-process: build the two-guest deployment from a manifest and drive the
//! workflow guest's `wasi:cli/run` through `omnia::run`, proving the
//! host-mediated `augentic:specify/source` link dispatch end to end (guest
//! stdout is inherited in-process, so these assert the exit path). Subprocess:
//! run the real `specify-runtime` binary against the checked-in `omnia.toml`
//! and assert the printed lead and mount-preopen lines — the full
//! walking-skeleton proof.

use anyhow::Result;
use omnia::{DeploymentBuilder, ExitStatus, Mode};

use crate::common::{self, Bundle, ECHO_WASM, Quiet, WORKFLOW_WASM};

// Drive one command-mode run of the skeleton deployment, with the echo guest
// registered under `echo_id`.
async fn run_command(echo_id: &str) -> Result<ExitStatus> {
    let manifest = common::skeleton_manifest(echo_id)?;
    let builder =
        DeploymentBuilder::new().config(manifest.path().to_path_buf()).mode(Mode::Command);
    omnia::run::<Bundle, Quiet>(builder).await
}

// The workflow guest's survey("source:echo") dispatches through the link to
// the echo guest, its preopen table carries the manifest's `"."` mount, and
// the run exits 0 (the guest hard-fails on a missing mount).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch() -> Result<()> {
    let status = run_command("source:echo").await?;
    assert_eq!(status.code(), 0, "composed command run exits 0");
    Ok(())
}

// With the echo guest registered under a different id, the survey dispatch
// finds no target and the run must not succeed — either the guest observes
// the failure and exits nonzero, or the dispatch error surfaces as a trap.
// The guest checks survey before the mount, so this fails for the bad id.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_source() {
    // An Err from the run (a host-side dispatch error surfacing as a trap) is
    // also a failed run, so only an Ok status needs the nonzero assertion.
    if let Ok(status) = run_command("source:other").await {
        assert_ne!(status.code(), 0, "survey against an unregistered id must not exit 0");
    }
}

// The real binary + the checked-in omnia.toml: stdout carries the echoed lead
// and the mount-preopen proof.
#[test]
fn binary_stdout() -> Result<()> {
    let engine = common::workspace_root();
    for file in [ECHO_WASM, WORKFLOW_WASM] {
        common::guest_wasm(file);
        // omnia.toml resolves guest paths relative to itself, so the built
        // artifacts must sit under engine/target (the default target dir).
        let expected = engine.join("target").join("wasm32-wasip2").join("debug").join(file);
        assert!(
            expected.exists(),
            "omnia.toml expects {expected}; run `cargo make build-guests` from engine/",
            expected = expected.display()
        );
    }

    // An ephemeral port keeps the background HTTP trigger from colliding with
    // parallel test runs; command mode exits when the CLI guest finishes.
    let port = free_port()?;
    let output = assert_cmd::Command::cargo_bin("specify-runtime")?
        .current_dir(&engine)
        .env("HTTP_ADDR", format!("127.0.0.1:{port}"))
        .args(["run", "--config", "omnia.toml"])
        .output()?;

    assert!(
        output.status.success(),
        "runtime exited {:?}; stderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("lead: echo.md#L1 — echo lead from source:echo"),
        "stdout did not carry the echoed lead:\n{stdout}"
    );
    assert!(
        stdout.contains("mount: . ok"),
        "stdout did not carry the mount-preopen proof:\n{stdout}"
    );
    Ok(())
}

// A port the OS just handed out and released — free at bind time with high
// probability.
fn free_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}
