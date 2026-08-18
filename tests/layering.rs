//! Layering gate over `cargo metadata` (CONSTITUTION.md invariant 3,
//! remediation Phase 2): the workspace crate DAG must match the
//! committed edge list — the Phase 3 spine cut revises it deliberately.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

/// Every allowed `dependent -> dependency` edge between workspace
/// crates, leaf → root per the AGENTS.md map. Normal and build
/// dependencies only: dev-dependencies may legally cycle (test
/// harnesses reach back down) and carry no layering authority.
const ALLOWED: &[(&str, &str)] = &[
    ("emery", "emery-guest"),
    ("emery", "emery-launcher"),
    ("emery-artifacts", "emery-diagnostics"),
    ("emery-artifacts", "emery-error"),
    ("emery-guest", "emery-error"),
    ("emery-guest", "emery-project"),
    ("emery-guest", "emery-transport"),
    ("emery-launcher", "emery-artifacts"),
    ("emery-launcher", "emery-diagnostics"),
    ("emery-launcher", "emery-error"),
    ("emery-launcher", "emery-project"),
    ("emery-launcher", "emery-transport"),
    ("emery-mock", "emery-adapter"),
    ("emery-mock", "emery-native"),
    ("emery-mock", "emery-project"),
    ("emery-mock-component", "emery-adapter"),
    ("emery-mock-component", "emery-mock"),
    ("emery-native", "emery-adapter"),
    ("emery-native", "emery-error"),
    ("emery-native", "emery-project"),
    ("emery-native", "emery-transport"),
    ("emery-project", "emery-artifacts"),
    ("emery-project", "emery-diagnostics"),
    ("emery-project", "emery-error"),
    ("emery-transport", "emery-artifacts"),
    ("emery-transport", "emery-diagnostics"),
    ("emery-transport", "emery-error"),
    ("emery-transport", "emery-project"),
];

#[test]
fn dag() {
    let actual = edges();
    let allowed: BTreeSet<(String, String)> =
        ALLOWED.iter().map(|(from, to)| ((*from).to_owned(), (*to).to_owned())).collect();

    let mut violations = String::new();
    for (from, to) in actual.difference(&allowed) {
        writeln!(violations, "  {from} -> {to} is not an allowed layering edge")
            .expect("infallible write to String");
    }
    for (from, to) in allowed.difference(&actual) {
        writeln!(violations, "  allowed edge {from} -> {to} no longer exists; remove it")
            .expect("infallible write to String");
    }
    assert!(
        violations.is_empty(),
        "crate-DAG violations (CONSTITUTION.md invariant 3). Changing the layering is a \
         deliberate act: update `tests/layering.rs` and cite the decision:\n{violations}"
    );
}

/// Workspace-internal normal/build dependency edges from
/// `cargo metadata`, resolved against the committed lockfile.
fn edges() -> BTreeSet<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
        .current_dir(root)
        .output()
        .expect("run cargo metadata");
    assert!(output.status.success(), "cargo metadata failed");

    let meta: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse cargo metadata JSON");
    let packages = meta["packages"].as_array().expect("packages array");
    let names: BTreeSet<&str> =
        packages.iter().map(|package| package["name"].as_str().expect("package name")).collect();

    let mut found = BTreeSet::new();
    for package in packages {
        let from = package["name"].as_str().expect("package name");
        for dep in package["dependencies"].as_array().expect("dependencies array") {
            let to = dep["name"].as_str().expect("dependency name");
            let kind = dep["kind"].as_str().unwrap_or("normal");
            if names.contains(to) && kind != "dev" {
                found.insert((from.to_owned(), to.to_owned()));
            }
        }
    }
    found
}
