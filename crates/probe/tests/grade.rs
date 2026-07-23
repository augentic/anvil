//! Shared trial grading helpers: provenance gate and baseline walk.

use std::fs;

use artifacts::spec::provenance::{Requirement, RequirementStatus, Span};
use probe::grade;
use tempfile::TempDir;

#[test]
fn provenance_accepts_evidenced_and_unknown() {
    grade::provenance(&[
        req("Login", "REQ-001", vec!["docs".into()], Some(RequirementStatus::Agreed)),
        req("Reset", "REQ-002", vec![], Some(RequirementStatus::Unknown)),
    ])
    .expect("shared provenance gate accepts a clean baseline");
}

#[test]
fn provenance_refuses_empty() {
    let err = grade::provenance(&[]).expect_err("empty baseline refuses");
    assert!(format!("{err:#}").contains("no requirements"), "{err:#}");
}

#[test]
fn provenance_refuses_missing_id() {
    let err = grade::provenance(&[req(
        "Login",
        "",
        vec!["docs".into()],
        Some(RequirementStatus::Agreed),
    )])
    .expect_err("missing id refuses");
    assert!(format!("{err:#}").contains("no id"), "{err:#}");
}

#[test]
fn provenance_refuses_evidenced_without_sources() {
    let err =
        grade::provenance(&[req("Login", "REQ-001", vec![], Some(RequirementStatus::Agreed))])
            .expect_err("evidenced without sources refuses");
    assert!(format!("{err:#}").contains("no provenance"), "{err:#}");
}

#[test]
fn baseline_reads_domain_specs() {
    let tmp = TempDir::new().expect("tempdir");
    let domain = tmp.path().join(".specify/specs/auth");
    fs::create_dir_all(&domain).expect("domain");
    fs::write(
        domain.join("spec.md"),
        "### Requirement: Login\n\nID: REQ-001\nSources: docs\nStatus: agreed\n\nBody.\n",
    )
    .expect("spec");
    let requirements = grade::baseline(tmp.path()).expect("baseline");
    assert_eq!(requirements.len(), 1);
    assert_eq!(requirements[0].id, "REQ-001");
}

fn req(
    name: &str, id: &str, sources: Vec<String>, status: Option<RequirementStatus>,
) -> Requirement {
    Requirement {
        id: id.into(),
        name: name.into(),
        sources,
        status,
        status_raw: status.map(|s| s.to_string()),
        tag: None,
        sources_line_absent: false,
        body: String::new(),
        span: Span {
            byte_start: 0,
            byte_end: 0,
            line_start: 1,
        },
    }
}
