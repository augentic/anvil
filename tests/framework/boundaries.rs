//! Dependency-direction predicates that keep concrete adapter crates out of
//! the workflow engine while allowing adapter vocabulary in wire examples.
//!
//! The check parses every engine Cargo manifest and inspects each
//! dependency table: an entry is a violation when its effective package
//! name (the `package` rename target when present) is a concrete adapter
//! crate, or when its `path`/`git` source points into the
//! `specify-adapters` repository. Rust sources need no scan — a crate
//! that no manifest declares cannot be imported.

use std::fs;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

use crate::support::{Finding, rel, walk_files};

/// An engine manifest declares a concrete adapter implementation.
pub const CHECK_ADAPTER_DEPENDENCY: &str = "architecture.adapter-dependency";

/// Engine manifest scopes: the workspace root plus every crate and
/// harness manifest.
const SCOPES: &[&str] = &["Cargo.toml", "crates", "harness"];

/// Crates owned by the `specify-adapters` repository. `omnia` is absent
/// on purpose: the name collides with the Omnia runtime crate, so the
/// omnia target adapter is caught by the repository-source rule instead.
/// `prose` is absent too: the engine owns its own `crates/prose`
/// build-time codegen crate of the same name.
const ADAPTER_CRATES: &[&str] = &[
    "adapter",
    "captures",
    "contracts",
    "documentation",
    "intent",
    "screenshots",
    "typescript",
    "vectis",
];

/// Any `path`/`git` dependency source containing this segment reaches
/// into the adapters repository.
const ADAPTER_REPOSITORY: &str = "specify-adapters";

/// Find concrete adapter dependencies in the engine's Cargo manifests.
pub fn run(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for manifest in manifests(root) {
        let Ok(body) = fs::read_to_string(&manifest) else {
            continue;
        };
        // Cargo gates parseability itself; an unreadable manifest cannot
        // hide a dependency from the build either.
        let Ok(document) = body.parse::<Table>() else {
            continue;
        };
        for (table, name, detail) in violations(&document) {
            findings.push(Finding::new(
                CHECK_ADAPTER_DEPENDENCY,
                format!("{}: [{table}] {name}: {detail}", rel(root, &manifest)),
            ));
        }
    }
    findings
}

/// Every Cargo manifest under the engine scopes.
fn manifests(root: &Path) -> Vec<PathBuf> {
    let mut manifests = Vec::new();
    for scope in SCOPES {
        let path = root.join(scope);
        if path.is_file() {
            manifests.push(path);
            continue;
        }
        let mut files = Vec::new();
        walk_files(&path, &mut files);
        manifests.extend(
            files
                .into_iter()
                .filter(|file| file.file_name().is_some_and(|name| name == "Cargo.toml")),
        );
    }
    manifests
}

/// Each offending `(table, dependency name, detail)` in one manifest.
fn violations(document: &Table) -> Vec<(String, String, String)> {
    let mut violations = Vec::new();
    for (table, entries) in dependency_tables(document) {
        for (name, spec) in entries {
            if let Some(detail) = offence(name, spec) {
                violations.push((table.clone(), name.clone(), detail));
            }
        }
    }
    violations
}

/// Every dependency table in one manifest: the three package tables,
/// `workspace.dependencies`, and each target-specific variant.
fn dependency_tables(root: &Table) -> Vec<(String, &Table)> {
    const KINDS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    let mut tables = Vec::new();
    for kind in KINDS {
        if let Some(entries) = root.get(kind).and_then(Value::as_table) {
            tables.push((kind.to_string(), entries));
        }
    }
    if let Some(entries) = root
        .get("workspace")
        .and_then(Value::as_table)
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(Value::as_table)
    {
        tables.push(("workspace.dependencies".to_string(), entries));
    }
    if let Some(targets) = root.get("target").and_then(Value::as_table) {
        for (cfg, entry) in targets {
            let Some(entry) = entry.as_table() else {
                continue;
            };
            for kind in KINDS {
                if let Some(entries) = entry.get(kind).and_then(Value::as_table) {
                    tables.push((format!("target.{cfg}.{kind}"), entries));
                }
            }
        }
    }
    tables
}

/// Why one dependency entry violates the boundary, if it does.
fn offence(name: &str, spec: &Value) -> Option<String> {
    let package = spec
        .as_table()
        .and_then(|table| table.get("package"))
        .and_then(Value::as_str)
        .unwrap_or(name);
    let effective = package.replace('_', "-");
    if ADAPTER_CRATES.contains(&effective.as_str()) {
        return Some(format!("depends on the concrete adapter crate `{effective}`"));
    }
    let table = spec.as_table()?;
    for key in ["path", "git"] {
        if let Some(source) = table.get(key).and_then(Value::as_str)
            && source.contains(ADAPTER_REPOSITORY)
        {
            return Some(format!("`{key}` points into {ADAPTER_REPOSITORY}: {source}"));
        }
    }
    None
}
