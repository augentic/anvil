//! `plan drop` scope authority: the `slice.dropped` tombstone is
//! recorded before archive movement and survives it (S7 / CC-03), so a
//! dropped entry stays excluded after its stamped `metadata.yaml`
//! leaves the live tree.

mod support;

use change::plan::handlers::{Drop, DropInput, Status as StatusOp, StatusInput};
use mock::invoke::run;
use mock::session::Session;
use project::journal::EventKind;
use support::write_greeting_plan;

#[tokio::test]
async fn tombstone_survives_archive() {
    let session = Session::scripted("mock", Vec::new());
    let root = session.root();
    write_greeting_plan(root);
    let slice_dir = root.join(".emery/change/slices/greeting");
    std::fs::create_dir_all(&slice_dir).expect("slice dir");
    std::fs::write(slice_dir.join("metadata.yaml"), "target: demo-target@1.0.0\n")
        .expect("metadata");

    let body = run::<Drop, _, _>(
        session.provider(),
        DropInput {
            name: "greeting".into(),
            reason: Some("cut from scope".into()),
        },
    )
    .await
    .expect("drop");
    assert_eq!(body.name, "greeting");
    assert!(!slice_dir.exists(), "slice tree archived out of the live tree");

    let layout = support::fixture_layout(root);
    let events = project::journal::read_union(layout).expect("events");
    assert!(
        events.iter().any(|event| matches!(
            &event.kind,
            EventKind::SliceDropped { slice_name, .. } if slice_name == "greeting"
        )),
        "tombstone fact recorded"
    );

    // The dropped entry must not re-enter scope as pending refine work
    // now that its live metadata is gone.
    let status = run::<StatusOp, _, _>(session.provider(), StatusInput {}).await.expect("status");
    assert_ne!(status.next_action, "refine greeting", "dropped entry re-entered scope");
    assert_eq!(status.next_action, "stop slice-dropped");
}
