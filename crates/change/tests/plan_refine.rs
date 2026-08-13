//! The serial refinement drain (`emery plan refine`, RFC-91): a fresh
//! closed plan drains every leaf into a per-slice `refinement.yaml`,
//! re-running skips fresh leaves, an amended entry goes stale and is
//! picked up again, `--slice` restricts the drain to the named leaf
//! plus its stale predecessor closure, and a mid-drain adapter failure
//! parks the drain with the typed `plan-refine-stopped` halt that a
//! re-run resumes past.

mod support;

use change::plan;
use change::plan::wire::{BindingArg, SourceAssign};
use mock::invoke::run;
use mock::session::Session;
use serde_json::json;

/// The two-source grouping: `alpha` bound to `main`, `beta` bound to
/// `aux` — both sources survey the minimal profile's single
/// `greeting` lead.
fn pair_grouping() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": [
            {
                "name": "alpha",
                "sources": [{ "source": "main", "lead": "greeting" }],
                "rationale": "The main source's lead."
            },
            {
                "name": "beta",
                "sources": [{ "source": "aux", "lead": "greeting" }],
                "rationale": "The aux source's lead."
            }
        ],
        "gate": {
            "change": "## Intent\n\nCharacterise the greeting service.\n\n## Scope\n\nTwo slices.",
            "discovery-summary": "Sources: 2. Leads: 2.",
            "discovery-source-inventory": "| key | adapter | binding |\n|---|---|---|\n| main | mock | \"The greeting service.\" |\n| aux | mock | \"The greeting service.\" |"
        }
    }))
    .expect("grouping serialises")
}

/// The greeting synthesis answer re-slotted for `slice`, with its
/// claim anchored on the `source` binding key. The spec domain is the
/// slice name, so the two leaves merge into disjoint baselines.
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

/// Two bindings over the minimal mock profile; `aux_adapter` selects
/// the behaviour of the second source (`mock` or a failure profile).
fn pair_bindings(aux_adapter: &str) -> Vec<SourceAssign> {
    ["main", "aux"]
        .map(|key| {
            let adapter = if key == "aux" { aux_adapter } else { "mock" };
            serde_json::from_value(json!({
                "key": key,
                "adapter": adapter,
                "value": "The greeting service.",
            }))
            .expect("mock binding parses")
        })
        .to_vec()
}

