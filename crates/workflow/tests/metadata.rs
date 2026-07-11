//! Metadata timestamp serde through public lifecycle operations.

use std::fs;

mod common;

use common::{Project, run};

#[tokio::test]
async fn timestamps_round_trip_in_rfc3339() {
    let project = Project::initialised();
    let slice = project.root.join(".specify/slices/demo");
    fs::create_dir_all(&slice).expect("create slice");
    fs::write(
        slice.join("metadata.yaml"),
        "target: demo\nstatus: refining\ncreated-at: 2026-06-02T01:02:03+00:00\n\
         touched-specs: []\noutcome:\n  phase: guidance\n  outcome: success\n  \
         at: 2026-06-02T01:02:03+00:00\n  summary: ready\n",
    )
    .expect("stage metadata");

    run::<workflow::slice::handlers::Transition, _>(
        &project,
        workflow::slice::handlers::TransitionInput {
            name: "demo".to_string(),
            target: workflow::slice::LifecycleStatus::Refined,
        },
    )
    .await
    .expect("metadata parses and transition saves");

    let yaml = fs::read_to_string(slice.join("metadata.yaml")).expect("metadata");
    assert!(yaml.contains("created-at: 2026-06-02T01:02:03Z"), "{yaml}");
    assert!(yaml.contains("at: 2026-06-02T01:02:03Z"), "{yaml}");
}

#[tokio::test]
async fn malformed_timestamp_is_rejected() {
    let project = Project::initialised();
    let slice = project.root.join(".specify/slices/demo");
    fs::create_dir_all(&slice).expect("create slice");
    fs::write(
        slice.join("metadata.yaml"),
        "target: demo\nstatus: refining\ncreated-at: not-a-timestamp\ntouched-specs: []\n",
    )
    .expect("stage metadata");

    let err = run::<workflow::slice::handlers::Transition, _>(
        &project,
        workflow::slice::handlers::TransitionInput {
            name: "demo".to_string(),
            target: workflow::slice::LifecycleStatus::Refined,
        },
    )
    .await
    .expect_err("malformed timestamp must fail");

    assert!(matches!(err.core(), error::Error::YamlDe(_)), "{err:?}");
}
