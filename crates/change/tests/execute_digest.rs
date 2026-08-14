//! RFC-88 D8: execute-start digest chain, epoch transitivity, and
//! live-claim-without-epoch / stale-definition refusals.

mod support;

use std::fs;

use change::plan::handlers::{Author, AuthorInput, Execute, ExecuteInput};
use mock::definition::{Spec, mint};
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::handler::Anchor;
use project::journal::EventKind;
use project::plan::epoch::{EpochFreshness, freshness};
use project::plan::{Plan, closed_plan, collect_events};
use project::snapshot::SnapshotId;

fn cid(digit: u8) -> SnapshotId {
    SnapshotId::from_digest(&format!("{digit:x}").repeat(64))
}

fn seed_target(root: &std::path::Path, name: &str) -> std::path::PathBuf {
    let target = root.join(name);
    fs::create_dir_all(target.join(".emery")).expect("target .emery");
    fs::write(
        target.join(".emery/project.yaml"),
        format!("name: {name}\nadapter: omnia\nrules: {{}}\n"),
    )
    .expect("project.yaml");
    target
}

fn mint_reviewed(root: &std::path::Path, intent: &str) -> (std::path::PathBuf, String) {
    let target = seed_target(root, "target-app");
    let definition = root.join("definition");
    let mut spec = Spec::degenerate(intent);
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    (definition, spec.wave)
}

async fn author_plan(session: &Session, from: &std::path::Path, wave: &str) {
    run::<Author, _, _>(
        session.provider(),
        AuthorInput {
            name: "demo".to_string(),
            from: from.to_path_buf(),
            wave: wave.to_string(),
            force: false,
        },
    )
    .await
    .expect("author");
}

#[tokio::test]
async fn claim_without_epoch() {
    let session = Session::scripted("mock", vec![mock::answers::greeting_synthesis()]);
    support::write_greeting_plan(session.root());
    support::refine_plan(&session).await;

    let claimed = support::advance(&session);
    assert!(claimed.advanced.is_some() || claimed.active.is_some(), "{claimed:?}");

    let err = support::build(&session, "greeting").await.expect_err("build without epoch");
    assert!(err.to_string().contains("plan-epoch-required"), "{err}");
}

#[tokio::test]
async fn stale_definition() {
    let session = Session::scripted("mock", mock::answers::greeting_author());
    let (definition, wave) = mint_reviewed(session.root(), "Ship the greeting.");
    author_plan(&session, &definition, &wave).await;

    fs::remove_dir_all(definition.join("handoffs")).expect("clear current handoffs");
    let mut spec = Spec::degenerate("Ship a different greeting.");
    spec.targets[0].locator = session.root().join("target-app").display().to_string();
    mint(&definition, &spec).expect("remint");

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("stale definition");
    assert!(err.to_string().contains("plan-definition-stale"), "{err}");
}

#[tokio::test]
async fn digest_stales_epoch() {
    let session = Session::scripted("mock", mock::answers::greeting_author());
    let (definition, wave) = mint_reviewed(session.root(), "Ship the greeting.");
    author_plan(&session, &definition, &wave).await;
    support::stage_manifest(session.root(), "greeting");
    drop(run::<Execute, _, _>(session.provider(), ExecuteInput::default()).await);

    let layout = Layout::new(session.root());
    let original = Plan::load(&layout.plan_path()).expect("plan");
    let events = collect_events(layout).expect("events");
    assert!(
        events.iter().any(|event| matches!(event.kind, EventKind::PlanExecuteStarted { .. })),
        "execute start journals an epoch"
    );
    assert!(
        matches!(
            freshness(layout, &original, &events).expect("freshness"),
            EpochFreshness::Fresh { .. }
        ),
        "authored plan is fresh under the epoch"
    );

    let mut drifted = original;
    drifted.discovery_digest = Some(cid(0xa));
    drifted.leads_digest = Some(cid(0xb));
    drifted.decomposition_digest = Some(cid(0xc));
    if let Some(def) = &mut drifted.definition {
        def.handoff_digest = cid(0xd);
        def.system_model_digest = cid(0xe);
        def.migration_plan_digest = cid(0xf);
    }
    for row in drifted.targets.values_mut() {
        if let Some(profile) = &mut row.model_capability_profile {
            profile.digest = cid(0x1);
        }
    }
    drifted.save(&layout.plan_path()).expect("save drifted plan");

    let events = collect_events(layout).expect("events");
    let live = Plan::load(&layout.plan_path()).expect("reloaded");
    assert!(
        matches!(
            freshness(layout, &live, &events).expect("freshness"),
            EpochFreshness::Stale { .. }
        ),
        "each bound digest rides plan.yaml; rewriting them stales the epoch"
    );
}

#[tokio::test]
async fn live_file_drift() {
    let session = Session::scripted("mock", mock::answers::greeting_author());
    let (definition, wave) = mint_reviewed(session.root(), "Ship the greeting.");
    author_plan(&session, &definition, &wave).await;

    let paths = session.provider().paths();
    let layout = paths.layout();
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    closed_plan(paths, &plan).expect("authored chain is closed");

    let discovery = layout.discovery_yaml_path();
    let text = fs::read_to_string(&discovery).expect("discovery.yaml");
    fs::write(&discovery, text.replacen("target-app", "target-drift", 1))
        .expect("mutate discovery");
    let err = closed_plan(paths, &plan).expect_err("discovery drift");
    assert!(err.to_string().contains("plan-discovery-mismatch"), "{err}");
    fs::write(&discovery, text).expect("restore discovery");

    let leads = layout.leads_path();
    let text = fs::read_to_string(&leads).expect("leads.md");
    fs::write(&leads, text.replacen("synopsis:", "synopsis: drifted", 1)).expect("mutate leads");
    let err = closed_plan(paths, &plan).expect_err("leads drift");
    assert!(err.to_string().contains("plan-leads-mismatch"), "{err}");
    fs::write(&leads, text).expect("restore leads");

    let tree = layout.decomposition_path();
    let text = fs::read_to_string(&tree).expect("decomposition.yaml");
    fs::write(&tree, text.replacen("greeting", "drifted", 1)).expect("mutate tree");
    let err = closed_plan(paths, &plan).expect_err("decomposition drift");
    assert!(err.to_string().contains("plan-decomposition-mismatch"), "{err}");
}
