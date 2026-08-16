//! Multi-leaf refinement conformance (RFC-91 AC1/AC7): a dependency
//! chain drains in topological order with dependency pins bound, a
//! plan-local merge does not stale sibling manifests (D4), execute
//! re-entry after a merge assembles fresh coverage, and a dependent
//! re-refines against a merged predecessor's archived manifest (D3).

mod support;

use change::plan;
use mock::invoke::run;
use mock::session::Session;
use serde_json::json;

/// The greeting synthesis answer re-slotted for `slice`, claim
/// anchored on `source`; each slice owns its own spec domain so the
/// leaves merge into disjoint baselines.
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

fn chain_plan(root: &std::path::Path, slices: &[(&str, &str)]) {
    let sources: Vec<(&str, &str, &str)> =
        slices.iter().map(|(_, source)| (*source, "mock", "The greeting service.")).collect();
    let entries: Vec<(&str, &str, &str)> =
        slices.iter().map(|(name, source)| (*name, *source, "greeting")).collect();
    support::write_plan_fixture(root, "demo", &sources, &entries);
}

async fn depend(session: &Session, slice: &str, on: &str) {
    run::<plan::handlers::Amend, _, _>(
        session.provider(),
        plan::handlers::AmendInput {
            name: Some(slice.to_string()),
            proposal: None,
            depends_on: Some(vec![on.to_string()]),
            sources: None,
            add_source: Vec::new(),
            remove_source: Vec::new(),
            divergence: None,
            description: None,
            context: None,
            authority_override: Vec::new(),
            clear_authority_override: Vec::new(),
            clear_authority_overrides: false,
            allow_composition_replace: None,
        },
    )
    .await
    .expect("amend depends-on");
}

async fn execute(
    session: &Session,
) -> Result<plan::handlers::ExecuteBody, project::handler::Error> {
    run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
}

fn manifest(root: &std::path::Path, slice: &str) -> slice::refinement::Manifest {
    slice::refinement::Manifest::load(&project::config::Layout::new(root).slice_dir(slice))
        .expect("refinement manifest loads")
}

