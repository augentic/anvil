//! RFC-86 Acceptance #4 — refinement-manifest → build-record →
//! wave-commit fixture, plus refinement-freshness review signals
//! (`slice-refinement-stale`, RFC-91).

mod support;

use std::fs;

use change::plan;
use diagnostics::DiagnosticKind;
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{EventKind, read_union};
use project::plan::{Plan, value_cid};
use project::slice::{LifecycleStatus, SliceMetadata};
use slice::refinement::Manifest;

async fn author_and_refine(session: &Session) {
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "auth".to_string(),
            sources: support::adversarial_bindings(),
            intent: None,
            force: false,
        },
    )
    .await
    .expect("author");

    support::refine(session, "login-flow").await.expect("refine");

    // Merge (and the plan-owned completion preflight) require a
    // projected in-progress entry — the advance step claims the slice.
    support::advance(session);
}

fn journal_kinds(root: &std::path::Path) -> Vec<&'static str> {
    let events = read_union(Layout::new(root)).expect("union");
    events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::TargetWaveOpened { .. } => Some("target.wave.opened"),
            EventKind::TargetMergeWaveCommitted { .. } => Some("target.merge.wave-committed"),
            EventKind::TargetMergeWaveSucceeded { .. } => Some("target.merge.wave-succeeded"),
            EventKind::SliceBuildSucceeded { .. } => Some("slice.build.succeeded"),
            _ => None,
        })
        .collect()
}

fn review_ids(body: &project::handler::ReportBody) -> Vec<String> {
    body.report()
        .findings
        .iter()
        .filter(|f| f.kind == DiagnosticKind::Review)
        .filter_map(|f| f.rule_id.clone())
        .collect()
}

#[tokio::test]
async fn pin_build_record_wave() {
    let session = Session::scripted(
        "mock",
        vec![mock::answers::adversarial_grouping(), mock::answers::login_flow_synthesis()],
    );
    let root = session.root().to_path_buf();
    author_and_refine(&session).await;

    let layout = Layout::new(&root);
    let slice_dir = layout.slice_dir("login-flow");
    let manifest = Manifest::load(&slice_dir).expect("refinement.yaml after refine");
    assert_eq!(manifest.inputs.sources["docs"], value_cid("The docs source."));
    assert!(!manifest.bundle.is_empty(), "manifest covers the output bundle");

    // Fresh manifest → validate PASSes with no freshness reviews.
    // (Checked before build: the mock target's granted `tasks.md`
    // stage write legitimately stales the manifest afterwards.)
    let clean = run::<slice::handlers::Validate, _, _>(
        session.provider(),
        slice::handlers::ValidateInput {
            name: "login-flow".to_string(),
        },
    )
    .await
    .expect("validate passes");
    let clean_reviews = review_ids(&clean);
    assert!(
        !clean_reviews
            .iter()
            .any(|id| id == "slice-refinement-missing" || id == "slice-refinement-stale"),
        "no freshness reviews on a fresh manifest: {clean_reviews:?}"
    );

    support::build(&session, "login-flow").await.expect("build over the covered manifest");

    assert!(
        project::build_record::BuildRecord::present(&slice_dir),
        "fact-substrate build record present"
    );
    assert!(
        !slice_dir.join("build/patch.yaml").exists(),
        "patch.yaml must not be build-outcome authority"
    );

    let meta = SliceMetadata::load(&slice_dir).expect("metadata");
    assert_eq!(
        LifecycleStatus::project(&slice_dir, &meta),
        LifecycleStatus::Built,
        "built projects from build record / completed_at — not patch.yaml"
    );

    let kinds = journal_kinds(&root);
    assert!(kinds.contains(&"target.wave.opened"), "{kinds:?}");
    assert!(kinds.contains(&"slice.build.succeeded"), "{kinds:?}");

    support::merge(&session, "login-flow").await.expect("merge wave-commits");

    let kinds = journal_kinds(&root);
    assert!(kinds.contains(&"target.merge.wave-committed"), "{kinds:?}");
    assert!(kinds.contains(&"target.merge.wave-succeeded"), "{kinds:?}");
    assert!(!kinds.contains(&"slice.code.applied"), "interim apply is gone: {kinds:?}");

    let events = read_union(layout).expect("union");
    let maps = events.iter().find_map(|event| match &event.kind {
        EventKind::TargetMergeWaveCommitted { identity_maps, .. } => Some(identity_maps.clone()),
        _ => None,
    });
    let maps = maps.expect("identity maps on wave-committed");
    assert!(!maps.is_empty(), "identity maps recorded: {maps:?}");
    assert!(maps.iter().any(|m| m.local == "REQ-001"), "local id mapped: {maps:?}");

    // Merged projection: accepted CID carries finalized ids; checkout is untouched.
    assert!(!root.join(".emery/specs/auth/spec.md").is_file());
    let tree = session.materialize_accepted("demo").await;
    assert!(tree.path().join(".emery/specs/auth/spec.md").is_file());
    let archive = fs::read_dir(root.join(".emery/change/archive"))
        .expect("archive")
        .map(|e| e.expect("entry").path())
        .find(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with("-login-flow"))
        })
        .expect("archived login-flow");
    assert!(
        !archive.join("build/patch.yaml").exists(),
        "archived patch.yaml must not be authority"
    );
    assert!(
        project::build_record::BuildRecord::present(&archive) || archive.join("builds").is_dir(),
        "archived fact-substrate build record retained"
    );
}

#[tokio::test]
async fn validate_staleness() {
    let session = Session::scripted(
        "mock",
        vec![mock::answers::adversarial_grouping(), mock::answers::login_flow_synthesis()],
    );
    let root = session.root().to_path_buf();
    author_and_refine(&session).await;

    // Move the baseline tree after refinement covered empty-cid.
    let specs = root.join(".emery/specs/auth");
    fs::create_dir_all(&specs).expect("baseline domain");
    fs::write(specs.join("spec.md"), "### Requirement: Drift bait\n\nID: REQ-001\n\nBody.\n")
        .expect("baseline move");

    let body = run::<slice::handlers::Validate, _, _>(
        session.provider(),
        slice::handlers::ValidateInput {
            name: "login-flow".to_string(),
        },
    )
    .await
    .expect("staleness is review — validate still PASSes");
    let ids = review_ids(&body);
    assert!(ids.iter().any(|id| id == "slice-refinement-stale"), "{ids:?}");

    // Restore baseline agreement, then drift a bound source value.
    fs::remove_dir_all(root.join(".emery/specs")).expect("clear drifted baseline");
    let plan_path = Layout::new(&root).plan_path();
    let mut plan = Plan::load(&plan_path).expect("plan");
    plan.sources.get_mut("docs").expect("docs").value = Some("The docs source — DRIFTED.".into());
    artifacts::atomic::yaml_write(&plan_path, &plan).expect("rewrite plan");

    let body = run::<slice::handlers::Validate, _, _>(
        session.provider(),
        slice::handlers::ValidateInput {
            name: "login-flow".to_string(),
        },
    )
    .await
    .expect("source drift is review");
    // Pin the *cause*, not just the rule id — the earlier baseline
    // drift already fired `slice-refinement-stale`.
    let details: Vec<_> = body
        .report()
        .findings
        .iter()
        .filter(|f| f.rule_id.as_deref() == Some("slice-refinement-stale"))
        .map(|f| match &f.evidence {
            diagnostics::FindingEvidence::Snippet { value } => value.clone(),
            other => format!("{other:?}"),
        })
        .collect();
    assert!(
        details.iter().any(|d| d.contains("source `docs`")),
        "staleness names the drifted source: {details:?}"
    );
}
