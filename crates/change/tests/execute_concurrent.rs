//! Cut A5 (RFC-96): the concurrent execute drain. Cap-one and
//! cap-four accept the same CIDs on an independent two-target plan
//! (cap-four overlaps the cross-target builds in one pool round), a
//! same-target ready group freezes one multi-member wave that a
//! single merge commits atomically (RFC-96 D7), a failed sibling
//! build parks the run while completed siblings keep their build
//! records, and `guest.lock` still excludes a second supervisor.

mod support;

use std::fs;

use change::plan::handlers::{Execute, ExecuteBody, ExecuteInput};
use mock::invoke::run;
use mock::session::Session;
use project::adapter::catalog::Pin;
use project::handler::Anchor as _;
use project::journal::{EventKind, read_union};
use project::plan::{LoopStep, Plan, TargetBinding};
use project::snapshot::SnapshotId;
use project::wave::accepted_cid;

#[expect(unsafe_code, reason = "EMERY_POOL is the launcher cap seam; nextest isolates the process")]
fn set_cap(cap: &str) {
    // SAFETY: nextest runs each test in its own process, and the env
    // write happens before any pool dispatch reads the cap.
    unsafe { std::env::set_var("EMERY_POOL", cap) };
}

async fn seed_cid(session: &Session, name: &str) -> SnapshotId {
    let home = session.root().parent().expect("session home");
    let seed = home.join(name);
    fs::create_dir_all(seed.join(".emery")).expect("seed .emery");
    fs::write(seed.join("README.md"), format!("{name}\n")).expect("seed file");
    session.store().snapshot(&seed).await.expect("snapshot seed")
}

fn adapter_target(adapter: &str, cid: SnapshotId) -> TargetBinding {
    TargetBinding::new(Pin::emery(adapter, semver::Version::new(0, 0, 0)), ".", cid)
}

/// Two independent slices on two targets — no `depends-on`, so the
/// ready set dispatches both builds in one round: `alpha` on `app`
/// (healthy mock), `beta` on `other` (bound to `other_adapter`).
fn write_two_target(session: &Session, app: SnapshotId, other: SnapshotId, other_adapter: &str) {
    support::write_plan_fixture(
        session.root(),
        "demo",
        &[("main", "mock", "The greeting service.")],
        &[("alpha", "main", "greeting"), ("beta", "main", "greeting")],
    );
    let layout = session.provider().paths().layout();
    let mut plan = Plan::load(&layout.plan_path()).expect("plan");
    plan.targets.insert("app".into(), adapter_target("mock", app));
    plan.targets.insert("other".into(), adapter_target(other_adapter, other));
    plan.targets.remove("default");
    for entry in &mut plan.entries {
        entry.target = if entry.name.as_str() == "alpha" { "app".into() } else { "other".into() };
    }
    plan.save(&layout.plan_path()).expect("plan.yaml");
}

fn stage_slices(root: &std::path::Path, slices: &[(&str, &str)]) {
    for (slice, adapter) in slices {
        let dir = support::fixture_layout(root).slice_dir(slice);
        fs::create_dir_all(&dir).expect("slice dir");
        fs::write(
            dir.join("metadata.yaml"),
            format!(
                "target: {adapter}@0.0.0\ncreated-at: 2026-01-01T00:00:00Z\ndefined-at: \
                 2026-01-01T00:00:01Z\n"
            ),
        )
        .expect("metadata");
        support::stage_manifest(root, slice);
    }
}

fn ran_phases(executed: &ExecuteBody) -> Vec<(String, LoopStep)> {
    executed.phases.iter().map(|phase| (phase.slice.clone(), phase.step)).collect()
}

