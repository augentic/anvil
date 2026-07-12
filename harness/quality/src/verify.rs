//! Process-spawning evaluators for `kind: registered` probes.
//!
//! `crates/scenario` ships only pure evaluators; anything that runs a
//! subprocess is registered by the owning harness. This module owns
//! generated-output verification for the guest loop.

use std::fs;

use scenario::grade::{Execution, Verdict};

/// Every generated crate under `crates/` passes its own `cargo check`;
/// a run that generated no crate fails the gate
/// (`guest-generated-crate-verifies`).
#[must_use]
pub fn generated_crates_verify(execution: &Execution) -> Verdict {
    let evidence = "crates/";
    let crates = execution.root().join("crates");
    let Ok(entries) = fs::read_dir(&crates) else {
        return Verdict::fail(evidence, "no generated crates/ directory");
    };
    let manifests: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("Cargo.toml"))
        .filter(|manifest| manifest.is_file())
        .collect();
    if manifests.is_empty() {
        return Verdict::fail(evidence, "no generated crate manifests under crates/");
    }
    for manifest in manifests {
        match std::process::Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(&manifest)
            .output()
        {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                return Verdict::fail(
                    evidence,
                    format!(
                        "cargo check failed for {}: {}",
                        manifest.display(),
                        String::from_utf8_lossy(&output.stderr)
                    ),
                );
            }
            Err(error) => {
                return Verdict::fail(
                    evidence,
                    format!("cargo check could not run for {}: {error}", manifest.display()),
                );
            }
        }
    }
    Verdict::pass(evidence)
}
