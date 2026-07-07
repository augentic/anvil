//! Triage-main integration tests: guest-owned verbs route through the
//! composed deployment, native verbs stay in-process.
//!
//! The guest leg spawns the generic `specify-host` binary (RFC-65
//! move 2), whose cursor backend needs `cursor-agent` on `PATH` at
//! connect, so these tests stage a stub script that answers
//! `--version` and fails any real invocation — the covered flows never
//! legitimately reach a completion. The composed run itself is real:
//! the deployment manifest is regenerated into the per-project cache
//! (RFC-65) with the embedded workflow guest staged beside it, adapter
//! guests resolve through the axis resolvers (bound adapters) or the
//! component-cache scan (unbound), and the guest's exit code passes
//! through to the process exit. A failure *inside* the host —
//! deployment assembly, backend connect — surfaces as the host's own
//! stderr text with its exit code passed through; only a failure
//! around the spawn itself renders the `guest-runtime-failed`
//! envelope. See DECISIONS.md §"One `specify` binary".

// The stub is a `sh` script; the covered behavior is identical across
// unix hosts and no CI leg runs the suite on Windows.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::OnceLock;

use common::{ensure_host_binary, parse_json, repo_root, specify_cmd};
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
// generated-manifest assembly, and the exit-code passthrough at once.
// The drive regenerates the deployment manifest into the per-project
// cache (RFC-65: one manifest-producing code path, no transient
// assembly), staging the embedded workflow guest beside it.
#[test]
fn guest_verb_exit_passthrough() {
    ensure_host_binary();
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

    let deployment = common::expected_cache_dir(project.path()).join("deployment");
    assert!(
        deployment.join("omnia.toml").is_file(),
        "the drive must regenerate the deployment manifest in the project cache"
    );
    assert!(
        deployment.join("workflow.wasm").is_file(),
        "the embedded workflow guest must be staged in the deployment tenant"
    );
}

// The developer posture is untouched: a project-root omnia.toml wins
// wholesale over the generated manifest. The staged file is garbage,
// so the deployment build fails *inside* the spawned host — its own
// stderr text, exit 1 passed through — proving the committed manifest
// was consumed rather than a regenerated one (which would reach the
// guest and fail `not-initialized`).
#[test]
fn project_root_manifest_wins() {
    ensure_host_binary();
    let stub = stub_cursor_agent();
    let project = tempdir().expect("project dir");
    fs::write(project.path().join("omnia.toml"), "this is not a manifest").expect("write garbage");

    let output = specify_cmd()
        .current_dir(project.path())
        .env("PATH", stub_path(stub.path()))
        .env_remove("RUST_LOG")
        .args(["plan", "execute", "--format", "json"])
        .output()
        .expect("running specify");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("building runtime"),
        "the committed manifest must be driven wholesale and fail the host's deployment build:\n{stderr}"
    );
}

// AC7 posture on the guest leg: a `project.yaml.adapter` pin absent
// from the global store fails ahead of the deployment with the typed
// `adapter-not-installed` (exit 2), naming the identity and the
// literal sync command — the guest never hydrates.
#[test]
fn pinned_store_miss_is_not_installed() {
    let stub = stub_cursor_agent();
    let project = tempdir().expect("project dir");
    let specify_dir = project.path().join(".specify");
    fs::create_dir_all(&specify_dir).expect("mkdir .specify");
    fs::write(
        specify_dir.join("project.yaml"),
        "name: demo\nadapter: demo-missing@9.9.9\nspecify: 0.1.0\n",
    )
    .expect("write project.yaml");

    let output = specify_cmd()
        .current_dir(project.path())
        .env("PATH", stub_path(stub.path()))
        .args(["plan", "execute", "--format", "json"])
        .output()
        .expect("running specify");

    assert_eq!(output.status.code(), Some(2), "a store miss is a validation failure");
    let envelope = parse_json(&output.stderr);
    assert_eq!(envelope["error"], "adapter-not-installed");
    let message = envelope["message"].as_str().expect("message");
    assert!(message.contains("demo-missing@9.9.9"), "error names the identity: {message}");
    assert!(
        message.contains("specify adapters sync"),
        "error names the literal sync command: {message}"
    );
}

// The background HTTP trigger in command mode: with a discovered
// adapter guest exporting `wasi:http` the trigger binds an ephemeral
// port and logs its listening line while the CLI guest runs. The echo
// component is staged unbound in the project component cache, so this
// also covers the sniff-axis discovery leg.
#[test]
fn guest_verb_http_trigger_background() {
    ensure_host_binary();
    let stub = stub_cursor_agent();
    let project = tempdir().expect("project dir");
    let cache = common::expected_cache_dir(project.path()).join("components");
    fs::create_dir_all(&cache).expect("component cache dir");
    fs::copy(echo_guest_wasm(), cache.join("echo.wasm")).expect("stage echo component");

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
// on PATH each verb fails at backend connect *inside* the spawned host
// (its stderr names the missing agent; exit 1 passes through) —
// reaching the composed-deployment leg at all distinguishes it from
// the native `argument` refusal.
#[test]
fn guest_owned_verbs_route_to_guest_leg() {
    ensure_host_binary();
    let empty = tempdir().expect("empty PATH dir");
    let project = tempdir().expect("project dir");
    for argv in [
        vec!["plan", "execute"],
        vec!["plan", "author", "demo-change", "--intent", "demo"],
        vec!["slice", "refine", "demo-slice"],
        vec!["slice", "build", "demo-slice"],
        vec!["slice", "merge", "run", "demo-slice"],
        vec!["source", "survey", "demo-source"],
        vec!["source", "extract", "demo-source", "demo-lead", "--slice", "demo-slice"],
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("cursor-agent"),
            "verb {argv:?} must route to the composed-deployment leg and fail at the host's \
             backend connect:\n{stderr}"
        );
    }
}

