//! Slice-local requirement ids and MODIFIED baseline-body digests
//! (RFC-86 D5) through the public synthesis projection kernel.

use std::collections::BTreeMap;
use std::fs;

use artifacts::evidence::ClaimKind;
use diagnostics::digest::sha256_hex;
use slice::{BaselineIndex, ProjectionHeader, SliceModel, project};
use tempfile::TempDir;

fn header() -> ProjectionHeader {
    ProjectionHeader {
        version: 1,
        slice: "auth-login".into(),
        target: Some("demo".into()),
    }
}

const fn baseline_spec() -> &'static str {
    "### Requirement: Session timeout\n\n\
     ID: REQ-007\n\
     Sources: [docs]\n\
     Status: agreed\n\n\
     Sessions expire after 15 minutes of inactivity.\n\n\
     #### Scenario: Idle expiry\n\n\
     - **WHEN** a session is idle for 15 minutes\n\
     - **THEN** the system expires the session\n\n\
     ### Requirement: Remember me\n\n\
     ID: REQ-008\n\
     Sources: [docs]\n\
     Status: agreed\n\n\
     Users may opt into a persistent session.\n\n\
     #### Scenario: Persistent session\n\n\
     - **WHEN** a user enables remember-me\n\
     - **THEN** the session survives browser restart\n"
}

fn stage_baseline(body: &str) -> (TempDir, BaselineIndex) {
    let tmp = TempDir::new().expect("tempdir");
    let domain = tmp.path().join("auth");
    fs::create_dir_all(&domain).expect("domain dir");
    fs::write(domain.join("spec.md"), body).expect("write baseline");
    let index = BaselineIndex::build(tmp.path()).expect("baseline index");
    (tmp, index)
}

fn draft(yaml: &str) -> SliceModel {
    SliceModel::parse_yaml(yaml).expect("draft parses")
}

#[test]
fn mints_slice_local_ids() {
    let (_tmp, baseline) = stage_baseline(baseline_spec());
    let model = draft(
        r"
requirements:
  - title: sessions expire after inactivity
    domain: auth
    baseline-id: REQ-007
    claims:
      - source: docs
        id: session.timeout
        kind: requirement
    statement: Sessions expire after 30 minutes of inactivity.
    scenarios:
      - An idle session expires
  - title: password reset entry point
    domain: auth
    claims:
      - source: docs
        id: reset.entry
        kind: requirement
    statement: A password reset entry exists on the login screen.
    scenarios:
      - A user opens reset from login
tasks:
  - id: TASK-001
    text: Align the session TTL.
    satisfies: [REQ-001]
  - id: TASK-002
    text: Wire the reset entry.
    satisfies: [REQ-002]
",
    );

    let evidence = BTreeMap::from([
        (("docs".into(), "session.timeout".into()), ClaimKind::Requirement),
        (("docs".into(), "reset.entry".into()), ClaimKind::Requirement),
    ]);

    let projected =
        project(model, header(), &BTreeMap::new(), &BTreeMap::new(), &evidence, &baseline)
            .expect("projects");

    assert_eq!(projected.requirements[0].id.as_deref(), Some("REQ-001"));
    assert_eq!(projected.requirements[1].id.as_deref(), Some("REQ-002"));
    // Baseline already uses REQ-007 / REQ-008 — slice-local ids must not
    // continue from that max, and must not reuse the baseline numbers.
    assert_ne!(projected.requirements[0].id.as_deref(), Some("REQ-007"));
    assert_ne!(projected.requirements[1].id.as_deref(), Some("REQ-009"));
}

