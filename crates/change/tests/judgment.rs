//! `plan author` dispatches partition and change-prose judgments.

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

#[tokio::test]
async fn author_runs_judgment() {
    let session = Session::scripted("mock", mock::answers::greeting_author());
    let target = seed_target(session.root());
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");

    let authored = run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            from: definition,
            wave: spec.wave,
            force: false,
        },
    )
    .await
    .expect("author succeeds with scripted partition and change answers");
    assert!(authored.slices.iter().any(|name| name == "greeting"), "{:?}", authored.slices);
    session.model().assert_exhausted();
}
