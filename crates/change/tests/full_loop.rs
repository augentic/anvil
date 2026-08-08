//! The native loop end to end: scaffold → `plan author` →
//! `plan execute` (running it is the approval), driven through the
//! same transport-neutral operations the shipped guest dispatches,
//! against the linked mock catalog — the *real* orchestrations,
//! validation tails, and journal cadence run in-process with only the
//! model scripted and adapter behaviour supplied by the mock core.
//! No wasm builds, no sibling checkout, no network.

mod support;

use std::fs;

use change::{LoopStep, Status, plan};
use mock::behaviour;
use mock::invoke::run;
use mock::session::Session;

/// The scripted answers for the whole loop, in dispatch order: the
/// reconciliation grouping (author) and the synthesis response
/// (execute's refine phase). Survey, extract, guidance, and build are
/// deterministic mock operations — no model dispatch.
fn suite_answers() -> Vec<String> {
    vec![mock::answers::greeting_grouping(), mock::answers::greeting_synthesis()]
}

/// Concatenate the per-writer union as JSONL text for substring asserts.
fn journal_text(root: &std::path::Path) -> String {
    let events =
        project::journal::read_union(project::config::Layout::new(root)).expect("journal union");
    events
        .iter()
        .map(|event| serde_json::to_string(event).expect("serialize journal event"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Scaffold a project bound to the mock target and author the
/// single-slice plan (left for operator review — running execute is
/// the approval) — the shared preamble of every loop test.
async fn scaffold_author(session: &Session) {
    let scaffolded = run::<project::init::handlers::Init, _, _>(
        session.provider(),
        project::init::handlers::InitInput {
            adapter: Some("mock".to_string()),
            name: Some("demo".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("scaffold initialises the mock-bound project");
    assert_eq!(scaffolded.adapter_name, "mock");

    let authored = run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding(),
            intent: None,
            force: false,
        },
    )
    .await
    .expect("author exits for review");
    assert_eq!(authored.slices, ["greeting"]);
    assert_eq!(authored.surveyed.len(), 1);
    assert_eq!(authored.surveyed[0].leads, ["greeting"]);
    assert!(authored.hint.contains("emery plan execute"), "{}", authored.hint);
}

#[tokio::test]
async fn author_approve_execute_drains() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();

    scaffold_author(&session).await;

    let executed = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect("execute drains the plan");
    assert_eq!(executed.status, "drained");
    assert_eq!(executed.plan, "demo");
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

    // Plan progress projects `done` from archive facts (RFC-86 D2 / D11).
    let plan_yaml = fs::read_to_string(root.join("plan.yaml")).expect("read plan.yaml");
    assert!(!plan_yaml.contains("status:"), "plan.yaml has no stored status: {plan_yaml}");
    let plan: change::Plan = serde_saphyr::from_str(&plan_yaml).expect("parse plan.yaml");
    let events =
        project::plan::collect_events(&plan, project::config::Layout::new(&root)).expect("events");
    let ladders = project::plan::project_ladders(&plan, &events);
    assert!(
        ladders.values().all(|status| *status == Status::Done),
        "projected ladders: {ladders:?}"
    );

    // Baseline merge output with complete provenance.
    let baseline = root.join(".emery/specs/greeting/spec.md");
    let content = fs::read_to_string(&baseline).expect("baseline spec written");
    assert!(content.contains("ID: REQ-001"), "{content}");
    assert!(content.contains("Sources: main"), "{content}");

    // The merge archived the slice directory; the archived model.yaml
    // carries the kernel-projected provenance inline.
    let archive = fs::read_dir(root.join(".emery/archive"))
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

    // The mock target produced a real, non-empty build output.
    let artifact = behaviour::build_artifact_path(&root, "greeting");
    let body = fs::read_to_string(&artifact).expect("mock build output exists");
    assert!(body.contains("Fixture build — greeting"), "{body}");
    assert!(body.contains("proposal 1, design 1, tasks 1, specs 1"), "{body}");

    // RFC-87 / RFC-86 D27: the artifact arrived through capture + the
    // interim post-merge apply, never an ambient checkout write — the
    // archived fact-substrate build record records the touched path
    // and the journal carries the apply event.
    let builds = archive.join("builds");
    let record_path = fs::read_dir(&builds)
        .expect("archived builds/")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
        .expect("archived build record");
    let record = fs::read_to_string(&record_path).expect("read build record");
    assert!(record.contains("mock-build/greeting.md"), "{record}");
    assert!(
        !archive.join("build/patch.yaml").exists(),
        "patch.yaml must not be build-outcome authority"
    );
    let journal = journal_text(&root);
    assert!(journal.contains("slice.code.applied"), "{journal}");
    assert!(journal.contains("target.wave.opened"), "{journal}");
    assert!(journal.contains("target.merge.wave-committed"), "{journal}");
    assert!(journal.contains("target.merge.wave-succeeded"), "{journal}");

    // Guidance dispatch proof, stronger than a call log: the mock
    // target's guidance brief reached the recorded synthesis prompt.
    let requests = session.model().requests();
    assert!(
        requests
            .iter()
            .flat_map(|request| request.messages.iter())
            .any(|message| message.content.contains("Fixture guidance (target:mock@0.0.0)")),
        "the mock guidance brief appears in a recorded judgment request"
    );

    // Both gate reports were schema-gated and persisted: preflight
    // travelled with the archived slice, postflight landed beside it.
    let preflight = fs::read_to_string(archive.join("merge/preflight.yaml"))
        .expect("archived preflight report");
    assert!(preflight.contains("status: success"), "{preflight}");
    let postflight = fs::read_to_string(archive.join("merge/postflight.yaml"))
        .expect("postflight report beside the archive");
    assert!(postflight.contains("status: success"), "{postflight}");

    // Model cadence: one reconciliation leg, one synthesis leg —
    // exactly the two scripted answers, all consumed.
    assert_eq!(requests.len(), 2);
    session.model().assert_exhausted();
}

// Execute opens plan.execute.started (never projects `approved` /
// plan.transition.approved); re-entrant execute on drained is a no-op.
#[tokio::test]
async fn execute_reentry_noop() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();

    scaffold_author(&session).await;

    let executed = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect("first execute drains");
    assert_eq!(executed.status, "drained");

    // Text rendering closes with the canonical finalize line, with no
    // approval ceremony anywhere.
    let mut out = Vec::new();
    project::handler::Render::render(&executed, &mut out).expect("render");
    let text = String::from_utf8(out).expect("utf8");
    assert!(!text.contains("approved"), "no approval line: {text}");
    assert!(text.contains("drained \u{2014} run /emery:finalize demo"), "{text}");

    // No approval field ever reaches disk: `plan.yaml` carries no
    // lifecycle key and the journal carries no approval event.
    let raw = fs::read_to_string(root.join("plan.yaml")).expect("read plan.yaml");
    assert!(!raw.contains("lifecycle"), "{raw}");
    let journal = journal_text(&root);
    assert!(!journal.contains("plan.transition.approved"), "{journal}");

    // Re-entry on the drained plan is a no-op: drained again, no
    // phases re-run.
    let resumed = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect("re-entrant execute is a no-op");
    assert_eq!(resumed.status, "drained");
    assert!(resumed.phases.is_empty(), "{:?}", resumed.phases);
    let mut out = Vec::new();
    project::handler::Render::render(&resumed, &mut out).expect("render");
    let text = String::from_utf8(out).expect("utf8");
    assert!(text.contains("drained \u{2014} run /emery:finalize demo"), "{text}");
}

// A failed merge preflight gate parks the slice at `built`.
#[tokio::test]
async fn preflight_parks_built() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();

    scaffold_author(&session).await;

    // Trip the mock's failed preflight merge gate.
    fs::write(root.join(behaviour::FAIL_MERGE_PREFLIGHT_MARKER), "").expect("write marker");

    let stopped = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect_err("execute parks on the failed preflight gate");
    assert!(stopped.to_string().contains("target-merge-preflight-failed"), "{stopped}");

    // Nothing merged: the build record remains, no baseline, no archive.
    let metadata = fs::read_to_string(root.join(".emery/slices/greeting/metadata.yaml"))
        .expect("slice still present");
    assert!(metadata.contains("completed-at:"), "{metadata}");
    assert!(
        project::build_record::BuildRecord::present(&root.join(".emery/slices/greeting")),
        "build record must remain after a parked preflight"
    );
    assert!(
        !root.join(".emery/slices/greeting/build/patch.yaml").exists(),
        "patch.yaml is not authority"
    );
    assert!(!root.join(".emery/specs/greeting/spec.md").exists());

    // Clear the gate and resume through the breakout merge, then the
    // loop confirms drained.
    fs::remove_file(root.join(behaviour::FAIL_MERGE_PREFLIGHT_MARKER)).expect("remove marker");
    let merged = run::<slice::handlers::MergeRun, _, _>(
        session.provider(),
        slice::handlers::MergeRunInput {
            name: "greeting".to_string(),
            allow_composition_replace: false,
            preview: false,
            conflict_check: false,
        },
    )
    .await
    .expect("breakout merge resumes");
    let slice::handlers::MergeRunBody::Merged(merged) = merged else {
        panic!("default merge mode commits: {merged:?}");
    };
    assert_eq!(merged.slice, "greeting");
    let resumed = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect("second execute drains");
    assert_eq!(resumed.status, "drained");
}

// A failed merge postflight gate is terminal but non-rollback: the
// committed merge stands, the failed report is archived, execute stops
// with `merge-postflight-failed`, and status stays sticky until the
// next execute acknowledges.
#[tokio::test]
async fn postflight_terminal() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();

    scaffold_author(&session).await;

    // Trip the mock's failed postflight merge gate.
    fs::write(root.join(behaviour::FAIL_MERGE_POSTFLIGHT_MARKER), "").expect("write marker");

    let stopped = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect_err("execute reports the failed postflight gate");
    let stopped = stopped.to_string();
    assert!(stopped.contains("merge-postflight-failed"), "{stopped}");
    assert!(
        !stopped.contains("baseline conflict") && !stopped.contains("in-progress"),
        "hint must not describe a retryable in-progress merge conflict: {stopped}"
    );

    // Non-rollback: the merge committed before the gate ran — baseline
    // written, slice archived, plan entry projects `done`.
    assert!(root.join(".emery/specs/greeting/spec.md").is_file());
    assert!(!root.join(".emery/slices/greeting").exists());
    let plan: change::Plan = serde_saphyr::from_str(
        &fs::read_to_string(root.join("plan.yaml")).expect("read plan.yaml"),
    )
    .expect("parse plan.yaml");
    let events =
        project::plan::collect_events(&plan, project::config::Layout::new(&root)).expect("events");
    let ladders = project::plan::project_ladders(&plan, &events);
    assert!(
        ladders.values().all(|status| *status == Status::Done),
        "projected ladders: {ladders:?}"
    );

    // Failed postflight report persists beside the archive.
    let archive = fs::read_dir(root.join(".emery/archive"))
        .expect("archive dir exists")
        .map(|entry| entry.expect("archive entry").path())
        .find(|path| {
            path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with("-greeting"))
        })
        .expect("archived greeting slice");
    let postflight = fs::read_to_string(archive.join("merge/postflight.yaml"))
        .expect("failed postflight report beside the archive");
    assert!(postflight.contains("status: failure"), "{postflight}");
    let preflight =
        fs::read_to_string(archive.join("merge/preflight.yaml")).expect("preflight still archived");
    assert!(preflight.contains("status: success"), "{preflight}");

    // Status stays sticky — not silent drained — until execute acks.
    let status =
        run::<plan::handlers::Status, _, _>(session.provider(), plan::handlers::StatusInput {})
            .await
            .expect("status after postflight failure");
    assert_eq!(status.next_action, "stop merge-postflight-failed");
    assert_eq!(status.current_step, None);
    assert_eq!(status.resume.as_deref(), Some("emery plan execute"));

    // The journal makes the irreversible state explicit; no ack yet.
    let journal = journal_text(&root);
    assert!(journal.contains("target.merge.wave-committed"), "{journal}");
    assert!(journal.contains("target.merge.wave-postflight-failed"), "{journal}");
    assert!(!journal.contains("target.merge.wave-succeeded"), "{journal}");
    assert!(!journal.contains("slice.merge.succeeded"), "{journal}");
    assert!(!journal.contains("plan.merge-postflight.acknowledged"), "{journal}");

    // Clear the gate and re-run execute: ack clears the sticky stop and
    // the single-entry plan drains.
    fs::remove_file(root.join(behaviour::FAIL_MERGE_POSTFLIGHT_MARKER)).expect("remove marker");
    let resumed = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect("second execute acknowledges and drains");
    assert_eq!(resumed.status, "drained");
    let journal = journal_text(&root);
    assert!(journal.contains("plan.merge-postflight.acknowledged"), "{journal}");
}

#[tokio::test]
async fn build_parks_then_resumes() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();

    scaffold_author(&session).await;

    // Trip the mock's failed-report mode for the first build.
    fs::write(root.join(behaviour::FAIL_BUILD_MARKER), "").expect("write fail marker");

    let stopped = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect_err("first execute parks on the failed build");
    assert!(stopped.to_string().contains("build-failed"), "{stopped}");

    // Clear the failure and resume through the breakout build.
    fs::remove_file(root.join(behaviour::FAIL_BUILD_MARKER)).expect("remove fail marker");
    let rebuilt = run::<slice::handlers::Build, _, _>(
        session.provider(),
        slice::handlers::BuildInput {
            name: "greeting".to_string(),
        },
    )
    .await
    .expect("breakout build resumes");
    assert_eq!(rebuilt.slice, "greeting");

    let resumed = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect("second execute drains");
    assert_eq!(resumed.status, "drained");
    assert_eq!(
        resumed.phases.iter().map(|phase| phase.step).collect::<Vec<_>>(),
        [LoopStep::Merge]
    );
}
