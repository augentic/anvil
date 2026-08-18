//! Asynchronous command execution over caller-supplied values.

use std::path::PathBuf;

use native::{CachePlacement, Catalog, DynModel, ExecutionPaths, Locations};
use omnia_testkit::model::Scripted;
use tempfile::TempDir;

// Explicit tempdir-rooted layout: hermetic carried locations, no
// environment reads.
fn paths(root: &std::path::Path) -> ExecutionPaths {
    let locations =
        Locations::explicit(root.join("store"), CachePlacement::Parent(root.join("project-cache")));
    ExecutionPaths::new(root, locations)
}

fn catalog() -> Catalog {
    Catalog::builder().build().expect("empty catalog")
}

fn model() -> DynModel {
    DynModel::new(Scripted::answers::<&str>([]))
}

fn root() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical tempdir");
    (tmp, root)
}

fn argv(args: &[&str]) -> Vec<String> {
    let mut full = vec!["emery".to_string()];
    full.extend(args.iter().map(ToString::to_string));
    full
}

#[tokio::test]
async fn executes_a_verb() {
    let (_tmp, root) = root();
    let response =
        native::command::execute(paths(&root), model(), catalog(), argv(&["completions", "zsh"]))
            .await
            .expect("the router assembles");
    assert_eq!(response.exit, 0, "{}", String::from_utf8_lossy(&response.stderr));
    let stdout = String::from_utf8_lossy(&response.stdout);
    assert!(stdout.contains("_emery"), "{stdout}");
}

#[tokio::test]
async fn failure_typed_response() {
    let (_tmp, root) = root();
    // An unknown verb is a buffered response, not a library error.
    let response =
        native::command::execute(paths(&root), model(), catalog(), argv(&["no-such-verb"]))
            .await
            .expect("the router assembles");
    assert_ne!(response.exit, 0);
    assert!(!response.stderr.is_empty());
}

#[tokio::test]
async fn specify_stub() {
    let (_tmp, root) = root();
    // The reserved verb parses and fails typed (ADR-0008 §3).
    let response = native::command::execute(paths(&root), model(), catalog(), argv(&["specify"]))
        .await
        .expect("the router assembles");
    assert_eq!(response.exit, 1);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("specify-not-implemented"), "{stderr}");
}
