//! Structural checks after execute, before finalize (`plan.yaml` still live).

use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result, ensure};
use artifacts::spec::provenance::{Requirement, RequirementStatus, parse_spec_md};
use change::Plan;
use change::plan::handlers::ExecuteBody;
use project::config::Layout;
use testkit::adapter::build_artifact_path;

/// Grade the drained plan against the adversarial fixture contract.
///
/// # Errors
///
/// Returns one failing assertion at a time, with the evidence inline.
pub fn run(root: &Path, plan: &Plan, _executed: &ExecuteBody) -> Result<()> {
    requirements(&baseline(root)?)?;
    outputs(root, plan)?;
    Ok(())
}

fn requirements(requirements: &[Requirement]) -> Result<()> {
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
    ensure!(
        requirements.iter().any(|requirement| matches!(
            requirement.status,
            Some(RequirementStatus::Divergence | RequirementStatus::Conflict)
        )),
        "the session-timeout disagreement must surface as a divergence or conflict: \
         {requirements:?}"
    );
    ensure!(
        requirements
            .iter()
            .any(|requirement| requirement.status == Some(RequirementStatus::Unknown)),
        "the password-reset gap must be marked unknown — the faithful answer records zero \
         contributing claims for the unevidenced lead (an answer that anchors it to the bare \
         `password-reset.mention` section claim projects `agreed` instead): {requirements:?}"
    );
    Ok(())
}

fn outputs(root: &Path, plan: &Plan) -> Result<()> {
    for entry in &plan.entries {
        let artifact = build_artifact_path(root, &entry.name);
        let body = fs::read_to_string(&artifact)
            .with_context(|| format!("build output for `{}`", entry.name))?;
        ensure!(!body.trim().is_empty(), "empty build output for `{}`", entry.name);
    }
    Ok(())
}

fn baseline(root: &Path) -> Result<Vec<Requirement>> {
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
