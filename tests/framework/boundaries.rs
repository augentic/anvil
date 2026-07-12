//! Dependency-direction predicates that keep concrete adapter crates out of
//! the workflow engine while allowing adapter vocabulary in wire examples.

use std::fs;
use std::path::Path;

use crate::support::{Finding, rel, walk_files};

/// Engine code or manifests import a concrete adapter implementation.
pub const CHECK_ADAPTER_DEPENDENCY: &str = "architecture.adapter-dependency";

const SCOPES: &[&str] = &["Cargo.toml", "crates", "harness", "src"];
const CRATES: &[&str] = &[
    "adapter",
    "captures",
    "contracts",
    "documentation",
    "intent",
    "omnia_target",
    "screenshots",
    "typescript",
    "vectis",
];

/// Find concrete adapter dependencies in engine manifests and Rust imports.
pub fn run(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for scope in SCOPES {
        let path = root.join(scope);
        if path.is_file() {
            check_file(root, &path, &mut findings);
            continue;
        }
        let mut files = Vec::new();
        walk_files(&path, &mut files);
        for file in files {
            check_file(root, &file, &mut findings);
        }
    }
    findings
}

fn check_file(root: &Path, path: &Path, findings: &mut Vec<Finding>) {
    let extension = path.extension().and_then(|value| value.to_str());
    if extension != Some("rs")
        && path.file_name().and_then(|value| value.to_str()) != Some("Cargo.toml")
    {
        return;
    }
    let Ok(body) = fs::read_to_string(path) else {
        return;
    };
    for (index, line) in body.lines().enumerate() {
        let trimmed = line.trim_start();
        let manifest_dependency = path.file_name().and_then(|value| value.to_str())
            == Some("Cargo.toml")
            && (line.contains("specify-adapters")
                || CRATES.iter().any(|name| {
                    let dependency = name.replace('_', "-");
                    trimmed.starts_with(&format!("{dependency} ="))
                }));
        let rust_import = extension == Some("rs")
            && CRATES.iter().any(|name| {
                trimmed.starts_with(&format!("use {name}::"))
                    || trimmed.starts_with(&format!("extern crate {name}"))
            });
        if manifest_dependency || rust_import {
            findings.push(Finding::new(
                CHECK_ADAPTER_DEPENDENCY,
                format!("{}:{} imports a concrete adapter: {trimmed}", rel(root, path), index + 1),
            ));
        }
    }
}