// (AC1) A three-leaf dependency chain alpha → beta → gamma: the drain
// refines in topological order binding each dependent's predecessor
// pin, then execute drains build + merge for all three — after alpha
// and beta merge (archiving their trees and moving the baseline),
// gamma still projects fresh and builds (D3 + D4 via the status
// projection's freshness recompute mid-loop).
#[tokio::test]
async fn chain_drains() {
    let session = Session::scripted(
        "mock",
        vec![
            synthesis_for("alpha", "main"),
            synthesis_for("beta", "aux"),
            synthesis_for("gamma", "ter"),
        ],
    );
    let root = session.root().to_path_buf();

    chain_plan(&root, &[("alpha", "main"), ("beta", "aux"), ("gamma", "ter")]);
    depend(&session, "beta", "alpha").await;
    depend(&session, "gamma", "beta").await;

    let drained = support::refine_plan(&session).await;
    assert_eq!(drained.refined, ["alpha", "beta", "gamma"], "topological drain order");

    // Each dependent's manifest pins its predecessor's live digest.
    let alpha = support::manifest_digest(&root, "alpha");
    let beta = support::manifest_digest(&root, "beta");
    let beta_deps = manifest(&root, "beta").inputs.dependencies;
    assert_eq!(beta_deps.len(), 1);
    assert_eq!(beta_deps[0].slice, "alpha");
    assert_eq!(beta_deps[0].refinement, alpha);
    let gamma_deps = manifest(&root, "gamma").inputs.dependencies;
    assert_eq!(gamma_deps.len(), 1);
    assert_eq!(gamma_deps[0].slice, "beta");
    assert_eq!(gamma_deps[0].refinement, beta);

    let executed = execute(&session).await.expect("execute drains the chain");
    assert_eq!(executed.status, "drained");
    let ran: Vec<(&str, change::LoopStep)> =
        executed.phases.iter().map(|phase| (phase.slice.as_str(), phase.step)).collect();
    assert_eq!(
        ran,
        [
            ("alpha", change::LoopStep::Build),
            ("alpha", change::LoopStep::Merge),
            ("beta", change::LoopStep::Build),
            ("beta", change::LoopStep::Merge),
            ("gamma", change::LoopStep::Build),
            ("gamma", change::LoopStep::Merge),
        ]
    );
    session.model().assert_exhausted();

    // Dependent waves open against the prior accepted CID, not a fresh freeze.
    let layout = project::config::Layout::new(&root);
    let events = project::journal::read_union(layout).expect("union");
    let commits: Vec<_> = events
        .iter()
        .filter_map(|event| match &event.kind {
            project::journal::EventKind::TargetMergeWaveCommitted {
                base,
                result,
                members,
                ..
            } => Some((members.clone(), base.clone(), result.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(commits.len(), 3, "one commit per leaf: {commits:?}");
    assert_eq!(
        commits[0].0.iter().map(project::name::SliceName::as_str).collect::<Vec<_>>(),
        ["alpha"]
    );
    assert_eq!(
        commits[1].0.iter().map(project::name::SliceName::as_str).collect::<Vec<_>>(),
        ["beta"]
    );
    assert_eq!(commits[1].1, commits[0].2, "beta's wave opened against alpha's accepted result");
    assert_eq!(commits[2].1, commits[1].2, "gamma's wave opened against beta's accepted result");
}

// (AC7) Execute re-entry after a plan-local merge: alpha merges and
// archives, execute stops on beta's failed build, and the re-run
// assembles fresh coverage — alpha is done and excluded, and beta's
// manifest is still fresh because checkout `.emery/specs/` no longer
// moves on merge (recorded refine pins still match live). Without
// that, the re-run would fail `plan-refinement-required`. Beta
// depends on alpha so the pair serializes into two waves instead of
// one frozen group (RFC-96 D7).
#[tokio::test]
async fn merge_stop_reentry() {
    let session = Session::scripted(
        "mock",
        vec![synthesis_for("alpha", "main"), synthesis_for("beta", "aux")],
    );
    let root = session.root().to_path_buf();

    chain_plan(&root, &[("alpha", "main"), ("beta", "aux")]);
    depend(&session, "beta", "alpha").await;
    support::refine_plan(&session).await;
    let beta_digest = support::manifest_digest(&root, "beta");

    // Run 1: alpha builds, then its merge preflight fails — the stop
    // parks with alpha built and not merged.
    support::marker(&root, mock::behaviour::PREFLIGHT_FAIL);
    let stopped = execute(&session).await.expect_err("preflight failure stops").to_string();
    assert!(stopped.contains("plan-execute-stopped"), "{stopped}");
    assert!(stopped.contains("merge-conflict"), "{stopped}");
    assert!(root.join(".emery/change/slices/alpha").is_dir(), "alpha not merged yet");

    // Run 2: alpha merges and archives (the baseline moves), then
    // beta's build fails on the marker — the stop leaves beta unbuilt
    // with its pre-merge manifest.
    std::fs::remove_file(root.join(mock::behaviour::PREFLIGHT_FAIL)).expect("rm marker");
    support::marker(&root, mock::behaviour::FAIL_BUILD_MARKER);
    let stopped = execute(&session).await.expect_err("build failure stops").to_string();
    assert!(stopped.contains("build-failed"), "{stopped}");
    assert!(!root.join(".emery/change/slices/alpha").exists(), "alpha merged and archived");

    // Run 3: fresh coverage over the moved baseline — alpha is done
    // and excluded, beta's untouched manifest still projects fresh —
    // and the loop drains beta.
    std::fs::remove_file(root.join(mock::behaviour::FAIL_BUILD_MARKER)).expect("rm marker");
    let executed = execute(&session).await.expect("re-entry drains");
    assert_eq!(executed.status, "drained");
    // Beta merged and archived during the drain; its archived manifest
    // still carries the pre-run digest — no re-refine happened.
    let layout = project::config::Layout::new(&root);
    let archived = slice::refinement::predecessor_digest(layout, "beta")
        .expect("archive lookup")
        .expect("archived manifest present");
    assert_eq!(archived, beta_digest, "beta drained without a re-refine");
    session.model().assert_exhausted();
}

// (D3) A dependent re-refines after its predecessor merged: amending
// the dependent post-merge stales its manifest through the entry
// projection, and the re-run drain pins the predecessor's *archived*
// manifest digest instead of stopping with "no refinement manifest".
#[tokio::test]
async fn dependent_rerefine() {
    let session = Session::scripted(
        "mock",
        vec![
            synthesis_for("alpha", "main"),
            synthesis_for("beta", "aux"),
            synthesis_for("beta", "aux"),
        ],
    );
    let root = session.root().to_path_buf();

    chain_plan(&root, &[("alpha", "main"), ("beta", "aux")]);
    depend(&session, "beta", "alpha").await;
    support::refine_plan(&session).await;
    let alpha_digest = support::manifest_digest(&root, "alpha");

    // Stage alpha merged + beta unbuilt via the two-marker flow: stop
    // at alpha's merge preflight, then let alpha merge and archive
    // while beta's build fails.
    support::marker(&root, mock::behaviour::PREFLIGHT_FAIL);
    let stopped = execute(&session).await.expect_err("preflight failure stops").to_string();
    assert!(stopped.contains("merge-conflict"), "{stopped}");
    std::fs::remove_file(root.join(mock::behaviour::PREFLIGHT_FAIL)).expect("rm marker");
    support::marker(&root, mock::behaviour::FAIL_BUILD_MARKER);
    let stopped = execute(&session).await.expect_err("beta build fails").to_string();
    assert!(stopped.contains("build-failed"), "{stopped}");
    std::fs::remove_file(root.join(mock::behaviour::FAIL_BUILD_MARKER)).expect("rm marker");
    assert!(!root.join(".emery/change/slices/alpha").exists(), "alpha merged and archived");

    // Amend the dependent post-merge: the entry projection drifts and
    // the manifest goes stale.
    run::<plan::handlers::Amend, _, _>(
        session.provider(),
        plan::handlers::AmendInput {
            name: Some("beta".to_string()),
            proposal: None,
            depends_on: None,
            sources: None,
            add_source: Vec::new(),
            remove_source: Vec::new(),
            divergence: None,
            description: Some("Sharper slice description.".to_string()),
            context: None,
            authority_override: Vec::new(),
            clear_authority_override: Vec::new(),
            clear_authority_overrides: false,
            allow_composition_replace: None,
        },
    )
    .await
    .expect("amend the dependent");

    // The drain re-refines the dependent against the archived
    // predecessor manifest — no "predecessor has no refinement
    // manifest" stop.
    let redrained = support::refine_plan(&session).await;
    assert_eq!(redrained.refined, ["beta"], "only the stale dependent re-refines");
    let beta_deps = manifest(&root, "beta").inputs.dependencies;
    assert_eq!(beta_deps.len(), 1);
    assert_eq!(beta_deps[0].slice, "alpha");
    assert_eq!(
        beta_deps[0].refinement, alpha_digest,
        "the archived manifest digest is the dependency pin"
    );

    // The re-refined dependent drains through execute.
    let executed = execute(&session).await.expect("execute drains the dependent");
    assert_eq!(executed.status, "drained");
    session.model().assert_exhausted();
}