#[test]
fn modified_keeps_baseline() {
    let (_tmp, baseline) = stage_baseline(baseline_spec());
    let body = "Sessions expire after 15 minutes of inactivity.\n\n\
#### Scenario: Idle expiry\n\n\
- **WHEN** a session is idle for 15 minutes\n\
- **THEN** the system expires the session";
    let expected = format!("sha256:{}", sha256_hex(body.as_bytes()));

    let model = draft(
        r"
requirements:
  - title: sessions expire after inactivity
    domain: auth
    baseline-id: REQ-007
    claims:
      - source: docs
        id: session.timeout
        kind: requirement
    statement: Sessions expire after 30 minutes of inactivity.
    scenarios:
      - An idle session expires
tasks:
  - id: TASK-001
    text: Align the session TTL.
    satisfies: [REQ-001]
",
    );

    let evidence =
        BTreeMap::from([(("docs".into(), "session.timeout".into()), ClaimKind::Requirement)]);

    let projected =
        project(model, header(), &BTreeMap::new(), &BTreeMap::new(), &evidence, &baseline)
            .expect("projects");

    let req = &projected.requirements[0];
    assert_eq!(req.id.as_deref(), Some("REQ-001"));
    assert_eq!(req.baseline_id.as_deref(), Some("REQ-007"));
    assert_eq!(req.baseline_digest.as_deref(), Some(expected.as_str()));
}

#[test]
fn parallel_slices_mint() {
    let (_tmp, baseline) = stage_baseline(baseline_spec());
    let evidence =
        BTreeMap::from([(("docs".into(), "session.timeout".into()), ClaimKind::Requirement)]);

    let left = draft(
        r"
requirements:
  - title: left slice change
    domain: auth
    baseline-id: REQ-007
    claims:
      - source: docs
        id: session.timeout
        kind: requirement
    statement: Left rewrite of the timeout.
    scenarios:
      - Idle expires
tasks:
  - id: TASK-001
    text: Left task.
    satisfies: [REQ-001]
",
    );
    let right = draft(
        r"
requirements:
  - title: right slice change
    domain: auth
    baseline-id: REQ-008
    claims:
      - source: docs
        id: session.timeout
        kind: requirement
    statement: Right rewrite of remember-me.
    scenarios:
      - Remember me persists
tasks:
  - id: TASK-001
    text: Right task.
    satisfies: [REQ-001]
",
    );

    let left = project(
        left,
        ProjectionHeader {
            version: 1,
            slice: "left".into(),
            target: None,
        },
        &BTreeMap::new(),
        &BTreeMap::new(),
        &evidence,
        &baseline,
    )
    .expect("left");
    let right = project(
        right,
        ProjectionHeader {
            version: 1,
            slice: "right".into(),
            target: None,
        },
        &BTreeMap::new(),
        &BTreeMap::new(),
        &evidence,
        &baseline,
    )
    .expect("right");

    assert_eq!(left.requirements[0].id.as_deref(), Some("REQ-001"));
    assert_eq!(right.requirements[0].id.as_deref(), Some("REQ-001"));
    assert_eq!(left.requirements[0].baseline_id.as_deref(), Some("REQ-007"));
    assert_eq!(right.requirements[0].baseline_id.as_deref(), Some("REQ-008"));
    assert!(left.requirements[0].baseline_digest.is_some());
    assert!(right.requirements[0].baseline_digest.is_some());
    assert_ne!(
        left.requirements[0].baseline_digest, right.requirements[0].baseline_digest,
        "different baseline bodies yield different digests"
    );
}

#[test]
fn additive_row_clears() {
    let (_tmp, baseline) = stage_baseline(baseline_spec());
    let model = draft(
        r"
requirements:
  - title: brand-new behaviour
    domain: auth
    claims:
      - source: docs
        id: reset.entry
        kind: requirement
    statement: Net-new additive requirement.
    scenarios:
      - Something happens
tasks:
  - id: TASK-001
    text: Implement it.
    satisfies: [REQ-001]
",
    );
    let evidence =
        BTreeMap::from([(("docs".into(), "reset.entry".into()), ClaimKind::Requirement)]);

    let projected =
        project(model, header(), &BTreeMap::new(), &BTreeMap::new(), &evidence, &baseline)
            .expect("projects");

    assert_eq!(projected.requirements[0].id.as_deref(), Some("REQ-001"));
    assert_eq!(projected.requirements[0].baseline_id, None);
    assert_eq!(projected.requirements[0].baseline_digest, None);
}