// `--plan-dir` is native-only on guest-routed verbs: the guest anchors
// plan artifacts at the `"."` preopen, so a plan root that is not the
// working directory is refused loudly on the standard argument surface
// instead of being silently ignored (Step 4 parity ledger).
#[test]
fn plan_dir_refused_on_guest_leg() {
    ensure_host_binary();
    let project = tempdir().expect("project dir");
    let elsewhere = tempdir().expect("other plan root");

    let output = specify_cmd()
        .current_dir(project.path())
        .arg("--plan-dir")
        .arg(elsewhere.path())
        .args(["plan", "execute", "--format", "json"])
        .output()
        .expect("running specify");

    assert_eq!(output.status.code(), Some(2), "a foreign plan root must refuse with exit 2");
    let envelope = parse_json(&output.stderr);
    assert_eq!(envelope["error"], "argument");

    // A value resolving to the working directory itself is a no-op and
    // passes through to the composed-deployment leg (which then fails
    // at the host's backend connect on the empty PATH — proving triage
    // proceeded).
    let empty = tempdir().expect("empty PATH dir");
    let output = specify_cmd()
        .current_dir(project.path())
        .env("PATH", empty.path())
        .arg("--plan-dir")
        .arg(project.path())
        .args(["plan", "execute", "--format", "json"])
        .output()
        .expect("running specify");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cursor-agent"), "the drive must reach the host spawn:\n{stderr}");
}

// A cache entry that is not a component (garbage bytes under a `.wasm`
// name) is skipped by the sniff-axis discovery leg — unbound cache
// contents never abort the deployment, the verb still reaches the
// guest and fails there.
#[test]
fn non_component_cache_entry_is_skipped() {
    ensure_host_binary();
    let stub = stub_cursor_agent();
    let project = tempdir().expect("project dir");
    let cache = common::expected_cache_dir(project.path()).join("components");
    fs::create_dir_all(&cache).expect("component cache dir");
    fs::write(cache.join("hollow.wasm"), b"not a component").expect("stage garbage entry");

    let output = specify_cmd()
        .current_dir(project.path())
        .env("PATH", stub_path(stub.path()))
        .env_remove("RUST_LOG")
        .args(["plan", "execute", "--format", "json"])
        .output()
        .expect("running specify");

    assert_eq!(output.status.code(), Some(1));
    let envelope = parse_json(&output.stderr);
    assert_eq!(
        envelope["error"], "not-initialized",
        "the garbage cache entry must be skipped and the verb fail inside the guest"
    );
}

// Bound adapters resolve with the resolvers' precedence: a `plan.yaml`
// source binding pinned to `(name, version)` reaches the RFC-48 store —
// which the old name-only directory scan could never probe — and a
// missing install surfaces the typed `adapter-not-installed` (RFC-65
// AC7: name the identity and the sync command; never fetch, never a
// silently thinner deployment).
#[test]
fn bound_adapter_resolves_from_store() {
    ensure_host_binary();
    let stub = stub_cursor_agent();
    let project = tempdir().expect("project dir");
    fs::write(
        project.path().join("plan.yaml"),
        "name: store-parity\nsources:\n  echo:\n    adapter: echo\n    version: 1.0.0\n    path: ./x\nslices: []\n",
    )
    .expect("write plan.yaml");

    let store = tempdir().expect("adapter store root");
    let entry = store.path().join("echo@1.0.0.wasm");
    fs::copy(echo_guest_wasm(), &entry).expect("stage store component");
    let digest = common::sha256_hex(&entry);
    fs::write(store.path().join("echo@1.0.0.meta"), format!("tree_digest: sha256:{digest}\n"))
        .expect("write store meta sidecar");

    let output = specify_cmd()
        .current_dir(project.path())
        .env("PATH", stub_path(stub.path()))
        .env("SPECIFY_ADAPTER_STORE", store.path())
        .env_remove("RUST_LOG")
        .args(["plan", "execute", "--format", "json"])
        .output()
        .expect("running specify");

    assert_eq!(output.status.code(), Some(1), "the verb must reach the guest: {output:?}");
    let envelope = parse_json(&output.stderr);
    assert_eq!(
        envelope["error"], "not-initialized",
        "the store-resolved guest must deploy and the verb fail *inside* the guest"
    );

    // Without the store entry the binding cannot resolve anywhere, and
    // the typed store-miss diagnostic surfaces host-side with the
    // literal sync remedy.
    let empty_store = tempdir().expect("empty store root");
    let output = specify_cmd()
        .current_dir(project.path())
        .env("PATH", stub_path(stub.path()))
        .env("SPECIFY_ADAPTER_STORE", empty_store.path())
        .args(["plan", "execute", "--format", "json"])
        .output()
        .expect("running specify");

    assert_eq!(output.status.code(), Some(2));
    let envelope = parse_json(&output.stderr);
    assert_eq!(envelope["error"], "adapter-not-installed");
    let message = envelope["message"].as_str().expect("message");
    assert!(message.contains("echo@1.0.0"), "error names the identity: {message}");
    assert!(
        message.contains("specify adapters sync"),
        "error names the literal sync command: {message}"
    );
}