/// Drain the two-target fixture at one cap; returns the per-target
/// accepted CIDs and the run's phase sequence.
async fn drain_two_targets(cap: &str) -> (SnapshotId, SnapshotId, Vec<(String, LoopStep)>) {
    set_cap(cap);
    let session = Session::scripted("mock", Vec::new());
    let app = seed_cid(&session, "seed-app").await;
    let other = seed_cid(&session, "seed-other").await;
    write_two_target(&session, app, other, "mock");
    stage_slices(session.root(), &[("alpha", "mock"), ("beta", "mock")]);

    let executed = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect("execute drains");
    assert_eq!(executed.status, "drained", "{executed:?}");

    let layout = session.provider().paths().layout();
    let events = read_union(layout).expect("union");
    let app_cid = accepted_cid(layout, &events, "app").expect("app chain").expect("app accepted");
    let other_cid =
        accepted_cid(layout, &events, "other").expect("other chain").expect("other accepted");
    (app_cid, other_cid, ran_phases(&executed))
}

// Cap-one is the serial reference: cap-four overlaps the cross-target
// builds in one pool round yet accepts identical CIDs per target.
#[tokio::test]
async fn cap_equivalence() {
    let (app_serial, other_serial, _) = drain_two_targets("1").await;
    let (app_pooled, other_pooled, phases) = drain_two_targets("4").await;
    assert_eq!(app_serial, app_pooled, "app accepts the same CID at every cap");
    assert_eq!(other_serial, other_pooled, "other accepts the same CID at every cap");
    assert_eq!(
        phases[..2],
        [("alpha".to_string(), LoopStep::Build), ("beta".to_string(), LoopStep::Build)],
        "cap-four dispatches the cross-target builds in one round: {phases:?}"
    );
}

// A same-target ready group freezes one two-member wave (RFC-96 D7):
// both builds join it and the single merge at the canonical head
// commits every member atomically — one `target.wave.opened` naming
// both members, one `target.merge.wave-committed`, no second merge.
#[tokio::test]
async fn same_target_wave() {
    set_cap("4");
    let session = Session::scripted("mock", Vec::new());
    let app = seed_cid(&session, "seed-app").await;
    let other = seed_cid(&session, "seed-other").await;
    write_two_target(&session, app, other, "mock");
    let layout = session.provider().paths().layout();
    let mut plan = Plan::load(&layout.plan_path()).expect("plan");
    for entry in &mut plan.entries {
        entry.target = "app".into();
    }
    plan.targets.remove("other");
    plan.save(&layout.plan_path()).expect("plan.yaml");
    stage_slices(session.root(), &[("alpha", "mock"), ("beta", "mock")]);

    let executed = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect("execute drains");
    assert_eq!(executed.status, "drained", "{executed:?}");
    assert_eq!(
        ran_phases(&executed),
        [
            ("alpha".to_string(), LoopStep::Build),
            ("beta".to_string(), LoopStep::Build),
            ("alpha".to_string(), LoopStep::Merge),
        ],
        "one merge commits the whole wave"
    );

    let events = read_union(layout).expect("union");
    let opened: Vec<Vec<&str>> = events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::TargetWaveOpened { members, .. } => {
                Some(members.iter().map(project::name::SliceName::as_str).collect())
            }
            _ => None,
        })
        .collect();
    assert_eq!(opened, [vec!["alpha", "beta"]], "one frozen two-member wave");
    let committed: Vec<Vec<&str>> = events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::TargetMergeWaveCommitted { members, .. } => {
                Some(members.iter().map(project::name::SliceName::as_str).collect())
            }
            _ => None,
        })
        .collect();
    assert_eq!(committed, [vec!["alpha", "beta"]], "the wave commits atomically");
}

