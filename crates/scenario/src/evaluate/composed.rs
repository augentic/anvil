//! Pure evaluators for the `composed-loop` registered probes.
//!
//! The composed replay scenario proves the inverted loop through the
//! hosted workflow and echo adapter components; these evaluators
//! settle its three `kind: registered` assertions against the trial
//! workspace artifacts.

use std::fs;

use crate::grade::{Execution, Verdict};

/// The plan drained: one `done` entry, nothing pending or in progress
/// (`composed-plan-drained`).
#[must_use]
pub fn plan_drained(execution: &Execution) -> Verdict {
    let evidence = "plan.yaml";
    let Ok(plan) = fs::read_to_string(execution.root().join(evidence)) else {
        return Verdict::fail(evidence, "plan.yaml not readable");
    };
    if plan.contains("status: done")
        && !plan.contains("status: pending")
        && !plan.contains("status: in-progress")
    {
        Verdict::pass(evidence)
    } else {
        Verdict::fail(evidence, "plan.yaml still carries pending or in-progress entries")
    }
}

/// The merged baseline carries the projected requirement and its
/// provenance (`composed-artifacts-complete`).
#[must_use]
pub fn artifacts_complete(execution: &Execution) -> Verdict {
    let evidence = ".specify/specs/echo/spec.md";
    let complete = fs::read_to_string(execution.root().join(evidence))
        .is_ok_and(|spec| spec.contains("REQ-001") && spec.contains("Sources: echo"));
    if complete {
        Verdict::pass(evidence)
    } else {
        Verdict::fail(evidence, "baseline spec is missing the projected requirement or provenance")
    }
}

/// The merge landed a visible baseline spec
/// (`composed-baseline-merge-visible`).
#[must_use]
pub fn baseline_merge_visible(execution: &Execution) -> Verdict {
    let evidence = ".specify/specs/echo/spec.md";
    if execution.root().join(evidence).is_file() {
        Verdict::pass(evidence)
    } else {
        Verdict::fail(evidence, "merge left no baseline spec")
    }
}
