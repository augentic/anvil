//! Wave-binding `plan author`: discovery.yaml, imports, --force, AC1/AC4.

use change::plan;
use mock::definition::{Spec, mint};
use mock::invoke::run;
use mock::session::Session;
use project::adapter::catalog::INTENT;
use project::config::Layout;
use project::definition::Home;
use project::plan::{Discovery, Plan};

fn seed_target(root: &std::path::Path) -> std::path::PathBuf {
    let target = root.join("target-app");
    std::fs::create_dir_all(target.join(".emery")).expect("target .emery");
    std::fs::write(target.join(".emery/project.yaml"), "name: app\nadapter: omnia\nrules: {}\n")
        .expect("project.yaml");
    target
}

fn mint_bindable(root: &std::path::Path, intent: &str) -> (std::path::PathBuf, String) {
    let target = seed_target(root);
    let definition = root.join("definition");
    let mut spec = Spec::degenerate(intent);
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    (definition, spec.wave)
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

fn code(err: &impl std::fmt::Display) -> String {
    err.to_string()
}

#[tokio::test]
async fn binds_degenerate_intent() {
    let session = Session::scripted("mock", Vec::new());
    let (from, wave) = mint_bindable(session.root(), "Ship the greeting.");
    let body = author(&session, &from, &wave, false).await.expect("bind");
    assert_eq!(body.pending, "decomposition");
    assert!(body.sources.iter().any(|key| key == INTENT), "{:?}", body.sources);
    assert!(body.targets.iter().any(|key| key == "app"), "{:?}", body.targets);

    let layout = Layout::new(session.root());
    let discovery = Discovery::load(&layout.discovery_yaml_path()).expect("discovery.yaml");
    let intent = discovery.sources.get(INTENT).expect("intent row");
    assert_eq!(intent.value.as_deref(), Some("Ship the greeting."));
    assert!(intent.locator.is_none());
    assert!(intent.cid.is_none());

    let plan = Plan::load(&layout.plan_path()).expect("plan.yaml");
    assert_eq!(plan.discovery_digest.as_ref(), Some(&discovery.digest().expect("digest")));
    assert!(plan.entries.is_empty(), "decomposition pending: slices stay empty");

    let reviewed = project::definition::resolve(&from, &wave).expect("resolve");
    let handoff =
        std::fs::read(layout.import_handoff_path(&reviewed.digest)).expect("handoff import");
    let original =
        std::fs::read(Home::new(&from).handoff_path(&reviewed.digest)).expect("handoff src");
    assert_eq!(handoff, original, "byte-identical handoff import");
}

#[tokio::test]
async fn empty_dir_discovery() {
    let session = Session::scripted("mock", Vec::new());
    let (from, wave) = mint_bindable(session.root(), "Greenfield intent.");
    assert!(!session.root().join("target-app/.git").exists());
    author(&session, &from, &wave, false).await.expect("bind");
    assert!(Layout::new(session.root()).discovery_yaml_path().is_file());
}

#[tokio::test]
async fn missing_handoff_blocks() {
    let session = Session::scripted("mock", Vec::new());
    let err = author(&session, &session.root().join("missing"), "deliver", false)
        .await
        .expect_err("missing definition");
    assert!(code(&err).contains("definition") || code(&err).contains("handoff"), "{err}");
}

#[tokio::test]
async fn intent_locator_refused() {
    let session = Session::scripted("mock", Vec::new());
    let target = seed_target(session.root());
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("inline");
    spec.targets[0].locator = target.display().to_string();
    spec.scopes[0].locator = Some("/tmp/intent".into());
    let err = mint(&definition, &spec).expect_err("locator");
    assert!(
        code(&err).contains("definition-intent-form")
            || code(&err).contains("source-intent-locator"),
        "{err}"
    );
}

#[tokio::test]
async fn force_same_handoff() {
    let session = Session::scripted("mock", Vec::new());
    let (from, wave) = mint_bindable(session.root(), "Ship it.");
    author(&session, &from, &wave, false).await.expect("first");
    author(&session, &from, &wave, true).await.expect("force");
}

#[tokio::test]
async fn force_changed_handoff() {
    let session = Session::scripted("mock", Vec::new());
    let (from, wave) = mint_bindable(session.root(), "First.");
    author(&session, &from, &wave, false).await.expect("first");
    let (from2, wave2) = mint_bindable(&session.root().join("other"), "Second.");
    let err = author(&session, &from2, &wave2, true).await.expect_err("changed");
    assert!(code(&err).contains("plan-author-handoff-changed"), "{err}");
}

#[test]
fn required_slice_target() {
    let yaml = "name: demo\nslices:\n  - name: greeting\n";
    let err = serde_saphyr::from_str::<Plan>(yaml).expect_err("target required");
    assert!(err.to_string().contains("target"), "{err}");
}

#[test]
fn leftover_project_field() {
    let yaml =
        "name: demo\nslices:\n  - name: greeting\n    target: default\n    project: default\n";
    let err = serde_saphyr::from_str::<Plan>(yaml).expect_err("unknown field");
    let text = err.to_string();
    assert!(text.contains("project") || text.contains("unknown"), "{text}");
}
