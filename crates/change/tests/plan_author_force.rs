//! `plan author --force` rebinds the same reviewed handoff.

use change::plan;
use mock::definition::{Spec, mint};
use mock::invoke::run;
use mock::session::Session;

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
