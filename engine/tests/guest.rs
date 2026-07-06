//! Triage-main integration tests: guest-owned verbs route through the
//! composed deployment, native verbs stay in-process.
//!
//! The guest leg needs `cursor-agent` on `PATH` at backend connect, so
//! these tests stage a stub script that answers `--version` and fails
//! any real invocation — the covered flows never legitimately reach a
//! completion. The composed run itself is real: the embedded workflow
//! guest is staged into a transient manifest, adapter guests are
//! discovered from the project tree, and the guest's exit code passes
//! through to the process exit. See DECISIONS.md §"One `specify`
//! binary".

// The stub is a `sh` script; the covered behavior is identical across
// unix hosts and no CI leg runs the suite on Windows.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::OnceLock;

use common::{parse_json, repo_root, specify_cmd};
use tempfile::{TempDir, tempdir};

use crate::common;

// A `cursor-agent` stub on its own PATH dir: answers `--version` (the
// backend's connect probe) and fails anything else.
fn stub_cursor_agent() -> TempDir {
    let dir = tempdir().expect("stub dir");
    let stub = dir.path().join("cursor-agent");
    fs::write(
        &stub,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"cursor-agent 0.0.0-stub\"; exit 0; fi\nexit 1\n",
    )
    .expect("write stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    dir
}

// PATH for the guest leg: the stub dir first, then the ambient PATH
// (the binary spawns nothing else, but keeping the tail is harmless).
fn stub_path(stub_dir: &Path) -> String {
    let ambient = std::env::var("PATH").unwrap_or_default();
    format!("{}:{ambient}", stub_dir.display())
}

// The built echo source-adapter guest (exports the source seam plus an
// MCP shelf over `wasi:http`), self-built so a bare `cargo nextest run`
// works without a prior `cargo make build-guests`.
fn echo_guest_wasm() -> PathBuf {
    static BUILT: OnceLock<()> = OnceLock::new();
    let target = repo_root().join("target");
    BUILT.get_or_init(|| {
        let status = StdCommand::new("cargo")
            .env("CARGO_TARGET_DIR", &target)
            .args(["build", "-p", "specify-echo-guest", "--target", "wasm32-wasip2"])
            .current_dir(repo_root())
            .status()
            .expect("spawning echo guest build");
        assert!(status.success(), "echo guest build failed with status {status}");
    });
    let wasm = target.join("wasm32-wasip2").join("debug").join("specify_echo_guest.wasm");
    assert!(wasm.exists(), "echo guest not found at {}", wasm.display());
    wasm
}

// A port the OS just handed out and released — free at bind time with
// high probability.
fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    listener.local_addr().expect("local addr").port()
}

// The guest leg end to end: `plan execute` (refused natively with
// `argument`/exit 2) reaches the workflow guest inside the composed
// deployment, fails there against the empty mount, and the guest's
// envelope and exit code pass through — proving triage routing, the
// transient-manifest assembly, and the exit-code passthrough at once.
#[test]
fn guest_verb_exit_passthrough() {
    let stub = stub_cursor_agent();
    let project = tempdir().expect("project dir");

    let output = specify_cmd()
        .current_dir(project.path())
        .env("PATH", stub_path(stub.path()))
        .env_remove("RUST_LOG")
        .args(["plan", "execute", "--format", "json"])
        .output()
        .expect("running specify");

    assert_eq!(output.status.code(), Some(1), "guest exit 1 must pass through");
    let envelope = parse_json(&output.stderr);
    assert_eq!(
        envelope["error"], "not-initialized",
        "the verb must fail inside the guest (native dispatch refuses it as `argument`)"
    );
}

// The background HTTP trigger in command mode: with a discovered
// adapter guest exporting `wasi:http` the trigger binds an ephemeral
// port and logs its listening line while the CLI guest runs.
#[test]
fn guest_verb_http_trigger_background() {
    let stub = stub_cursor_agent();
    let project = tempdir().expect("project dir");
    let echo_dir = project.path().join("adapters").join("sources").join("echo");
    fs::create_dir_all(&echo_dir).expect("adapter dir");
    fs::copy(echo_guest_wasm(), echo_dir.join("guest.wasm")).expect("stage echo guest");

    let output = specify_cmd()
        .current_dir(project.path())
        .env("PATH", stub_path(stub.path()))
        .env("HTTP_ADDR", format!("127.0.0.1:{}", free_port()))
        .env("RUST_LOG", "info")
        .args(["plan", "execute", "--format", "json"])
        .output()
        .expect("running specify");

    assert_eq!(output.status.code(), Some(1), "guest exit 1 must pass through");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("http server listening on"),
        "the HTTP trigger must serve in the background during the command run:\n{combined}"
    );
    assert!(combined.contains("not-initialized"), "the guest envelope must reach stderr");
}

// Triage routing for the full guest-owned set: with no `cursor-agent`
// on PATH each verb fails at backend connect with the host-side
// `guest-runtime-failed` envelope — reaching the composed-deployment
// leg at all distinguishes it from the native `argument` refusal.
#[test]
fn guest_owned_verbs_route_to_guest_leg() {
    let empty = tempdir().expect("empty PATH dir");
    let project = tempdir().expect("project dir");
    for argv in [
        vec!["plan", "execute"],
        vec!["plan", "author", "demo-change", "--intent", "demo"],
        vec!["slice", "refine", "demo-slice"],
    ] {
        let output = specify_cmd()
            .current_dir(project.path())
            .env("PATH", empty.path())
            .env_remove("RUST_LOG")
            .args(&argv)
            .arg("--format")
            .arg("json")
            .output()
            .expect("running specify");

        assert_eq!(output.status.code(), Some(1), "host-side connect failure exits 1: {argv:?}");
        let envelope = parse_json(&output.stderr);
        assert_eq!(
            envelope["error"], "guest-runtime-failed",
            "verb {argv:?} must route to the composed-deployment leg"
        );
    }
}

// Native residue is untouched by triage: a non-guest verb dispatches
// in-process and needs no `cursor-agent`, no guests, no deployment.
#[test]
fn native_verbs_stay_in_process() {
    let empty = tempdir().expect("empty PATH dir");
    let project = tempdir().expect("project dir");

    let output = specify_cmd()
        .current_dir(project.path())
        .env("PATH", empty.path())
        .args(["plan", "status", "--format", "json"])
        .output()
        .expect("running specify");

    assert_eq!(output.status.code(), Some(1));
    let envelope = parse_json(&output.stderr);
    assert_eq!(envelope["error"], "not-initialized", "native dispatch must run in-process");
}
