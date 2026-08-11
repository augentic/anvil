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

/// A grouping response over `(slice, source)` pairs; every source
/// surveys the minimal profile's single `greeting` lead.
fn grouping(slices: &[(&str, &str)]) -> String {
    let rows: Vec<serde_json::Value> = slices
        .iter()
        .map(|(name, source)| {
            json!({
                "name": name,
                "sources": [{ "source": source, "lead": "greeting" }],
                "rationale": format!("The {source} source's lead."),
            })
        })
        .collect();
    let inventory: Vec<String> = slices
        .iter()
        .map(|(_, source)| format!("| {source} | mock | \"The greeting service.\" |"))
        .collect();
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": rows,
        "gate": {
            "change": "## Intent\n\nCharacterise the greeting service.\n\n## Scope\n\nSlices.",
            "discovery-summary": format!("Sources: {}. Leads: {}.", slices.len(), slices.len()),
            "discovery-source-inventory": format!(
                "| key | adapter | binding |\n|---|---|---|\n{}",
                inventory.join("\n")
            ),
        }
    }))
    .expect("grouping serialises")
}

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

/// One `value` binding per source key onto the minimal mock profile.
fn bindings(keys: &[&str]) -> Vec<change::plan::wire::SourceAssign> {
    keys.iter()
        .map(|key| {
            serde_json::from_value(json!({
                "key": key,
                "adapter": "mock",
                "value": "The greeting service.",
            }))
            .expect("mock binding parses")
        })
        .collect()
}

async fn author(session: &Session, sources: Vec<change::plan::wire::SourceAssign>) {
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources,
            intent: None,
            force: false,
        },
    )
    .await
    .expect("author exits for review");
}

async fn depend(session: &Session, slice: &str, on: &str) {
    run::<plan::handlers::Amend, _, _>(
        session.provider(),
        plan::handlers::AmendInput {
            name: slice.to_string(),
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
            grouping(&[("alpha", "main"), ("beta", "aux"), ("gamma", "ter")]),
            synthesis_for("alpha", "main"),
            synthesis_for("beta", "aux"),
            synthesis_for("gamma", "ter"),
        ],
    );
    let root = session.root().to_path_buf();

    author(&session, bindings(&["main", "aux", "ter"])).await;
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
}

// (AC7) Execute re-entry after a plan-local merge: alpha merges and
// archives (moving the baseline), execute stops on beta's failed
// build, and the re-run assembles fresh coverage — alpha is done and
// excluded, and beta's manifest is still fresh because the live
// baseline matches the journaled post-merge digest (D4). Without the
// plan-local carve-out the re-run would fail `plan-refinement-required`.
#[tokio::test]
async fn merge_stop_reentry() {
    let session = Session::scripted(
        "mock",
        vec![
            grouping(&[("alpha", "main"), ("beta", "aux")]),
            synthesis_for("alpha", "main"),
            synthesis_for("beta", "aux"),
        ],
    );
    let root = session.root().to_path_buf();

    author(&session, bindings(&["main", "aux"])).await;
    support::refine_plan(&session).await;
    let beta_digest = support::manifest_digest(&root, "beta");

    // Run 1: alpha builds, then its merge preflight fails — the stop
    // parks with alpha built and not merged.
    support::marker(&root, mock::behaviour::PREFLIGHT_FAIL);
    let stopped = execute(&session).await.expect_err("preflight failure stops").to_string();
    assert!(stopped.contains("plan-execute-stopped"), "{stopped}");
    assert!(stopped.contains("merge-conflict"), "{stopped}");
    assert!(root.join(".emery/slices/alpha").is_dir(), "alpha not merged yet");

    // Run 2: alpha merges and archives (the baseline moves), then
    // beta's build fails on the marker — the stop leaves beta unbuilt
    // with its pre-merge manifest.
    std::fs::remove_file(root.join(mock::behaviour::PREFLIGHT_FAIL)).expect("rm marker");
    support::marker(&root, mock::behaviour::FAIL_BUILD_MARKER);
    let stopped = execute(&session).await.expect_err("build failure stops").to_string();
    assert!(stopped.contains("build-failed"), "{stopped}");
    assert!(!root.join(".emery/slices/alpha").exists(), "alpha merged and archived");

    // Run 3: fresh coverage over the moved baseline — alpha is done
    // and excluded, beta's untouched manifest still projects fresh —
    // and the loop drains beta.
    std::fs::remove_file(root.join(mock::behaviour::FAIL_BUILD_MARKER)).expect("rm marker");
    let executed = execute(&session).await.expect("re-entry drains");
    assert_eq!(executed.status, "drained");
    assert_eq!(
        support::manifest_digest(&root, "beta"),
        beta_digest,
        "beta drained without a re-refine"
    );
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
            grouping(&[("alpha", "main"), ("beta", "aux")]),
            synthesis_for("alpha", "main"),
            synthesis_for("beta", "aux"),
            synthesis_for("beta", "aux"),
        ],
    );
    let root = session.root().to_path_buf();

    author(&session, bindings(&["main", "aux"])).await;
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
    assert!(!root.join(".emery/slices/alpha").exists(), "alpha merged and archived");

    // Amend the dependent post-merge: the entry projection drifts and
    // the manifest goes stale.
    run::<plan::handlers::Amend, _, _>(
        session.provider(),
        plan::handlers::AmendInput {
            name: "beta".to_string(),
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
