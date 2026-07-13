//! The native loop end to end: scaffold → `plan author` → the
//! operator's `approved` stamp → `plan execute`, driven through the
//! same transport-neutral operations the shipped guest dispatches,
//! against the fixture provider — the *real* orchestrations,
//! validation tails, and journal cadence run in-process with only the
//! model scripted and adapter behaviour supplied by the fixture core.
//! No wasm builds, no sibling checkout, no network.

use std::fs;

use omnia_guest::api::invoke::Invoker;
use workflow::change::{LoopStep, Status, plan};

mod common;

use common::answers;
use common::fixture::{ScriptedProvider, run, scripted_invoker};

/// The scripted judgment answers for the whole loop, in dispatch
/// order: the reconciliation grouping (author) and the synthesis
/// response (execute's refine phase). Survey, extract, guidance, and
/// build are deterministic fixture operations — no model dispatch.
fn scripted_answers() -> Vec<String> {
    vec![answers::greeting_grouping(), answers::greeting_synthesis()]
}

/// Scaffold a project bound to the fixture target and author + approve
/// the single-slice plan — the shared preamble of every loop test.
async fn scaffold_author_approve(invoker: &Invoker<ScriptedProvider>) {
    let scaffolded = run::<workflow::init::handlers::Init, _>(
        invoker,
        workflow::init::handlers::InitInput {
            adapter: Some("fixture".to_string()),
            name: Some("demo".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("scaffold initialises the fixture-bound project");
    assert_eq!(scaffolded.adapter_name, "fixture");

    let authored = run::<plan::handlers::Author, _>(
        invoker,
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: answers::greeting_binding(),
            intent: None,
        },
    )
    .await
    .expect("author walks to pending");
    assert_eq!(authored.lifecycle, "pending");
    assert_eq!(authored.slices, ["greeting"]);
    assert_eq!(authored.surveyed.len(), 1);
    assert_eq!(authored.surveyed[0].leads, ["greeting"]);
    assert!(authored.hint.contains("specify plan transition demo approved"), "{}", authored.hint);

    run::<plan::handlers::Transition, _>(
        invoker,
        plan::handlers::TransitionInput {
            name: "demo".to_string(),
            target: Some("approved".to_string()),
            undo: false,
            actor: "operator".to_string(),
        },
    )
    .await
    .expect("the operator stamps Gate 1");
}

#[tokio::test]
async fn author_approve_execute_drains() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical tempdir");
    let _cache = common::scoped_cache(&root);
    let invoker = scripted_invoker(&root, scripted_answers());

    scaffold_author_approve(&invoker).await;

    let executed = run::<plan::handlers::Execute, _>(&invoker, plan::handlers::ExecuteInput {})
        .await
        .expect("execute drains the plan");
    assert_eq!(executed.status, "drained");
    let ran: Vec<(&str, LoopStep)> =
        executed.phases.iter().map(|phase| (phase.slice.as_str(), phase.step)).collect();
    assert_eq!(
        ran,
        [
            ("greeting", LoopStep::Refine),
            ("greeting", LoopStep::Build),
            ("greeting", LoopStep::Merge),
        ]
    );

    // Plan lifecycle: every entry is `done`.
    let plan: workflow::change::Plan = serde_saphyr::from_str(
        &fs::read_to_string(root.join("plan.yaml")).expect("read plan.yaml"),
    )
    .expect("parse plan.yaml");
    assert!(plan.entries.iter().all(|entry| entry.status == Status::Done), "{:?}", plan.entries);

    // Baseline merge output with complete provenance.
    let baseline = root.join(".specify/specs/greeting/spec.md");
    let content = fs::read_to_string(&baseline).expect("baseline spec written");
    assert!(content.contains("ID: REQ-001"), "{content}");
    assert!(content.contains("Sources: main"), "{content}");

    // The merge archived the slice directory; the archived model.yaml
    // carries the kernel-projected provenance inline.
    let archive = fs::read_dir(root.join(".specify/archive"))
        .expect("archive dir exists")
        .map(|entry| entry.expect("archive entry").path())
        .find(|path| {
            path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with("-greeting"))
        })
        .expect("archived greeting slice");
    let model: serde_json::Value = serde_saphyr::from_str(
        &fs::read_to_string(archive.join("model.yaml")).expect("archived model.yaml"),
    )
    .expect("model.yaml parses");
    assert_eq!(model["slice"], "greeting");
    let requirement = &model["requirements"][0];
    assert_eq!(requirement["id"], "REQ-001");
    assert_eq!(requirement["status"], "agreed");
    assert_eq!(requirement["claims"][0]["source"], "main");

    // The fixture target produced a real, non-empty build output.
    let artifact = fixtures::build_artifact_path(&root, "greeting");
    let body = fs::read_to_string(&artifact).expect("fixture build output exists");
    assert!(body.contains("Fixture build — greeting"), "{body}");
    assert!(body.contains("proposal 1, design 1, tasks 1, specs 1"), "{body}");

    // Seam cadence: survey (author), extract + guidance (refine),
    // build, then the phased merge gates around the deterministic
    // commit.
    assert_eq!(
        invoker.provider().calls(),
        [
            "survey source:fixture",
            "extract source:fixture",
            "guidance target:fixture",
            "build target:fixture",
            "merge-preflight target:fixture",
            "merge-postflight target:fixture",
        ]
    );

    // Both gate reports were schema-gated and persisted: preflight
    // travelled with the archived slice, postflight landed beside it.
    let preflight = fs::read_to_string(archive.join("merge/preflight.yaml"))
        .expect("archived preflight report");
    assert!(preflight.contains("status: success"), "{preflight}");
    let postflight = fs::read_to_string(archive.join("merge/postflight.yaml"))
        .expect("postflight report beside the archive");
    assert!(postflight.contains("status: success"), "{postflight}");

    // Model cadence: one reconciliation leg, one synthesis leg.
    let requests = invoker.provider().model().requests();
    assert_eq!(requests.len(), 2, "reconcile + synthesis only; adapters are deterministic");
    invoker.provider().model().assert_exhausted();
}