async fn author(session: &Session, sources: Vec<SourceAssign>) {
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

fn manifest_path(root: &std::path::Path, slice: &str) -> std::path::PathBuf {
    root.join(".emery/change/slices").join(slice).join("refinement.yaml")
}

// (1) + (2): a fresh closed plan drains every leaf in entry order and
// writes one refinement.yaml per slice; the immediate re-run refines
// nothing — every leaf is fresh and skipped, with no model dispatch.
#[tokio::test]
async fn drains_then_skips() {
    let session = Session::scripted(
        "mock",
        vec![pair_grouping(), synthesis_for("alpha", "main"), synthesis_for("beta", "aux")],
    );
    let root = session.root().to_path_buf();

    author(&session, pair_bindings("mock")).await;

    let drained = support::refine_plan(&session).await;
    assert_eq!(drained.status, "refined");
    assert_eq!(drained.refined, ["alpha", "beta"]);
    assert!(drained.skipped.is_empty(), "{:?}", drained.skipped);
    assert!(!drained.gaps, "the minimal profile synthesises no gaps");
    for slice in ["alpha", "beta"] {
        assert!(manifest_path(&root, slice).is_file(), "refinement.yaml written for `{slice}`");
    }
    // Both synthesis answers were consumed by the drain — the script
    // is exhausted, so any re-refine below would fail loudly.
    session.model().assert_exhausted();
    let digests = ["alpha", "beta"].map(|slice| support::manifest_digest(&root, slice));

    // Re-running the drain is the resume path: every leaf is fresh, so
    // nothing re-refines and the manifests do not move.
    let resumed = support::refine_plan(&session).await;
    assert!(resumed.refined.is_empty(), "{:?}", resumed.refined);
    assert_eq!(resumed.skipped, ["alpha", "beta"]);
    assert_eq!(
        ["alpha", "beta"].map(|slice| support::manifest_digest(&root, slice)),
        digests,
        "fresh manifests are never rewritten"
    );
}

// (3): amending the leaf's own plan entry drifts the recorded
// planning `entry` projection, so the manifest goes stale and the next
// drain re-refines exactly that leaf.
#[tokio::test]
async fn amend_goes_stale() {
    let session = Session::scripted(
        "mock",
        vec![
            mock::answers::greeting_grouping(),
            mock::answers::greeting_synthesis(),
            mock::answers::greeting_synthesis(),
        ],
    );
    let root = session.root().to_path_buf();

    author(&session, support::greeting_binding()).await;
    let drained = support::refine_plan(&session).await;
    assert_eq!(drained.refined, ["greeting"]);
    let before = support::manifest_digest(&root, "greeting");

    // The amended description enters the covered `entry` projection.
    run::<plan::handlers::Amend, _, _>(
        session.provider(),
        plan::handlers::AmendInput {
            name: "greeting".to_string(),
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
    .expect("amend the entry");

    let redrained = support::refine_plan(&session).await;
    assert_eq!(redrained.refined, ["greeting"], "the stale leaf re-refines");
    assert!(redrained.skipped.is_empty(), "{:?}", redrained.skipped);
    let after = support::manifest_digest(&root, "greeting");
    assert_ne!(before, after, "re-refinement rewrote the manifest over the amended entry");
    session.model().assert_exhausted();
}

// (4): `--slice <leaf>` targets the named leaf plus the stale-or-
// missing predecessor closure it needs — unrelated siblings are left
// untouched, and a second restricted run skips the now-fresh leaf.
#[tokio::test]
async fn slice_closure() {
    let session = Session::scripted(
        "mock",
        vec![
            mock::answers::adversarial_grouping(),
            mock::answers::login_flow_synthesis(),
            mock::answers::password_reset_synthesis(),
        ],
    );
    let root = session.root().to_path_buf();

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
    .expect("author exits for review");

    // Give the selected leaf a predecessor so the closure has work.
    run::<plan::handlers::Amend, _, _>(
        session.provider(),
        plan::handlers::AmendInput {
            name: "password-reset".to_string(),
            depends_on: Some(vec!["login-flow".to_string()]),
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

    let drained = support::refine_slices(&session, &["password-reset"])
        .await
        .expect("restricted drain completes");
    // The missing predecessor joins the target set; the unrelated
    // sibling does not.
    assert_eq!(drained.refined, ["login-flow", "password-reset"]);
    assert!(drained.skipped.is_empty(), "{:?}", drained.skipped);
    assert!(
        !root.join(".emery/change/slices/session-policy").exists(),
        "the unselected sibling is untouched"
    );
    // The password-reset evidence gap surfaces as an open `[unknown]`.
    assert!(drained.gaps, "the gapped slice reports open gaps");
    session.model().assert_exhausted();

    // Re-running the restriction skips: the leaf is fresh and its
    // fresh predecessor no longer enters the closure.
    let resumed = support::refine_slices(&session, &["password-reset"])
        .await
        .expect("restricted re-run completes");
    assert!(resumed.refined.is_empty(), "{:?}", resumed.refined);
    assert_eq!(resumed.skipped, ["password-reset"]);
}

// (5): a mid-drain adapter failure parks the drain with the typed
// `plan-refine-stopped` halt; prior manifests stay, re-running skips
// the fresh leaf and retries the failed one, and fixing the input
// lets the resume complete.
#[tokio::test]
async fn stop_then_resume() {
    let session = Session::scripted(
        "mock",
        vec![pair_grouping(), synthesis_for("alpha", "main"), synthesis_for("beta", "main")],
    );
    let root = session.root().to_path_buf();

    // `beta`'s bound source fails extract; `alpha`'s succeeds.
    author(&session, pair_bindings("mock-fail-extract")).await;

    let stopped = support::refine_slices(&session, &[])
        .await
        .expect_err("the failing extraction parks the drain")
        .to_string();
    assert!(stopped.contains("plan-refine-stopped"), "{stopped}");
    assert!(stopped.contains("`beta`"), "the stop names the parked slice: {stopped}");
    assert!(stopped.contains("mock extract failure"), "typed detail preserved: {stopped}");
    // The leaf refined before the stop keeps its manifest; the parked
    // leaf has none.
    assert!(manifest_path(&root, "alpha").is_file());
    assert!(!manifest_path(&root, "beta").exists());
    let alpha = support::manifest_digest(&root, "alpha");

    // Re-running resumes: `alpha` is fresh (no model dispatch — a
    // re-refine would consume `beta`'s scripted answer and unravel
    // below), `beta` fails again with the same typed stop.
    let stopped = support::refine_slices(&session, &[])
        .await
        .expect_err("the unfixed extraction parks the drain again")
        .to_string();
    assert!(stopped.contains("plan-refine-stopped"), "{stopped}");
    assert!(stopped.contains("mock extract failure"), "{stopped}");
    assert_eq!(support::manifest_digest(&root, "alpha"), alpha, "the fresh leaf is skipped");

    // Fix the input: rebind `beta` onto the healthy `main` source.
    // The unrelated amend leaves `alpha`'s manifest fresh.
    run::<plan::handlers::Amend, _, _>(
        session.provider(),
        plan::handlers::AmendInput {
            name: "beta".to_string(),
            depends_on: None,
            sources: Some(vec![BindingArg {
                key: "main".to_string(),
                lead: Some("greeting".to_string()),
            }]),
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
    .expect("rebind the failing source");

    let resumed = support::refine_plan(&session).await;
    assert_eq!(resumed.refined, ["beta"], "the resume refines only the parked leaf");
    assert_eq!(resumed.skipped, ["alpha"]);
    assert!(manifest_path(&root, "beta").is_file());
    assert_eq!(support::manifest_digest(&root, "alpha"), alpha, "unrelated amend keeps fresh");
    session.model().assert_exhausted();
}

// Regression: a multi-entry plan drains through execute. The first
// merge archives `alpha`'s slice tree, so `beta`'s pre-build epoch
// gate must re-check only the claimed slice — not every covered leaf —
// or the loop dies `plan-epoch-stale` after the first merge.
#[tokio::test]
async fn pair_execute_drains() {
    let session = Session::scripted(
        "mock",
        vec![pair_grouping(), synthesis_for("alpha", "main"), synthesis_for("beta", "aux")],
    );
    author(&session, pair_bindings("mock")).await;
    let drained = support::refine_plan(&session).await;
    assert_eq!(drained.refined, ["alpha", "beta"]);

    let executed = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect("execute drains both entries past the first merge");
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
        ]
    );
    session.model().assert_exhausted();
}
