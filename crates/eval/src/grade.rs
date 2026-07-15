//! Structural checks after execute, before finalize (`plan.yaml` still live).

use std::fs;
use std::path::Path;

use artifacts::spec::provenance::{Requirement, RequirementStatus, parse_spec_md};
use change::Plan;
use project::config::Layout;
use testkit::adapter::build_artifact_path;

/// Grade the drained plan against the adversarial fixture contract.
pub fn run(root: &Path, plan: &Plan) {
    requirements(&baseline(root));
    outputs(root, plan);
}

fn requirements(requirements: &[Requirement]) {
    assert!(!requirements.is_empty(), "the baseline carries no requirements");
    for requirement in requirements {
        assert!(!requirement.id.is_empty(), "requirement `{}` carries no id", requirement.name);
        if requirement.status != Some(RequirementStatus::Unknown) {
            assert!(
                !requirement.sources.is_empty(),
                "evidenced requirement `{}` carries no provenance",
                requirement.name
            );
        }
    }
    assert!(
        requirements.iter().any(|requirement| matches!(
            requirement.status,
            Some(RequirementStatus::Divergence | RequirementStatus::Conflict)
        )),
        "the session-timeout disagreement must surface as a divergence or conflict: {requirements:?}"
    );
    assert!(
        requirements
            .iter()
            .any(|requirement| requirement.status == Some(RequirementStatus::Unknown)),
        "the password-reset gap must be marked unknown — the faithful answer records zero \
         contributing claims for the unevidenced lead (an answer that anchors it to the bare \
         `password-reset.mention` section claim projects `agreed` instead): {requirements:?}"
    );
}

fn outputs(root: &Path, plan: &Plan) {
    for entry in &plan.entries {
        let artifact = build_artifact_path(root, &entry.name);
        let body = fs::read_to_string(&artifact)
            .unwrap_or_else(|err| panic!("build output for `{}`: {err}", entry.name));
        assert!(!body.trim().is_empty(), "empty build output for `{}`", entry.name);
    }
}

fn baseline(root: &Path) -> Vec<Requirement> {
    let mut requirements = Vec::new();
    for domain in fs::read_dir(Layout::new(root).specs_dir()).expect("baseline specs dir") {
        let spec = domain.expect("domain dir").path().join("spec.md");
        if spec.is_file() {
            let body = fs::read_to_string(&spec).expect("read baseline spec");
            requirements.extend(parse_spec_md(&body).requirements);
        }
    }
    requirements
}