// A failed merge preflight gate parks the slice at `built`.
#[tokio::test]
async fn preflight_parks_built() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical tempdir");
    let _cache = common::scoped_cache(&root);
    let invoker = scripted_invoker(&root, scripted_answers());

    scaffold_author_approve(&invoker).await;

    // Trip the fixture's failed preflight merge gate.
    fs::write(root.join(fixtures::FAIL_MERGE_PREFLIGHT_MARKER), "").expect("write marker");

    let stopped = run::<plan::handlers::Execute, _>(&invoker, plan::handlers::ExecuteInput {})
        .await
        .expect_err("execute parks on the failed preflight gate");
    assert!(stopped.to_string().contains("target-merge-preflight-failed"), "{stopped}");

    // Nothing merged: the slice stays `built`, no baseline, no archive.
    let metadata = fs::read_to_string(root.join(".specify/slices/greeting/metadata.yaml"))
        .expect("slice still present");
    assert!(metadata.contains("status: built"), "{metadata}");
    assert!(!root.join(".specify/specs/greeting/spec.md").exists());

    // Clear the gate and resume through the breakout merge, then the
    // loop confirms drained.
    fs::remove_file(root.join(fixtures::FAIL_MERGE_PREFLIGHT_MARKER)).expect("remove marker");
    let merged = run::<workflow::slice::handlers::MergeRun, _>(
        &invoker,
        workflow::slice::handlers::MergeRunInput {
            name: "greeting".to_string(),
            allow_composition_replace: false,
        },
    )
    .await
    .expect("breakout merge resumes");
    assert_eq!(merged.slice, "greeting");
    let resumed = run::<plan::handlers::Execute, _>(&invoker, plan::handlers::ExecuteInput {})
        .await
        .expect("second execute drains");
    assert_eq!(resumed.status, "drained");
    invoker.provider().model().assert_exhausted();
}

// A failed merge postflight gate is terminal but non-rollback: the
// committed merge stands.
#[tokio::test]
async fn postflight_terminal() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical tempdir");
    let _cache = common::scoped_cache(&root);
    let invoker = scripted_invoker(&root, scripted_answers());

    scaffold_author_approve(&invoker).await;

    // Trip the fixture's failed postflight merge gate.
    fs::write(root.join(fixtures::FAIL_MERGE_POSTFLIGHT_MARKER), "").expect("write marker");

    let stopped = run::<plan::handlers::Execute, _>(&invoker, plan::handlers::ExecuteInput {})
        .await
        .expect_err("execute reports the failed postflight gate");
    assert!(stopped.to_string().contains("target-merge-postflight-failed"), "{stopped}");

    // Non-rollback: the merge committed before the gate ran — baseline
    // written, slice archived, plan entry `done`.
    assert!(root.join(".specify/specs/greeting/spec.md").is_file());
    assert!(!root.join(".specify/slices/greeting").exists());
    let plan: workflow::change::Plan = serde_saphyr::from_str(
        &fs::read_to_string(root.join("plan.yaml")).expect("read plan.yaml"),
    )
    .expect("parse plan.yaml");
    assert!(plan.entries.iter().all(|entry| entry.status == Status::Done), "{:?}", plan.entries);

    // The journal makes the irreversible state explicit.
    let journal = fs::read_to_string(root.join(".specify/journal.jsonl")).expect("journal");
    assert!(journal.contains("slice.merge.postflight-failed"), "{journal}");
    assert!(!journal.contains("slice.merge.succeeded"), "{journal}");

    invoker.provider().model().assert_exhausted();
}

#[tokio::test]
async fn build_failure_parks_then_resumes() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical tempdir");
    let _cache = common::scoped_cache(&root);
    let invoker = scripted_invoker(&root, scripted_answers());

    scaffold_author_approve(&invoker).await;

    // Trip the fixture's failed-report mode for the first build.
    fs::write(root.join(fixtures::FAIL_BUILD_MARKER), "").expect("write fail marker");

    let stopped = run::<plan::handlers::Execute, _>(&invoker, plan::handlers::ExecuteInput {})
        .await
        .expect_err("first execute parks on the failed build");
    assert!(stopped.to_string().contains("build-failed"), "{stopped}");

    // Clear the failure and resume through the breakout build.
    fs::remove_file(root.join(fixtures::FAIL_BUILD_MARKER)).expect("remove fail marker");
    let rebuilt = run::<workflow::slice::handlers::Build, _>(
        &invoker,
        workflow::slice::handlers::BuildInput {
            name: "greeting".to_string(),
        },
    )
    .await
    .expect("breakout build resumes");
    assert_eq!(rebuilt.slice, "greeting");

    let resumed = run::<plan::handlers::Execute, _>(&invoker, plan::handlers::ExecuteInput {})
        .await
        .expect("second execute drains");
    assert_eq!(resumed.status, "drained");
    assert_eq!(
        resumed.phases.iter().map(|phase| phase.step).collect::<Vec<_>>(),
        [LoopStep::Merge]
    );
    invoker.provider().model().assert_exhausted();
}