// A group-wide build failure resumes under the same frozen wave: the
// re-run rejoins the existing membership instead of opening a second
// wave — failed build, new attempt, same membership (RFC-96 D7).
#[tokio::test]
async fn retry_rejoins_wave() {
    set_cap("4");
    let session = Session::scripted("mock", Vec::new());
    let app = seed_cid(&session, "seed-app").await;
    let other = seed_cid(&session, "seed-other").await;
    write_two_target(&session, app, other, "mock");
    let layout = session.provider().paths().layout();
    let mut plan = Plan::load(&layout.plan_path()).expect("plan");
    for entry in &mut plan.entries {
        entry.target = "app".into();
    }
    plan.targets.remove("other");
    plan.save(&layout.plan_path()).expect("plan.yaml");
    stage_slices(session.root(), &[("alpha", "mock"), ("beta", "mock")]);

    support::marker(session.root(), mock::behaviour::FAIL_BUILD_MARKER);
    let stopped = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("the marked builds park the run")
        .to_string();
    assert!(stopped.contains("build-failed"), "{stopped}");

    fs::remove_file(session.root().join(mock::behaviour::FAIL_BUILD_MARKER)).expect("rm marker");
    let executed = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect("the resume drains");
    assert_eq!(executed.status, "drained", "{executed:?}");

    let events = read_union(layout).expect("union");
    let opened: Vec<usize> = events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::TargetWaveOpened { members, .. } => Some(members.len()),
            _ => None,
        })
        .collect();
    assert_eq!(opened, [2], "the retry rejoins the frozen wave — no second open");
}

// A failed sibling build parks the run with the typed stop naming the
// failing slice, while the completed sibling keeps its build record;
// fixing the binding and re-running resumes from the parked phase
// without rebuilding the healthy sibling.
#[tokio::test]
async fn park_resume_with_siblings() {
    set_cap("4");
    let session = Session::scripted("mock", Vec::new());
    let app = seed_cid(&session, "seed-app").await;
    let other = seed_cid(&session, "seed-other").await;
    write_two_target(&session, app, other.clone(), "mock-fail-build");
    stage_slices(session.root(), &[("alpha", "mock"), ("beta", "mock-fail-build")]);

    let stopped = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("the failing build parks the run")
        .to_string();
    assert!(stopped.contains("plan-execute-stopped"), "{stopped}");
    assert!(stopped.contains("build-failed"), "{stopped}");
    assert!(stopped.contains("beta"), "the stop names the failing slice: {stopped}");

    // Fix the input: rebind `other` onto the healthy mock adapter. The
    // adapter pin is not a refinement input (the profile digest is),
    // so the manifest stays fresh and execute resumes without a
    // re-refine.
    let layout = session.provider().paths().layout();
    let mut plan = Plan::load(&layout.plan_path()).expect("plan");
    plan.targets.insert("other".into(), adapter_target("mock", other));
    plan.save(&layout.plan_path()).expect("plan.yaml");
    fs::write(
        layout.slice_dir("beta").join("metadata.yaml"),
        "target: mock@0.0.0\ncreated-at: 2026-01-01T00:00:00Z\ndefined-at: \
         2026-01-01T00:00:01Z\n",
    )
    .expect("metadata");

    let resumed = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect("re-running execute resumes");
    assert_eq!(resumed.status, "drained", "{resumed:?}");
    let phases = ran_phases(&resumed);
    assert!(
        !phases.contains(&("alpha".to_string(), LoopStep::Build)),
        "the healthy sibling's build record survives the park: {phases:?}"
    );
    let events = read_union(layout).expect("union");
    assert!(accepted_cid(layout, &events, "app").expect("app").is_some());
    assert!(accepted_cid(layout, &events, "other").expect("other").is_some());
}

// `guest.lock` stays the one-supervisor interlock: a held marker
// refuses a second execute before any epoch or dispatch.
#[tokio::test]
async fn second_supervisor_locked() {
    let session = Session::scripted("mock", Vec::new());
    support::write_greeting_plan(session.root());

    let layout = session.provider().paths().layout();
    let lock = layout.guest_lock_path();
    fs::create_dir_all(lock.parent().expect("lock parent")).expect("change home");
    fs::write(&lock, "pid: 1\nstarted-at: 2026-01-01T00:00:00Z\n").expect("hold the marker");

    let refused = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("a held guest.lock refuses the second supervisor")
        .to_string();
    assert!(refused.contains("guest-marker-held"), "{refused}");
}
