//! Regression coverage for the `plan transition` no-op contract:
//! stamping `approved` on an already-approved plan must neither
//! rewrite `plan.yaml` nor append a journal event (the auto-approve
//! Gate-1 contract — `Mutation::unchanged` skips both effects).

use std::fs;
use std::os::unix::fs::MetadataExt;

use workflow::change::plan::handlers::{Transition, TransitionInput};

mod common;

use common::{Project, run};

const PENDING_PLAN: &str = "\
name: demo
sources: {}
slices:
  - name: feature-x
    status: pending
";

fn approve_input() -> TransitionInput {
    serde_json::from_value(serde_json::json!({
        "name": "demo",
        "target": "approved",
    }))
    .expect("input deserialises")
}

fn journal_lines(project: &Project) -> Vec<String> {
    let path = project.root.join(".specify/journal.jsonl");
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(path)
        .expect("read journal")
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

#[tokio::test]
async fn repeated_approve_is_a_true_noop() {
    let project = Project::initialised();
    let plan_path = project.root.join("plan.yaml");
    fs::write(&plan_path, PENDING_PLAN).expect("stage plan.yaml");

    // First stamp: lifecycle moves and exactly one event lands.
    let body = run::<Transition, _>(&project, approve_input()).await.expect("Gate 1 stamps");
    assert_eq!(body.previous, "pending");
    assert_eq!(body.current, "approved");
    let events = journal_lines(&project);
    assert_eq!(events.len(), 1);
    assert!(events[0].contains("plan.transition.approved"), "{}", events[0]);

    // The atomic writer replaces the file, so a rewrite mints a new
    // inode; capture the stamped identity.
    let stamped = fs::metadata(&plan_path).expect("plan metadata");
    let stamped_bytes = fs::read(&plan_path).expect("plan bytes");

    // Second stamp: idempotent no-op — success envelope, no disk
    // write, no journal event.
    let body = run::<Transition, _>(&project, approve_input()).await.expect("re-stamp is a no-op");
    assert_eq!(body.previous, "approved");
    assert_eq!(body.current, "approved");

    let after = fs::metadata(&plan_path).expect("plan metadata");
    assert_eq!(
        (stamped.dev(), stamped.ino()),
        (after.dev(), after.ino()),
        "an already-approved transition must not rewrite plan.yaml"
    );
    assert_eq!(fs::read(&plan_path).expect("plan bytes"), stamped_bytes);
    assert_eq!(journal_lines(&project).len(), 1, "no second plan.transition.approved event");
}
