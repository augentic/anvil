//! Asynchronous command execution over caller-supplied values.

use std::fs;
use std::path::PathBuf;

use linked::{Catalog, DynModel, ExecutionPaths};
use omnia_testkit::model::Scripted;
use tempfile::TempDir;

fn catalog() -> Catalog {
    Catalog::builder().build().expect("empty catalog")
}

fn model() -> DynModel {
    DynModel::new(Scripted::answers::<&str>([]))
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

fn argv(args: &[&str]) -> Vec<String> {
    let mut full = vec!["specify".to_string()];
    full.extend(args.iter().map(ToString::to_string));
    full
}

#[tokio::test]
async fn executes_a_verb() {
    let (_tmp, root) = project();
    let paths = ExecutionPaths::operator(&root);
    let response = linked::command::execute(
        paths,
        model(),
        catalog(),
        argv(&["journal", "emit", "slice.build.started", "--payload", r#"{"slice-name":"demo"}"#]),
    )
    .await
    .expect("the router assembles");
    assert_eq!(response.exit, 0, "{}", String::from_utf8_lossy(&response.stderr));
    assert!(root.join(".specify/journal.jsonl").is_file());
}

#[tokio::test]
async fn failure_is_a_typed_response() {
    let (_tmp, root) = project();
    let paths = ExecutionPaths::operator(&root);
    // An unknown verb is a buffered response, not a library error.
    let response = linked::command::execute(paths, model(), catalog(), argv(&["no-such-verb"]))
        .await
        .expect("the router assembles");
    assert_ne!(response.exit, 0);
    assert!(!response.stderr.is_empty());
}
