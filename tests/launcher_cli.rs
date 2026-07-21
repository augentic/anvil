//! CLI-boundary exit-code contract for the shipped binary's launcher:
//! host-side help/version displays, grammar rejection, fail-closed
//! store verification, and the `SPECIFY_HOME` environment-capture
//! contract — all before any runtime starts, and all without touching
//! the network (each fixture pre-seeds the pinned store so hydration
//! never fetches).

#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// One sandboxed binary invocation: a project directory plus one
/// relocated `SPECIFY_HOME`, all inside one tempdir.
struct Sandbox {
    tmp: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let sandbox = Self {
            tmp: tempfile::TempDir::new().expect("tempdir"),
        };
        for dir in ["project", "home/store", "home/cache"] {
            std::fs::create_dir_all(sandbox.tmp.path().join(dir)).expect("mkdir sandbox dir");
        }
        sandbox
    }

    fn home(&self) -> PathBuf {
        self.tmp.path().join("home")
    }

    fn store(&self) -> PathBuf {
        self.home().join("store")
    }

    fn seed_engine_at(store: &Path, sidecar: Option<&str>) {
        std::fs::create_dir_all(store).expect("mkdir store");
        std::fs::write(store.join(format!("engine@{VERSION}.wasm")), b"engine bytes")
            .expect("write engine entry");
        if let Some(body) = sidecar {
            std::fs::write(store.join(format!("engine@{VERSION}.meta")), body)
                .expect("write engine sidecar");
        }
    }

    fn seed_engine(&self, sidecar: Option<&str>) {
        Self::seed_engine_at(&self.store(), sidecar);
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_specify"));
        command.args(args).current_dir(self.tmp.path().join("project"));
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).env("SPECIFY_HOME", self.home()).output().expect("spawn specify")
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn version_renders_host_side() {
    let output = Sandbox::new().run(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), format!("specify {VERSION}\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn help_renders_host_side() {
    // The shared grammar answers help without assembling a deployment:
    // exit 0 with usage on stdout, and nothing hydrated into the
    // (empty) store.
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: specify"));
    assert!(!sandbox.store().join(format!("engine@{VERSION}.wasm")).exists());
}

#[test]
fn grammar_rejection_exits_2() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["frobnicate"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unrecognized subcommand"), "{}", stderr(&output));
    // Nothing started, nothing hydrated.
    assert!(!sandbox.store().join(format!("engine@{VERSION}.wasm")).exists());
}

#[test]
fn verify_sidecar_missing_exits_1() {
    let sandbox = Sandbox::new();
    sandbox.seed_engine(None);
    let output = sandbox.run(&["registry", "validate"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("adapter-sidecar-missing"), "{}", stderr(&output));
}

#[test]
fn verify_digest_mismatch_exits_1() {
    let sandbox = Sandbox::new();
    let stale = format!("tree_digest: sha256:{}\n", "0".repeat(64));
    sandbox.seed_engine(Some(&stale));
    let output = sandbox.run(&["registry", "validate"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("adapter-digest-mismatch"), "{}", stderr(&output));
}

#[test]
fn verify_failure_renders_format_json() {
    let sandbox = Sandbox::new();
    sandbox.seed_engine(None);
    let output = sandbox.run(&["--format", "json", "registry", "validate"]);
    assert_eq!(output.status.code(), Some(1));
    let body: serde_json::Value =
        serde_json::from_str(&stderr(&output)).expect("JSON failure envelope");
    assert_eq!(body["error"], "adapter-sidecar-missing");
    assert_eq!(body["exit-code"], 1);
}

// The environment-capture contract: `SPECIFY_HOME` relocates store and
// cache together; the effective default is `$HOME/.specify`; empty or
// relative overrides are ignored; without `$HOME` the temp directory
// anchors the fallback. Each assertion reads the store entry path the
// verification diagnostic names.
mod environment_capture {
    use super::*;

    #[test]
    fn specify_home_relocates_the_store() {
        let sandbox = Sandbox::new();
        sandbox.seed_engine(None);
        let output = sandbox.run(&["registry", "validate"]);
        let expected = sandbox.store().join(format!("engine@{VERSION}.wasm"));
        assert!(
            stderr(&output).contains(&expected.display().to_string()),
            "the diagnostic names the relocated store entry: {}",
            stderr(&output)
        );
    }

    #[test]
    fn default_home_derives_from_user_home() {
        let sandbox = Sandbox::new();
        let user_home = sandbox.tmp.path().join("user-home");
        let store = user_home.join(".specify/store");
        Sandbox::seed_engine_at(&store, None);

        let output = sandbox
            .command(&["registry", "validate"])
            .env_remove("SPECIFY_HOME")
            .env("HOME", &user_home)
            .output()
            .expect("spawn specify");
        assert_eq!(output.status.code(), Some(1));
        assert!(stderr(&output).contains("adapter-sidecar-missing"), "{}", stderr(&output));
        let expected = store.join(format!("engine@{VERSION}.wasm"));
        assert!(
            stderr(&output).contains(&expected.display().to_string()),
            "the default store derives from $HOME/.specify: {}",
            stderr(&output)
        );
    }

    #[test]
    fn relative_home_ignored() {
        let sandbox = Sandbox::new();
        let user_home = sandbox.tmp.path().join("user-home");
        let store = user_home.join(".specify/store");
        Sandbox::seed_engine_at(&store, None);

        let output = sandbox
            .command(&["registry", "validate"])
            .env("SPECIFY_HOME", "relative/home")
            .env("HOME", &user_home)
            .output()
            .expect("spawn specify");
        let expected = store.join(format!("engine@{VERSION}.wasm"));
        assert!(
            stderr(&output).contains(&expected.display().to_string()),
            "a relative override falls through to the default home: {}",
            stderr(&output)
        );
    }

    #[test]
    fn temp_fallback_without_home() {
        let sandbox = Sandbox::new();
        let temp = sandbox.tmp.path().join("temp");
        let store = temp.join("specify/store");
        Sandbox::seed_engine_at(&store, None);

        let output = sandbox
            .command(&["registry", "validate"])
            .env_remove("SPECIFY_HOME")
            .env_remove("HOME")
            .env("TMPDIR", &temp)
            .output()
            .expect("spawn specify");
        let expected = store.join(format!("engine@{VERSION}.wasm"));
        assert!(
            stderr(&output).contains(&expected.display().to_string()),
            "without $HOME the temp directory anchors the fallback: {}",
            stderr(&output)
        );
    }
}
