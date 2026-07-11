//! CLI-mode project anchoring: the shim-global `--project-dir` flag
//! (both spellings) anchors every workflow verb at the named project
//! root — the CLI counterpart of serve mode's flag — so the binary can
//! run from any working directory.

mod common;

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// Spawn the built `specify-dev` from `cwd` with `args`.
fn run_from(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_specify-dev"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("spawn specify-dev")
}

fn assert_created(project: &Path, elsewhere: &Path, output: &Output) {
    assert!(
        output.status.success(),
        "plan create failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(project.join("plan.yaml").is_file(), "plan.yaml lands under --project-dir");
    assert!(!elsewhere.join("plan.yaml").exists(), "nothing written at the working directory");
}

mod project_dir_forms {
    use super::*;

    #[test]
    fn space() {
        let project = common::Project::new();
        let elsewhere = TempDir::new().expect("tempdir");

        let root = project.root().to_string_lossy().into_owned();
        let output = run_from(
            elsewhere.path(),
            &["--project-dir", &root, "plan", "create", "demo-change", "--intent", "Say hello"],
        );
        assert_created(project.root(), elsewhere.path(), &output);
    }

    #[test]
    fn equals() {
        let project = common::Project::new();
        let elsewhere = TempDir::new().expect("tempdir");

        let flag = format!("--project-dir={}", project.root().display());
        let output = run_from(
            elsewhere.path(),
            &[&flag, "plan", "create", "demo-change", "--intent", "Say hello"],
        );
        assert_created(project.root(), elsewhere.path(), &output);
    }
}

#[test]
fn scaffold_component_free() {
    // A bare adapter name resolves through the linked-crate catalog:
    // init succeeds with no `.wasm` artifact anywhere near the project.
    let project = TempDir::new().expect("tempdir");
    let elsewhere = TempDir::new().expect("tempdir");

    let root = project.path().canonicalize().expect("canonical tempdir");
    let flag = format!("--project-dir={}", root.display());
    let output =
        run_from(elsewhere.path(), &[&flag, "init", "omnia", "--name", "demo", "--scaffold-only"]);
    assert!(
        output.status.success(),
        "component-free init failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let config = std::fs::read_to_string(root.join(".specify/project.yaml")).expect("project.yaml");
    assert!(config.contains("adapter: omnia"), "the bare identity is recorded:\n{config}");
    assert!(!root.join("target").exists(), "no development artifact tree is demanded or created");
}

mod project_dir_errors {
    use super::*;

    #[test]
    fn missing_path_refused() {
        let elsewhere = TempDir::new().expect("tempdir");
        let output = run_from(elsewhere.path(), &["--project-dir"]);
        assert!(!output.status.success(), "a bare --project-dir must refuse");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("requires a path"), "stderr: {stderr}");
    }

    #[test]
    fn nonexistent_root_refused() {
        let elsewhere = TempDir::new().expect("tempdir");
        let missing = elsewhere.path().join("no-such-project");
        let flag = format!("--project-dir={}", missing.display());
        let output = run_from(elsewhere.path(), &[&flag, "plan", "status"]);
        assert!(!output.status.success(), "a missing project root must refuse");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--project-dir"), "stderr: {stderr}");
    }
}