// RFC-65 AC8 on the guest leg: drive-time manifest regeneration
// verifies every pinned entry against the committed
// `.specify/adapters.lock`. A warm-but-divergent store (populated by
// another project or machine) aborts with the typed
// `adapter-digest-mismatch` naming the identity and both digests —
// before any manifest is written or guest driven; once the lock pins
// the actual digest, the same drive proceeds into the guest.
#[test]
fn committed_lock_gates_drive() {
    ensure_host_binary();
    let stub = stub_cursor_agent();
    let project = tempdir().expect("project dir");
    fs::write(
        project.path().join("plan.yaml"),
        "name: lock-gate\nsources:\n  echo:\n    adapter: echo\n    version: 1.0.0\n    path: ./x\nslices: []\n",
    )
    .expect("write plan.yaml");

    let store = tempdir().expect("adapter store root");
    let entry = store.path().join("echo@1.0.0.wasm");
    fs::copy(echo_guest_wasm(), &entry).expect("stage store component");
    let digest = common::sha256_hex(&entry);
    fs::write(store.path().join("echo@1.0.0.meta"), format!("tree_digest: sha256:{digest}\n"))
        .expect("write store meta sidecar");

    let specify_dir = project.path().join(".specify");
    fs::create_dir_all(&specify_dir).expect("mkdir .specify");
    let divergent = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    fs::write(
        specify_dir.join("adapters.lock"),
        format!("version: 1\nadapters:\n  echo@1.0.0: {divergent}\n"),
    )
    .expect("write divergent lock");

    let drive = || {
        specify_cmd()
            .current_dir(project.path())
            .env("PATH", stub_path(stub.path()))
            .env("SPECIFY_ADAPTER_STORE", store.path())
            .env_remove("RUST_LOG")
            .args(["plan", "execute", "--format", "json"])
            .output()
            .expect("running specify")
    };

    let output = drive();
    // `adapter-digest-mismatch` is Diag-routed (generic failure, exit
    // 1) — the same posture as the resolver's D4 verify-on-read.
    assert_eq!(output.status.code(), Some(1), "lock drift must abort the drive: {output:?}");
    let envelope = parse_json(&output.stderr);
    assert_eq!(envelope["error"], "adapter-digest-mismatch");
    let message = envelope["message"].as_str().expect("message");
    assert!(message.contains("echo@1.0.0"), "error names the identity: {message}");
    assert!(message.contains(divergent), "error names the locked digest: {message}");
    assert!(
        message.contains(&format!("sha256:{digest}")),
        "error names the actual digest: {message}"
    );
    let deployment = common::expected_cache_dir(project.path()).join("deployment");
    assert!(
        !deployment.join("omnia.toml").is_file(),
        "no manifest may be written when the lock gate refuses"
    );

    // A lock-clean warm store drives fine: the verb reaches the guest
    // and fails there (`not-initialized` against the bare mount).
    fs::write(
        specify_dir.join("adapters.lock"),
        format!("version: 1\nadapters:\n  echo@1.0.0: sha256:{digest}\n"),
    )
    .expect("write clean lock");
    let output = drive();
    assert_eq!(output.status.code(), Some(1), "the verb must reach the guest: {output:?}");
    let envelope = parse_json(&output.stderr);
    assert_eq!(
        envelope["error"], "not-initialized",
        "a lock-clean warm store must drive the deployment"
    );
}

// Manifest hygiene: host paths are emitted as escaped TOML strings, so
// a project path containing `"` and `\` still assembles a parseable
// deployment manifest and the verb fails *inside* the guest.
#[test]
fn manifest_escapes_hostile_paths() {
    ensure_host_binary();
    let stub = stub_cursor_agent();
    let tmp = tempdir().expect("parent dir");
    let project = tmp.path().join("we\"ird\\dir");
    fs::create_dir_all(&project).expect("hostile project dir");

    let output = specify_cmd()
        .current_dir(&project)
        .env("PATH", stub_path(stub.path()))
        .env_remove("RUST_LOG")
        .args(["plan", "execute", "--format", "json"])
        .output()
        .expect("running specify");

    assert_eq!(output.status.code(), Some(1));
    let envelope = parse_json(&output.stderr);
    assert_eq!(
        envelope["error"], "not-initialized",
        "the manifest must parse (a raw interpolation would fail host-side): {envelope}"
    );
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
