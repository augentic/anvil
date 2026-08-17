//! `plan author --force` rebinds the same reviewed handoff.

use change::plan;
use mock::definition::{Spec, mint};
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::plan::Plan;

fn seed_target(root: &std::path::Path) -> std::path::PathBuf {
    let target = root.join("target-app");
    std::fs::create_dir_all(target.join(".emery")).expect("target .emery");
    std::fs::write(target.join(".emery/project.yaml"), "name: app\nadapter: omnia\nrules: {}\n")
        .expect("project.yaml");
    target
}

async fn author(
    session: &Session, from: &std::path::Path, wave: &str, force: bool,
) -> Result<plan::handlers::AuthorBody, project::handler::Error> {
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            from: from.to_path_buf(),
            wave: wave.to_string(),
            force,
        },
    )
    .await
}

// Re-entry on a reconciled plan without `--force` is a read-only
// no-op: same outcome, no model call, no artifact write.
#[tokio::test]
async fn reconciled_reentry_noop() {
    let session = Session::scripted("mock", mock::answers::greeting_author());
    let target = seed_target(session.root());
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship it.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    let first = author(&session, &definition, &spec.wave, false).await.expect("first");
    session.model().assert_exhausted();
    let second = author(&session, &definition, &spec.wave, false).await.expect("re-entry no-op");
    assert_eq!(second.decomposition_digest, first.decomposition_digest);
    assert_eq!(second.slices, first.slices);
    session.model().assert_exhausted();
}

// A different plan name over an existing plan still refuses without
// `--force`.
#[tokio::test]
async fn name_mismatch_refused() {
    let session = Session::scripted("mock", mock::answers::greeting_author());
    let target = seed_target(session.root());
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship it.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    author(&session, &definition, &spec.wave, false).await.expect("first");
    let err = run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "other-demo".to_string(),
            from: definition.clone(),
            wave: spec.wave.clone(),
            force: false,
        },
    )
    .await
    .expect_err("second name");
    assert!(err.to_string().contains("plan-already-exists"), "{err}");
}

#[tokio::test]
async fn force_replaces() {
    let mut answers = mock::answers::greeting_author();
    answers.extend(mock::answers::greeting_author());
    let session = Session::scripted("mock", answers);
    let target = seed_target(session.root());
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship it.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    author(&session, &definition, &spec.wave, false).await.expect("first");
    author(&session, &definition, &spec.wave, true).await.expect("force");
}

// S26 regression: a `--force` run that parks reopens authoring — the
// historical reconcile fact must not project the parked plan as
// reconciled, topology verbs must refuse, and re-entry must resume.
#[tokio::test]
async fn force_park_then_resume() {
    let mut answers = mock::answers::greeting_author();
    answers.push(mock::answers::greeting_leaf_loud());
    answers.push(mock::answers::greeting_unready());
    answers.extend(mock::answers::greeting_author());
    let session = Session::scripted("mock", answers);
    let target = seed_target(session.root());
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship it.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    author(&session, &definition, &spec.wave, false).await.expect("first");
    let err = author(&session, &definition, &spec.wave, true).await.expect_err("force parks");
    assert!(err.to_string().contains("plan-author-stopped"), "{err}");

    // Topology verbs refuse over the parked partial tree even though a
    // historical reconcile fact exists.
    let gaps_err =
        run::<plan::handlers::Gaps, _, _>(session.provider(), plan::handlers::GapsInput::default())
            .await
            .expect_err("gaps refuses on a parked home");
    assert!(gaps_err.to_string().contains("plan-author-incomplete"), "{gaps_err}");

    let body = author(&session, &definition, &spec.wave, false).await.expect("resume completes");
    assert_eq!(body.slices, ["greeting"]);
    session.model().assert_exhausted();
}

