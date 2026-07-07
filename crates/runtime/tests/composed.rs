//! Command-mode tests over the composed deployment, driving the real
//! workflow guest shim (RFC-61 Step 4, Milestone D).
//!
//! In-process: build the two-guest deployment from a manifest and drive the
//! workflow guest's `wasi:cli/run` through `omnia::run` with real argv,
//! proving the wasip3 argv seam and the exit-code passthrough end to end
//! (guest stdout is inherited in-process, so these assert the exit path).
//! Subprocess: run the replay binary against the checked-in
//! `omnia.toml` and assert the shim's stdout. The adapter link dispatch
//! itself (survey/extract/build through `specify:adapter/source`/`target`)
//! needs a scaffolded `.specify/` project in the mount and lands with the
//! Milestone F composed workflow tests.

use anyhow::Result;
use omnia::{DeploymentBuilder, ExitStatus, Mode};

use crate::common::{self, Bundle, Quiet, WORKFLOW_WASM};

// Drive one command-mode run of the composed deployment with the given
// guest argv (argv[0], the program name, is supplied by the runtime core),
// with the echo guest registered under `echo_id`.
async fn run_command(echo_id: &str, args: &[&str]) -> Result<ExitStatus> {
    let manifest = common::skeleton_manifest(echo_id)?;
    let builder = DeploymentBuilder::new()
        .config(manifest.path().to_path_buf())
        .mode(Mode::Command)
        .args(args.iter().map(ToString::to_string).collect::<Vec<_>>());
    omnia::run::<Bundle, Quiet>(builder).await
}

// The happy exit path: `--version` parses through the shared grammar and
// the run exits 0 — the deployment composes (both `specify:adapter` link
// imports resolve), argv reaches clap, and success returns through
// `wasi:cli/run`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch() -> Result<()> {
    let status = run_command("source:echo", &["--version"]).await?;
    assert_eq!(status.code(), 0, "composed command run exits 0");
    Ok(())
}

// Nonzero exit-code passthrough: a pure workflow verb against the empty
// mount fails `not-initialized` (exit 1), and the guest carries the exact
// code through `wasi:cli/exit#exit-with-code` — not a bare trap.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exit_passthrough() -> Result<()> {
    let status = run_command("source:echo", &["plan", "status"]).await?;
    assert_eq!(status.code(), 1, "not-initialized must pass exit 1 through");
    Ok(())
}

// clap's usage-error contract passes through too: an unknown verb exits 2,
// matching the native binary.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn usage_error_passthrough() -> Result<()> {
    let status = run_command("source:echo", &["no-such-verb"]).await?;
    assert_eq!(status.code(), 2, "clap usage errors must pass exit 2 through");
    Ok(())
}

// The real binary + the checked-in omnia.toml: stdout carries the shared
// grammar's version line, proving argv forwarding (`-- --version`) through
// the subprocess surface — and that the full N+1 manifest (workflow + the
// eight committed adapter guests) composes. Targets the replay sibling
// (`specify-runtime-replay`): the `specify` binary's cursor-bound guest
// leg requires `cursor-agent` on PATH at backend connect, which CI must
// not.
#[test]
fn binary_stdout() -> Result<()> {
    let engine = common::workspace_root();
    common::guest_wasm(WORKFLOW_WASM);
    // omnia.toml resolves guest paths relative to itself, so the built
    // workflow artifact must sit under the repo-root target/ (the default target
    // dir) and the committed adapter guests in the sibling checkout.
    let expected = engine.join("target").join("wasm32-wasip2").join("debug").join(WORKFLOW_WASM);
    assert!(
        expected.exists(),
        "omnia.toml expects {expected}; run `cargo make build-guests` from the repo root",
        expected = expected.display()
    );
    common::adapters_root();

    // An ephemeral port keeps the background HTTP trigger from colliding with
    // parallel test runs; command mode exits when the CLI guest finishes.
    let port = free_port()?;
    let output = assert_cmd::Command::cargo_bin("specify-runtime-replay")?
        .current_dir(&engine)
        .env("HTTP_ADDR", format!("127.0.0.1:{port}"))
        .args(["run", "--config", "omnia.toml", "--", "--version"])
        .output()?;

    assert!(
        output.status.success(),
        "runtime exited {:?}; stderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version_line = format!("specify {}", env!("CARGO_PKG_VERSION"));
    assert!(stdout.contains(&version_line), "stdout did not carry `{version_line}`:\n{stdout}");
    Ok(())
}

// The bare host form (RFC-65 move 2): the generic `specify-host`
// binary — the macro-generated command-mode runtime over the
// cursor-bound backends — drives a manifest directly as
// `specify-host run --config <manifest> -- --version`. A stub
// `cursor-agent` satisfies the backend's connect probe; argv reaches
// the workflow guest and exit 0 passes through with the version line
// on stdout.
#[cfg(unix)]
#[test]
fn host_binary_stdout() -> Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    let manifest = common::skeleton_manifest("source:echo")?;

    let stub_dir = tempfile::tempdir()?;
    let stub = stub_dir.path().join("cursor-agent");
    std::fs::write(
        &stub,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"cursor-agent 0.0.0-stub\"; exit 0; fi\nexit 1\n",
    )?;
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755))?;
    let path =
        format!("{}:{}", stub_dir.path().display(), std::env::var("PATH").unwrap_or_default());

    let port = free_port()?;
    let output = assert_cmd::Command::cargo_bin("specify-host")?
        .env("HTTP_ADDR", format!("127.0.0.1:{port}"))
        .env("PATH", path)
        .env_remove("RUST_LOG")
        .args(["run", "--config"])
        .arg(manifest.path())
        .args(["--", "--version"])
        .output()?;

    assert!(
        output.status.success(),
        "host exited {:?}; stderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version_line = format!("specify {}", env!("CARGO_PKG_VERSION"));
    assert!(stdout.contains(&version_line), "stdout did not carry `{version_line}`:\n{stdout}");
    Ok(())
}

// A port the OS just handed out and released — free at bind time with high
// probability.
fn free_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}
