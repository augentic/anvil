//! Cut A4 (RFC-96 D5): read-heavy fan-out over the bounded pool.
//! Cap-one and cap-four produce byte-identical refinement manifests
//! and lead catalogs even when scripted completions stagger out of
//! dispatch order, and a failed focused survey cancels the whole
//! boundary-escalation proposal — nothing persists.

mod support;

use mock::session::Session;
use project::handler::Anchor as _;
use project::plan::{Plan, Proposal, SourceBinding};
use project::snapshot::SnapshotId;
use serde_json::json;

/// The greeting synthesis answer re-slotted for `slice`, with its
/// claim anchored on the `source` binding key.
fn synthesis_for(slice: &str, source: &str) -> String {
    let mut answer: serde_json::Value =
        serde_json::from_str(&mock::answers::greeting_synthesis()).expect("answer parses");
    answer["slice"] = json!(slice);
    answer["model"]["requirements"][0]["domain"] = json!(slice);
    answer["model"]["requirements"][0]["claims"][0]["source"] = json!(source);
    answer["artifacts"]["specs"][0]["domain"] = json!(slice);
    let proposal = answer["artifacts"]["proposal"].as_str().expect("proposal");
    answer["artifacts"]["proposal"] = json!(proposal.replace(
        "- greeting — the affected surface",
        &format!("- {slice} — the affected surface")
    ));
    serde_json::to_string(&answer).expect("answer serialises")
}

const LEAVES: [&str; 3] = ["alpha", "beta", "gamma"];

/// Three independent leaves, each on its own value-backed source.
fn triple_plan(root: &std::path::Path) {
    support::write_plan_fixture(
        root,
        "demo",
        &[
            ("s-alpha", "mock", "The greeting service."),
            ("s-beta", "mock", "The greeting service."),
            ("s-gamma", "mock", "The greeting service."),
        ],
        &[
            ("alpha", "s-alpha", "greeting"),
            ("beta", "s-beta", "greeting"),
            ("gamma", "s-gamma", "greeting"),
        ],
    );
}

#[expect(unsafe_code, reason = "EMERY_POOL is the launcher cap seam; nextest isolates the process")]
fn set_cap(cap: &str) {
    // SAFETY: nextest runs each test in its own process, and the env
    // write happens before any pool dispatch reads the cap.
    unsafe { std::env::set_var("EMERY_POOL", cap) };
}

/// Drain the triple plan at one cap and return the per-leaf manifest
/// digests plus the final catalog bytes.
async fn drain(cap: &str, yields: Vec<usize>) -> ([SnapshotId; 3], Vec<u8>) {
    set_cap(cap);
    let session = Session::scripted_staggered(
        "mock",
        LEAVES.iter().map(|leaf| synthesis_for(leaf, &format!("s-{leaf}"))).collect(),
        yields,
    );
    let root = session.root().to_path_buf();
    triple_plan(&root);

    let drained = support::refine_plan(&session).await;
    assert_eq!(drained.status, "refined");
    // Outcomes join in topological order — never completion order.
    assert_eq!(drained.refined, LEAVES);
    session.model().assert_exhausted();

    let digests = LEAVES.map(|leaf| support::manifest_digest(&root, leaf));
    let catalog = std::fs::read(support::fixture_layout(&root).leads_path()).expect("leads.md");
    (digests, catalog)
}

// Cap-one is the serial reference: cap-four with staggered scripted
// completions (alpha settles last, beta first) must produce
// byte-identical manifests and an identical catalog.
#[tokio::test]
async fn cap_equivalence() {
    let (serial, serial_catalog) = drain("1", Vec::new()).await;
    let (concurrent, concurrent_catalog) = drain("4", vec![4, 0, 2]).await;
    assert_eq!(serial, concurrent, "cap-one and cap-four mint identical manifests");
    assert_eq!(serial_catalog, concurrent_catalog, "identical catalogs");
}

