//! RFC-88 D7: detached load without `project.yaml`, and a two-target
//! drain that advances one accepted CID per target without writing the
//! checkout.

mod support;

use std::fs;

use change::plan::handlers::{Execute, ExecuteInput, Gaps, GapsInput, Status, StatusInput};
use mock::invoke::run;
use mock::session::Session;
use project::adapter::catalog::Pin;
use project::handler::Anchor;
use project::journal::{EventKind, read_union};
use project::plan::{Plan, TargetBinding};
use project::snapshot::SnapshotId;
use project::wave::accepted_cid;

async fn seed_cid(session: &Session, name: &str) -> SnapshotId {
    let home = session.root().parent().expect("session home");
    let seed = home.join(name);
    fs::create_dir_all(seed.join(".emery")).expect("seed .emery");
    fs::write(seed.join("README.md"), format!("{name}\n")).expect("seed file");
    session.store().snapshot(&seed).await.expect("snapshot seed")
}

fn mock_target(cid: SnapshotId) -> TargetBinding {
    TargetBinding::new(Pin::emery("mock", semver::Version::new(0, 0, 0)), ".", cid)
}

fn write_two_target(session: &Session, app: SnapshotId, other: SnapshotId) {
    support::write_plan_fixture(
        session.root(),
        "demo",
        &[("main", "mock", "The greeting service.")],
        &[("alpha", "main", "greeting"), ("beta", "main", "greeting")],
    );
    let layout = session.provider().paths().layout();
    let mut plan = Plan::load(&layout.plan_path()).expect("plan");
    plan.targets.insert("app".into(), mock_target(app));
    plan.targets.insert("other".into(), mock_target(other));
    plan.targets.remove("default");
    for entry in &mut plan.entries {
        entry.target = if entry.name.as_str() == "alpha" { "app".into() } else { "other".into() };
    }
    plan.entries.iter_mut().find(|entry| entry.name.as_str() == "beta").expect("beta").depends_on =
        vec!["alpha".into()];
    plan.save(&layout.plan_path()).expect("plan.yaml");
}

fn stage_slices(root: &std::path::Path) {
    for slice in ["alpha", "beta"] {
        let dir = support::fixture_layout(root).slice_dir(slice);
        fs::create_dir_all(&dir).expect("slice dir");
        fs::write(
            dir.join("metadata.yaml"),
            "target: mock@0.0.0\ncreated-at: 2026-01-01T00:00:00Z\ndefined-at: 2026-01-01T00:00:01Z\n",
        )
        .expect("metadata");
        support::stage_manifest(root, slice);
    }
}

#[tokio::test]
async fn two_target_drain() {
    let session = Session::scripted("mock", Vec::new());
    let app = seed_cid(&session, "seed-app").await;
    let other = seed_cid(&session, "seed-other").await;
    write_two_target(&session, app.clone(), other.clone());
    stage_slices(session.root());

    let executed = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect("execute drains");
    assert_eq!(executed.status, "drained", "{executed:?}");

    let layout = session.provider().paths().layout();
    let events = read_union(layout).expect("union");
    let app_cid = accepted_cid(layout, &events, "app").expect("app chain").expect("app accepted");
    let other_cid =
        accepted_cid(layout, &events, "other").expect("other chain").expect("other accepted");
    assert_ne!(app_cid, other, "app merge advanced off the seed");
    assert_ne!(other_cid, app, "other merge advanced off the seed");
    assert_ne!(app_cid, other_cid, "each target keeps its own accepted CID");

    let opened: Vec<&str> = events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::TargetWaveOpened { target, .. } => Some(target.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(opened, ["app", "other"], "cross-target dep opens the dependent wave later");

    assert!(
        !session.root().join(".emery/specs/alpha").exists()
            && !session.root().join(".emery/specs/beta").exists(),
        "checkout is never a write target"
    );
    let app_tree = session.materialize_accepted("app").await;
    let other_tree = session.materialize_accepted("other").await;
    assert!(app_tree.path().exists(), "app accepted CID materializes");
    assert!(other_tree.path().exists(), "other accepted CID materializes");
}

#[tokio::test]
async fn detached_verbs() {
    let session = Session::detached(Vec::new());
    assert!(!session.root().join(".emery/project.yaml").exists(), "no init");

    let cid = seed_cid(&session, "seed-app").await;
    let other = seed_cid(&session, "seed-other").await;
    write_two_target(&session, cid, other);
    stage_slices(session.root());

    let status =
        run::<Status, _, _>(session.provider(), StatusInput {}).await.expect("detached status");
    assert_eq!(status.plan, "demo");

    run::<Gaps, _, _>(session.provider(), GapsInput {}).await.expect("detached gaps");
    run::<slice::handlers::Debt, _, _>(session.provider(), slice::handlers::DebtInput {})
        .await
        .expect("detached debt");

    let executed = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect("detached execute");
    assert_eq!(executed.status, "drained", "{executed:?}");

    let layout = session.provider().paths().layout();
    assert_eq!(layout.plan_path(), session.root().join("plan.yaml"));
    let events = read_union(layout).expect("union");
    assert!(
        events.iter().any(|event| matches!(event.kind, EventKind::PlanExecuteStarted { .. })),
        "detached execute opens an epoch"
    );
    assert!(
        accepted_cid(layout, &events, "app").expect("app").is_some(),
        "detached drain accepts app"
    );
    assert!(
        accepted_cid(layout, &events, "other").expect("other").is_some(),
        "detached drain accepts other"
    );
}
