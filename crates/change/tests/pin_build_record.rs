//! RFC-86 Acceptance #4 / D25 / D27 — pin → build-record → wave-commit
//! fixture, plus pin-drift review signals (`slice-base-drifted` /
//! `slice-evidence-stale`).

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
use slice::Base;

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

    run::<slice::handlers::Refine, _, _>(
        session.provider(),
        slice::handlers::RefineInput {
            name: "login-flow".to_string(),
        },
    )
    .await
    .expect("refine");

    // Merge (and the plan-owned completion preflight) require a
    // projected in-progress entry — advance claims the slice.
    run::<plan::handlers::Advance, _, _>(session.provider(), plan::handlers::AdvanceInput {})
        .await
        .expect("advance claims login-flow");
}

fn journal_kinds(root: &std::path::Path) -> Vec<&'static str> {
    let events = read_union(Layout::new(root)).expect("union");
    events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::TargetWaveOpened { .. } => Some("target.wave.opened"),
            EventKind::TargetMergeWaveCommitted { .. } => Some("target.merge.wave-committed"),
            EventKind::TargetMergeWaveSucceeded { .. } => Some("target.merge.wave-succeeded"),
            EventKind::SliceCodeApplied { .. } => Some("slice.code.applied"),
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
async fn pin_build_record_wave_commit_and_apply() {
    let session = Session::scripted(
        "mock",
        vec![mock::answers::adversarial_grouping(), mock::answers::login_flow_synthesis()],
    );
    let root = session.root().to_path_buf();
    author_and_refine(&session).await;

    let layout = Layout::new(&root);
    let slice_dir = layout.slice_dir("login-flow");
    let base = Base::load(&slice_dir).expect("base.yaml after refine");
    assert!(
        base.target_base.as_str().starts_with("sha256:"),
        "target-base pin recorded: {}",
        base.target_base
    );
    assert_eq!(base.sources["docs"], value_cid("The docs source."));

    run::<slice::handlers::Build, _, _>(
        session.provider(),
        slice::handlers::BuildInput {
            name: "login-flow".to_string(),
        },
    )
    .await
    .expect("build from recorded pin");

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

    // Clean pins → validate PASSes with no pin-drift reviews.
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
        !clean_reviews.iter().any(|id| id == "slice-base-drifted" || id == "slice-evidence-stale"),
        "no pin drift on clean pins: {clean_reviews:?}"
    );

    run::<slice::handlers::MergeRun, _, _>(
        session.provider(),
        slice::handlers::MergeRunInput {
            name: "login-flow".to_string(),
            allow_composition_replace: false,
            preview: false,
            conflict_check: false,
        },
    )
    .await
    .expect("merge wave-commits");

    let kinds = journal_kinds(&root);
    assert!(kinds.contains(&"target.merge.wave-committed"), "{kinds:?}");
    assert!(kinds.contains(&"target.merge.wave-succeeded"), "{kinds:?}");
    assert!(kinds.contains(&"slice.code.applied"), "interim apply still runs: {kinds:?}");

    let events = read_union(layout).expect("union");
    let maps = events.iter().find_map(|event| match &event.kind {
        EventKind::TargetMergeWaveCommitted { identity_maps, .. } => Some(identity_maps.clone()),
        _ => None,
    });
    let maps = maps.expect("identity maps on wave-committed");
    assert!(!maps.is_empty(), "identity maps recorded: {maps:?}");
    assert!(maps.iter().any(|m| m.local == "REQ-001"), "local id mapped: {maps:?}");

    // Merged projection: baseline carries finalized ids; no leftover
    // patch.yaml authority under the archived slice.
    assert!(root.join(".emery/specs/auth/spec.md").is_file());
    let archive = fs::read_dir(root.join(".emery/archive"))
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
async fn validate_reports_baseline_and_source_pin_drift() {
    let session = Session::scripted(
        "mock",
        vec![mock::answers::adversarial_grouping(), mock::answers::login_flow_synthesis()],
    );
    let root = session.root().to_path_buf();
    author_and_refine(&session).await;

    // Move the baseline tree after refine pinned empty-cid.
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
    .expect("pin drift is review — validate still PASSes");
    let ids = review_ids(&body);
    assert!(ids.iter().any(|id| id == "slice-base-drifted"), "{ids:?}");

    // Restore baseline pin agreement, then drift a bound source value.
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
    let ids = review_ids(&body);
    assert!(ids.iter().any(|id| id == "slice-evidence-stale"), "{ids:?}");
    assert!(!ids.iter().any(|id| id == "slice-base-drifted"), "baseline restored: {ids:?}");
}