/// Run the unfocused survey fan-out over the adversarial two-source
/// plan and return the merged catalog bytes.
async fn survey_catalog(cap: &str) -> Vec<u8> {
    set_cap(cap);
    let session = Session::scripted("mock", Vec::new());
    let root = session.root().to_path_buf();
    support::write_adversarial_plan(&root);

    let provider = session.provider();
    let surveyed = change::orchestrate::survey_all(
        provider,
        provider,
        provider,
        provider.paths(),
        jiff::Timestamp::now(),
    )
    .await
    .expect("survey fan-out completes");
    let sources: Vec<&str> = surveyed.iter().map(|row| row.source.as_str()).collect();
    assert_eq!(sources, ["code", "docs"], "results join in plan order");

    std::fs::read(support::fixture_layout(&root).leads_path()).expect("leads.md")
}

// The survey fan-out merges lead sets serially in plan order, so the
// catalog is byte-identical at every cap.
#[tokio::test]
async fn survey_cap_equivalence() {
    let serial = survey_catalog("1").await;
    let concurrent = survey_catalog("4").await;
    assert_eq!(serial, concurrent, "cap-one and cap-four merge identical catalogs");
}

// A boundary escalation whose focused survey fails cancels the whole
// proposal assembly: the drain parks with the survey failure, no
// proposal persists, no park fact is journalled, and live planning
// artifacts are untouched.
#[tokio::test]
async fn escalation_cancels() {
    let mut answer: serde_json::Value =
        serde_json::from_str(&mock::answers::greeting_escalation()).expect("escalation parses");
    answer["affected"] = json!([{ "source": "docs", "lead": "greeting-tone" }]);
    let session = Session::scripted(
        "mock",
        vec![serde_json::to_string(&answer).expect("escalation serialises")],
    );
    let root = session.root().to_path_buf();
    // The affected pair must be one of the leaf's bound terminals, so
    // `greeting` binds `docs / auth-flows` beside `main / greeting`.
    support::write_plan_fixture(
        &root,
        "demo",
        &[
            ("main", "mock", "The greeting service."),
            ("docs", "mock-fail-survey", "The docs source."),
        ],
        &[("greeting", "main", "greeting"), ("greeting", "docs", "greeting-tone")],
    );

    // Rebind `docs` locator-backed (focused resurvey skips value-backed
    // parents), with its tree snapshotted so the survey can bind a view.
    let docs_dir = root.join("docs-src");
    std::fs::create_dir_all(&docs_dir).expect("docs tree");
    std::fs::write(docs_dir.join("README.md"), "auth flows\n").expect("docs file");
    let cid = session.store().snapshot(&docs_dir).await.expect("docs cid");
    let layout = support::fixture_layout(&root);
    let mut plan = Plan::load(&layout.plan_path()).expect("plan.yaml");
    plan.sources.insert(
        "docs".into(),
        SourceBinding {
            adapter: project::adapter::catalog::Pin::emery(
                "mock-fail-survey",
                semver::Version::new(0, 0, 0),
            ),
            locator: Some(docs_dir.display().to_string()),
            value: None,
            cid: Some(cid),
        },
    );
    plan.save(&layout.plan_path()).expect("plan.yaml saved");

    let leads_bytes = std::fs::read(layout.leads_path()).expect("leads.md bytes");
    let plan_bytes = std::fs::read(layout.plan_path()).expect("plan.yaml bytes");

    let stopped = support::refine_slices(&session, &[])
        .await
        .expect_err("the failed focused survey parks the drain")
        .to_string();
    assert!(stopped.contains("plan-refine-stopped"), "{stopped}");
    assert!(stopped.contains("mock survey failure"), "{stopped}");

    let proposals = Proposal::load_all(layout).expect("proposals");
    assert!(proposals.is_empty(), "nothing persists on a cancelled escalation");
    let parked = project::journal::read_union(layout)
        .expect("journal union")
        .into_iter()
        .filter(|event| {
            matches!(event.kind, project::journal::EventKind::SliceRefinementParked { .. })
        })
        .count();
    assert_eq!(parked, 0, "no park fact without a persisted proposal");
    assert_eq!(std::fs::read(layout.leads_path()).expect("leads.md"), leads_bytes);
    assert_eq!(std::fs::read(layout.plan_path()).expect("plan.yaml"), plan_bytes);
    session.model().assert_exhausted();
}
