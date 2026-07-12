//! Registered-probe evaluators for the `guest-execute-loop` scenario.
//!
//! The canonical scenario declares two assertions with `kind:
//! registered` probes — evidence a generic path/exit/JSON probe cannot
//! express. This module settles them against the trial workspace,
//! mirroring [`crate::grade::hard`]'s verdict shape.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::{AssertionId, AssertionResult, Outcome};

/// Journal events the guest loop must emit for one drained slice.
const CADENCE_EVENTS: [&str; 2] = ["slice.merge.succeeded", "slice.archive.created"];

/// Settle the registered `guest-*` assertions against the trial
/// workspace at `root`. Assertions with other ids pass through
/// untouched; generic probes are already settled by
/// [`crate::grade::hard`].
pub fn guest(assertions: &mut [AssertionResult], root: &Path) {
    for assertion in assertions {
        let (passed, evidence, detail) = match assertion.id {
            AssertionId::GuestJournalCadence => journal_cadence(root),
            AssertionId::GuestGeneratedCrateVerifies => generated_crates_verify(root),
            _ => continue,
        };
        assertion.outcome = if passed { Outcome::Pass } else { Outcome::Fail };
        assertion.evidence = Some(evidence);
        assertion.detail = detail;
    }
}

/// The guest loop journalled the merge and archive events over the
/// `"."` preopen.
fn journal_cadence(root: &Path) -> (bool, String, Option<String>) {
    let path = root.join(".specify/journal.jsonl");
    let Ok(journal) = fs::read_to_string(&path) else {
        return (
            false,
            ".specify/journal.jsonl".to_owned(),
            Some(format!("journal not readable at {}", path.display())),
        );
    };
    let missing: Vec<&str> = CADENCE_EVENTS
        .iter()
        .filter(|event| !journal.contains(&format!("\"{event}\"")))
        .copied()
        .collect();
    if missing.is_empty() {
        (true, ".specify/journal.jsonl".to_owned(), None)
    } else {
        (
            false,
            ".specify/journal.jsonl".to_owned(),
            Some(format!("journal is missing {}", missing.join(", "))),
        )
    }
}

/// Every generated crate under `crates/` passes its own `cargo check`;
/// a run that generated no crate fails the gate.
fn generated_crates_verify(root: &Path) -> (bool, String, Option<String>) {
    let evidence = "crates/".to_owned();
    let crates = root.join("crates");
    let Ok(entries) = fs::read_dir(&crates) else {
        return (false, evidence, Some("no generated crates/ directory".to_owned()));
    };
    let manifests: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("Cargo.toml"))
        .filter(|manifest| manifest.is_file())
        .collect();
    if manifests.is_empty() {
        return (false, evidence, Some("no generated crate manifests under crates/".to_owned()));
    }
    for manifest in manifests {
        match Command::new("cargo").arg("check").arg("--manifest-path").arg(&manifest).output() {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                return (
                    false,
                    evidence,
                    Some(format!(
                        "cargo check failed for {}: {}",
                        manifest.display(),
                        String::from_utf8_lossy(&output.stderr)
                    )),
                );
            }
            Err(error) => {
                return (
                    false,
                    evidence,
                    Some(format!("cargo check could not run for {}: {error}", manifest.display())),
                );
            }
        }
    }
    (true, evidence, None)
}
