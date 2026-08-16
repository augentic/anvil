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

/// Concatenated baseline `spec.md` bodies across domains — the text
/// the blind acceptance set grades against.
///
/// # Errors
///
/// Returns specs-dir I/O failures and unreadable `spec.md` bodies.
pub fn baseline_text(root: &Path) -> Result<String> {
    let mut text = String::new();
    let specs = Layout::new(root).specs_dir();
    for domain in fs::read_dir(&specs)
        .with_context(|| format!("reading the baseline specs dir {}", specs.display()))?
    {
        let spec = domain.context("domain dir")?.path().join("spec.md");
        if spec.is_file() {
            text.push_str(&fs::read_to_string(&spec).context("reading a baseline spec")?);
            text.push('\n');
        }
    }
    Ok(text)
}

/// The blind acceptance set (RFC-96 D11).
///
/// Loaded from a TOML file beside `case.toml`, never copied into the
/// sandbox — the workflow's model calls cannot read it; only the
/// grading over completed outcomes consumes it.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Blind {
    /// Substrings each of which must appear in the accepted baseline.
    pub accept: Vec<String>,
}

/// Load and validate one blind acceptance set.
///
/// # Errors
///
/// Returns a missing or malformed file and an empty `accept` list.
pub fn load_blind(path: &Path) -> Result<Blind> {
    let body = fs::read_to_string(path)
        .with_context(|| format!("reading the blind set {}", path.display()))?;
    let blind: Blind = toml::from_str(&body)
        .with_context(|| format!("parsing the blind set {}", path.display()))?;
    ensure!(!blind.accept.is_empty(), "the blind set declares no `accept` entries");
    for needle in &blind.accept {
        ensure!(!needle.trim().is_empty(), "empty `accept` entry in the blind set");
    }
    Ok(blind)
}

/// Grade the blind acceptance set against the accepted baseline text.
///
/// # Errors
///
/// Returns the first unmatched `accept` entry.
pub fn blind(baseline_text: &str, blind: &Blind) -> Result<()> {
    for needle in &blind.accept {
        ensure!(
            baseline_text.contains(needle),
            "the accepted baseline does not satisfy the blind acceptance entry `{needle}`"
        );
    }
    Ok(())
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
