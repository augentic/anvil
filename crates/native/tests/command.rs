//! Asynchronous command execution over caller-supplied values.

use std::fs;
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

fn project() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical tempdir");
    for sub in [".emery/change/slices", ".emery/specs"] {
        fs::create_dir_all(root.join(sub)).expect("mkdir");
    }
    fs::write(root.join(".emery/project.yaml"), "name: demo\nadapter: mock\nrules: {}\n")
        .expect("write project.yaml");
    (tmp, root)
}

fn argv(args: &[&str]) -> Vec<String> {
    let mut full = vec!["emery".to_string()];
    full.extend(args.iter().map(ToString::to_string));
    full
}

#[tokio::test]
async fn executes_a_verb() {
    let (_tmp, root) = project();
    let paths = paths(&root);
    fs::create_dir_all(root.join(".emery/change/events")).expect("events dir");
    fs::write(
        root.join(".emery/change/events/local.jsonl"),
        "{\"timestamp\":\"2026-01-01T00:00:00Z\",\"writer\":\"local\",\"sequence\":1,\
         \"event\":\"slice.build.started\",\"payload\":{\"slice-name\":\"demo\"}}\n",
    )
    .expect("stage journal");
    let response = native::command::execute(
        paths,
        model(),
        catalog(),
        argv(&["journal", "show", "--filter", "slice.build"]),
    )
    .await
    .expect("the router assembles");
    assert_eq!(response.exit, 0, "{}", String::from_utf8_lossy(&response.stderr));
    let stdout = String::from_utf8_lossy(&response.stdout);
    assert!(stdout.contains("slice.build.started"), "{stdout}");
}

#[tokio::test]
async fn failure_typed_response() {
    let (_tmp, root) = project();
    let paths = paths(&root);
    // An unknown verb is a buffered response, not a library error.
    let response = native::command::execute(paths, model(), catalog(), argv(&["no-such-verb"]))
        .await
        .expect("the router assembles");
    assert_ne!(response.exit, 0);
    assert!(!response.stderr.is_empty());
}

#[tokio::test]
async fn system_dir_reanchors() {
    // `system * --dir` roots at the definition home (launcher
    // parity); the caller's product paths carry no `scope.yaml`.
    let (_tmp, root) = project();
    let home = root.join("definition-home");
    fs::create_dir_all(&home).expect("definition home");
    fs::write(home.join("scope.yaml"), "version: 1\nid: acme\ndecision: consolidate\n")
        .expect("scope.yaml");
    fs::write(
        home.join("coverage.yaml"),
        "version: 1\ncandidates:\n  - key: legacy-erp\n    location: https://erp.example.com\n    \
         disposition: inaccessible\n    reason: vendor system\n",
    )
    .expect("coverage.yaml");

    // Without `--dir`, the product root has no scope and fails typed.
    let response =
        native::command::execute(paths(&root), model(), catalog(), argv(&["system", "status"]))
            .await
            .expect("the router assembles");
    let stderr = String::from_utf8_lossy(&response.stderr).to_string();
    assert_ne!(response.exit, 0);
    assert!(stderr.contains("system-scope-missing"), "{stderr}");

    // With `--dir`, the same paths reanchor at the definition home.
    let dir = home.to_string_lossy().to_string();
    let response = native::command::execute(
        paths(&root),
        model(),
        catalog(),
        argv(&["--format", "json", "system", "status", "--dir", &dir]),
    )
    .await
    .expect("the router assembles");
    let stdout = String::from_utf8_lossy(&response.stdout).to_string();
    let stderr = String::from_utf8_lossy(&response.stderr).to_string();
    assert_eq!(response.exit, 0, "{stderr}");
    assert!(stdout.contains("\"id\": \"acme\""), "the status projects the home's scope: {stdout}");
}
