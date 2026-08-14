//! Wave-binding `plan author`: discovery.yaml, imports, --force, AC1/AC4.

use change::plan;
use mock::definition::{Spec, load_reviewed, mint};
use mock::invoke::run;
use mock::session::Session;
use project::adapter::catalog::INTENT;
use project::config::Layout;
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
    let session = Session::scripted("mock", mock::answers::greeting_author());
    let (from, wave) = mint_bindable(session.root(), "Ship the greeting.");
    let body = author(&session, &from, &wave, false).await.expect("bind");
    assert!(body.slices.iter().any(|name| name == "greeting"), "{:?}", body.slices);
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
    assert_eq!(plan.entries.len(), 1, "degenerate root → leaf projects one slice");
    assert_eq!(plan.entries[0].name.as_str(), "greeting");
    assert_eq!(plan.entries[0].target, "app");
    assert!(plan.decomposition_digest.is_some());

    let catalog = artifacts::leads::Leads::load(&layout.leads_path()).expect("leads.md");
    assert_eq!(catalog.leads().len(), 1);
    assert_eq!(catalog.leads()[0].source, INTENT);
    assert_eq!(catalog.leads()[0].lead, INTENT);
    assert_eq!(catalog.leads()[0].synopsis, "Ship the greeting.");
    let leads_digest =
        project::snapshot::SnapshotId::from_digest(&catalog.digest_hex().expect("hex"));
    assert_eq!(plan.leads_digest.as_ref(), Some(&leads_digest));
    let retained = std::fs::read(layout.leads_revision_path(&leads_digest)).expect("retained");
    let current = std::fs::read(layout.leads_path()).expect("current");
    assert_eq!(retained, current, "retention copies exact leads.md bytes");

    let reviewed = load_reviewed(&from, &wave).expect("resolve");
    let handoff =
        std::fs::read(layout.import_handoff_path(&reviewed.digest)).expect("handoff import");
    let original =
        std::fs::read(from.join("handoffs").join(format!("{}.yaml", reviewed.digest.digest())))
            .expect("handoff src");
    assert_eq!(handoff, original, "byte-identical handoff import");

    let compiled = project::profile::Table::compiled();
    let bound = compiled.resolve().expect("compiled").reference().expect("reference");
    assert!(
        discovery.targets.values().all(|row| row.model_capability_profile.is_none()),
        "discovery.yaml does not carry profile pins"
    );
    let stamped = plan.targets.get("app").expect("app").model_capability_profile.as_ref();
    assert_eq!(stamped, Some(&bound));
}

#[tokio::test]
async fn empty_dir_discovery() {
    let session = Session::scripted("mock", mock::answers::greeting_author());
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
    let text = code(&err);
    assert!(
        text.contains("scope") || text.contains("handoff") || text.contains("definition"),
        "{err}"
    );
}

#[tokio::test]
async fn intent_locator_refused() {
    let session = Session::scripted("mock", mock::answers::greeting_author());
    let target = seed_target(session.root());
    let definition = session.root().join("definition");
    let intent_file = definition.join("brief.txt");
    std::fs::create_dir_all(&definition).expect("definition");
    std::fs::write(&intent_file, "inline").expect("intent file");
    let mut spec = Spec::degenerate("inline");
    spec.targets[0].locator = target.display().to_string();
    spec.scopes[0].location = intent_file.display().to_string();
    spec.scopes[0].value = Some("inline".into());
    mint(&definition, &spec).expect("mint");
    author(&session, &definition, &spec.wave, false).await.expect("bind");
    let discovery =
        Discovery::load(&Layout::new(session.root()).discovery_yaml_path()).expect("discovery");
    let intent = discovery.sources.get(INTENT).expect("intent");
    assert_eq!(intent.value.as_deref(), Some("inline"));
    assert!(intent.locator.is_none(), "intent must not record a locator");
    assert!(intent.cid.is_none(), "intent must not record a delivery CID");
}

#[tokio::test]
async fn force_same_handoff() {
    let mut answers = mock::answers::greeting_author();
    answers.extend(mock::answers::greeting_author());
    let session = Session::scripted("mock", answers);
    let (from, wave) = mint_bindable(session.root(), "Ship it.");
    author(&session, &from, &wave, false).await.expect("first");
    author(&session, &from, &wave, true).await.expect("force");
}

#[tokio::test]
async fn force_changed_handoff() {
    let session = Session::scripted("mock", mock::answers::greeting_author());
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

#[tokio::test]
async fn host_table_stamps() {
    let mut other = project::profile::Profile::frontier_v1();
    other.weights.coupling = 5;
    let table = project::profile::Table::new(std::collections::BTreeMap::from([(
        project::profile::FRONTIER_LARGE.into(),
        other,
    )]))
    .expect("table");
    let override_ref = table.resolve().expect("resolve").reference().expect("reference");
    let compiled = project::profile::Table::compiled()
        .resolve()
        .expect("compiled")
        .reference()
        .expect("reference");
    assert_ne!(override_ref.digest, compiled.digest);

    let session = Session::scripted("mock", mock::answers::greeting_author()).with_profiles(table);
    let (from, wave) = mint_bindable(session.root(), "Ship the greeting.");
    author(&session, &from, &wave, false).await.expect("bind");
    let plan = Plan::load(&Layout::new(session.root()).plan_path()).expect("plan");
    assert_eq!(
        plan.targets.get("app").expect("app").model_capability_profile.as_ref(),
        Some(&override_ref)
    );
}
