//! Native command anchoring owned by the shared dev shim.

use std::fs;
use std::path::{Path, PathBuf};

use harness::catalog::{Binding, Catalog};
use omnia_guest::Model;
use tempfile::TempDir;

struct Adapters;

impl Binding for Adapters {
    fn catalog<M: Model>() -> Catalog<M> {
        Catalog::builder().build()
    }
}

fn project() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical tempdir");
    for sub in [".specify/slices", ".specify/specs"] {
        fs::create_dir_all(root.join(sub)).expect("mkdir");
    }
    fs::write(root.join(".specify/project.yaml"), "name: demo\nadapter: fixture\nrules: {}\n")
        .expect("write project.yaml");
    (tmp, root)
}

fn emit_args(root: &Path, equals: bool) -> Vec<String> {
    let mut args = vec!["engine".to_string()];
    if equals {
        args.push(format!("--project-dir={}", root.display()));
    } else {
        args.extend(["--project-dir".to_string(), root.display().to_string()]);
    }
    args.extend([
        "journal".to_string(),
        "emit".to_string(),
        "slice.build.started".to_string(),
        "--payload".to_string(),
        r#"{"slice-name":"demo"}"#.to_string(),
    ]);
    args
}

#[tokio::test]
async fn forms() {
    for equals in [false, true] {
        let (_tmp, root) = project();
        let exit = harness::command::run::<Adapters>(emit_args(&root, equals)).await;
        assert_eq!(exit, 0);
        assert!(root.join(".specify/journal.jsonl").is_file());
    }
}

#[tokio::test]
async fn refusals() {
    let exit =
        harness::command::run::<Adapters>(vec!["engine".to_string(), "--project-dir".to_string()])
            .await;
    assert_eq!(exit, 1);

    let tmp = TempDir::new().expect("tempdir");
    let missing = tmp.path().join("missing");
    let exit = harness::command::run::<Adapters>(vec![
        "engine".to_string(),
        format!("--project-dir={}", missing.display()),
        "plan".to_string(),
        "status".to_string(),
    ])
    .await;
    assert_eq!(exit, 1);
}