// P7 regression: a changed revision spec on the same repository must
// re-resolve — the moved-ref re-anchor (re-pinning a moved branch to
// the recorded commit) applies only when the re-author binds the same
// reviewed handoff, never across a changed definition.
#[tokio::test]
async fn changed_revision_repins() {
    let mut answers = mock::answers::greeting_author();
    answers.extend(mock::answers::greeting_author());
    let session = Session::scripted("mock", answers);

    // A local Git origin with two branches: `main` at commit A, `dev`
    // one commit ahead at commit B. Git-shaped locators route through
    // the trees seam and settle exact SHAs.
    let origin = session.root().join("origin");
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&origin)
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(args)
            .output()
            .expect("git runs");
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    };
    std::fs::create_dir_all(origin.join(".emery")).expect("origin .emery");
    std::fs::write(origin.join(".emery/project.yaml"), "name: app\nadapter: omnia\nrules: {}\n")
        .expect("project.yaml");
    std::fs::write(origin.join("README.md"), "v1\n").expect("readme");
    git(&["init", "-q", "-b", "main"]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "a"]);
    git(&["checkout", "-qb", "dev"]);
    std::fs::write(origin.join("README.md"), "v2\n").expect("readme v2");
    git(&["add", "-A"]);
    git(&["commit", "-qm", "b"]);
    git(&["checkout", "-q", "main"]);

    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship it.");
    spec.targets[0].locator = format!("{}@main", origin.display());
    mint(&definition, &spec).expect("mint");
    author(&session, &definition, &spec.wave, false).await.expect("first");

    let layout = Layout::new(session.root());
    let mut plan = Plan::load(&layout.plan_path()).expect("plan");
    let first = plan.targets.get("app").expect("app").clone();
    // The vulnerable shape: no bound definition identity, so a changed
    // handoff reaches target binding with the old row still offered.
    plan.definition = None;
    plan.save(&layout.plan_path()).expect("save");

    // Same repository, different branch: the old SHA must not be
    // offered as a moved-ref re-anchor against the new revision spec.
    let definition2 = session.root().join("definition-v2");
    let mut spec2 = Spec::degenerate("Ship it.");
    spec2.targets[0].locator = format!("{}@dev", origin.display());
    mint(&definition2, &spec2).expect("mint v2");
    author(&session, &definition2, &spec2.wave, true).await.expect("force");

    let plan = Plan::load(&layout.plan_path()).expect("plan v2");
    let row = plan.targets.get("app").expect("app");
    assert_ne!(row.locator, first.locator, "the dev tip settles a new exact locator");
    assert_ne!(row.cid, first.cid, "a changed revision spec must never keep the stale pin");
}

// P7 regression: a changed locator under a stable target id must never
// reuse the previously bound CID — reuse is locator-keyed, not id-keyed.
#[tokio::test]
async fn changed_locator_rebinds() {
    let mut answers = mock::answers::greeting_author();
    answers.extend(mock::answers::greeting_author());
    let session = Session::scripted("mock", answers);
    let target = seed_target(session.root());
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship it.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    author(&session, &definition, &spec.wave, false).await.expect("first");

    let layout = Layout::new(session.root());
    let mut plan = Plan::load(&layout.plan_path()).expect("plan");
    let first = plan.targets.get("app").expect("app").cid.clone();
    // A plan bound before wave binding carries no definition identity;
    // that is the shape where a changed handoff reaches target binding.
    plan.definition = None;
    plan.save(&layout.plan_path()).expect("save");

    // Same target id, different locator, different tree content.
    let moved = session.root().join("target-app-v2");
    std::fs::create_dir_all(moved.join(".emery")).expect("moved .emery");
    std::fs::write(moved.join(".emery/project.yaml"), "name: app\nadapter: omnia\nrules: {}\n")
        .expect("project.yaml");
    std::fs::write(moved.join("README.md"), "v2\n").expect("readme");
    let definition2 = session.root().join("definition-v2");
    let mut spec2 = Spec::degenerate("Ship it.");
    spec2.targets[0].locator = moved.display().to_string();
    mint(&definition2, &spec2).expect("mint v2");
    author(&session, &definition2, &spec2.wave, true).await.expect("force");

    let plan = Plan::load(&layout.plan_path()).expect("plan v2");
    let row = plan.targets.get("app").expect("app");
    assert_eq!(row.locator, moved.display().to_string());
    assert_ne!(row.cid, first, "changed locator must never reuse the old target CID");
}
