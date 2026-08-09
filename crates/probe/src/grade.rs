//! Shared deterministic grading helpers for live-model eval cases.
//!
//! Anything catalog-specific stays out of this shared library.

use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result, ensure};
use artifacts::spec::provenance::{Requirement, RequirementStatus, parse_spec_md};
use project::config::Layout;

/// Every baseline `spec.md` requirement across domains.
///
/// # Errors
///
/// Returns specs-dir I/O failures and unreadable `spec.md` bodies.
pub fn baseline(root: &Path) -> Result<Vec<Requirement>> {
    let mut requirements = Vec::new();
    let specs = Layout::new(root).specs_dir();
    for domain in fs::read_dir(&specs)
        .with_context(|| format!("reading the baseline specs dir {}", specs.display()))?
    {
        let spec = domain.context("domain dir")?.path().join("spec.md");
        if spec.is_file() {
            let body = fs::read_to_string(&spec).context("reading a baseline spec")?;
            requirements.extend(parse_spec_md(&body).requirements);
        }
    }
    Ok(requirements)
}

/// Shared provenance gate: non-empty baseline, ids present, evidenced
/// requirements carry sources.
///
/// # Errors
///
/// Returns one failing assertion at a time, with the evidence inline.
pub fn provenance(requirements: &[Requirement]) -> Result<()> {
    ensure!(!requirements.is_empty(), "the baseline carries no requirements");
    for requirement in requirements {
        ensure!(!requirement.id.is_empty(), "requirement `{}` carries no id", requirement.name);
        if requirement.status != Some(RequirementStatus::Unknown) {
            ensure!(
                !requirement.sources.is_empty(),
                "evidenced requirement `{}` carries no provenance",
                requirement.name
            );
        }
    }
    Ok(())
}
