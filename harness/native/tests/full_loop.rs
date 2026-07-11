//! The native loop end to end: `plan author`
//! → the operator's `approved` stamp → `plan execute`, driven through
//! the same transport-neutral operations the `specify-dev` binary
//! dispatches, against one native [`Provider`] — so the *real* intent
//! and omnia adapter operations (prompts, schema gates, validation
//! tails) run in-process with only the model scripted. No wasm builds.

use std::fs;

use omnia_guest::api::invocation::Invocation;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::api::operation::Operation;
use omnia_testkit::model::{Harness, Scripted};
use scenario::{ModelBackend, Runtime, Scenario};
use serde_json::json;
use specify_dev::provider::Provider;
use workflow::change::plan::wire::SourceAssign;
use workflow::change::{LoopStep, Status, plan};

mod common;

/// Invoke one operation against the shared provider.
async fn run<R, B>(
    invoker: &Invoker<Provider<Harness<Scripted>>>, input: R::Input,
) -> Result<B, workflow::handler::Error>
where
    R: Operation<Provider<Harness<Scripted>>, Output = B, Error = workflow::handler::Error>,
    B: Send,
{
    invoker.invoke::<R>(Invocation::new(input)).await
}

/// The single raw intent binding `plan author` carries on the wire
/// (the operation desugars it into the structured sources map).
fn bindings() -> Vec<SourceAssign> {
    let intent: SourceAssign = serde_json::from_value(
        json!({ "key": "intent", "adapter": "intent", "value": "Fix the greeting." }),
    )
    .expect("intent binding parses");
    vec![intent]
}

/// The scripted judgment answers for the whole loop, in dispatch
/// order: the intent survey lead and the reconciliation grouping
/// (author), then the intent extract Evidence, the synthesis response,
/// and the omnia build's four phase legs (execute).
fn scripted_answers() -> Vec<&'static str> {
    let survey = r#"{"leads":[{"lead":"feature-x","synopsis":"Fix the greeting."}]}"#;
    let grouping = serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": [{
            "name": "feature-x",
            "sources": [{ "source": "intent", "lead": "feature-x" }],
            "rationale": "One inline intent, one slice."
        }],
        "gate": {
            "change": "## Intent\n\nFix the greeting.\n\n## Scope\n\nOne slice.",
            "discovery-summary": "Sources: 1. Leads: 1.",
            "discovery-source-inventory": "| key | adapter | binding |\n|---|---|---|\n| intent | intent | \"Fix the greeting.\" |"
        }
    }))
    .expect("grouping serialises");
    let extract = r#"{"authority":"intent","claims":[{"kind":"intent","id":"greeting.fix","statement":"Fix the greeting."}]}"#;
    let synthesis = serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slice": "feature-x",
        "model": {
            "requirements": [{
                "title": "greeting behaves as intended",
                "domain": "greeting",
                "claims": [{ "source": "intent", "id": "greeting.fix", "kind": "intent" }],
                "statement": "The greeting surface behaves as the operator intends.",
                "scenarios": ["Intended behaviour observed"]
            }],
            "tasks": [
                { "id": "TASK-001", "text": "Implement the greeting change.", "satisfies": ["REQ-001"] }
            ]
        },
        "artifacts": {
            "proposal": "# feature-x\n\n## Why\n\nThe operator asked for it.\n\n## Domains\n\n- greeting — the affected surface\n\n## Non-goals\n\n- Nothing else.\n",
            "design": "# Design\n\nHow feature-x lands.\n",
            "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Implement the change (TASK-001)\n",
            "specs": [{ "domain": "greeting", "content": "## greeting\nAgent prose body.\n" }]
        }
    }))
    .expect("synthesis response serialises");
    vec![
        survey,
        Box::leak(grouping.into_boxed_str()),
        extract,
        Box::leak(synthesis.into_boxed_str()),
        r#"{"applicable":true,"summary":"generation complete"}"#,
        r#"{"applicable":true,"summary":"review complete"}"#,
        r#"{"applicable":false,"summary":"no captures binding"}"#,
        r#"{"status":"success","findings":[]}"#,
    ]
}

#[tokio::test]
async fn author_approve_execute_drains() {
    let scenario = Scenario::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../quality/scenarios/guest-execute-loop.yaml"),
    )
    .expect("canonical full-loop scenario");
    let profile = scenario
        .profiles
        .iter()
        .find(|profile| profile.id == "native-scripted")
        .expect("native scripted profile");
    assert_eq!(profile.runtime, Runtime::Native);
    assert_eq!(profile.model, ModelBackend::Scripted);
    assert_eq!(
        scenario.workflow.iter().map(|step| step.id.as_str()).collect::<Vec<_>>(),
        ["author", "approve", "execute"]
    );

    let project = common::Project::new();
    let invoker = Invoker::new(
        "specify",
        Provider::new(project.root(), Harness::new(Scripted::answers(scripted_answers()))),
    );

    let resolved = run::<workflow::adapter::handlers::TargetResolve, _>(
        &invoker,
        workflow::adapter::handlers::ResolveInput {
            value: "omnia".to_string(),
            project_dir: None,
        },
    )
    .await
    .expect("typed target resolve uses the native provider");
    assert_eq!(resolved.location, "native");
    assert_eq!(resolved.resolved_path, "rust:target:omnia");

    let authored = run::<plan::handlers::Author, _>(
        &invoker,
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: bindings(),
            intent: None,
        },
    )
    .await
    .expect("author walks to pending");
    assert_eq!(authored.lifecycle, "pending");
    assert_eq!(authored.slices, ["feature-x"]);
    assert!(authored.hint.contains("specify plan transition demo approved"), "{}", authored.hint);

    run::<plan::handlers::Transition, _>(
        &invoker,
        plan::handlers::TransitionInput {
            name: "demo".to_string(),
            target: Some("approved".to_string()),
            undo: false,
            actor: "operator".to_string(),
        },
    )
    .await
    .expect("the operator stamps Gate 1");

    let executed = run::<plan::handlers::Execute, _>(&invoker, plan::handlers::ExecuteInput {})
        .await
        .expect("execute drains the plan");
    assert_eq!(executed.status, "drained");
    let ran: Vec<(&str, LoopStep)> =
        executed.phases.iter().map(|phase| (phase.slice.as_str(), phase.step)).collect();
    assert_eq!(
        ran,
        [
            ("feature-x", LoopStep::Refine),
            ("feature-x", LoopStep::Build),
            ("feature-x", LoopStep::Merge),
        ]
    );

    let plan: workflow::change::Plan = serde_saphyr::from_str(
        &fs::read_to_string(project.root().join("plan.yaml")).expect("read plan.yaml"),
    )
    .expect("parse plan.yaml");
    assert!(plan.entries.iter().all(|entry| entry.status == Status::Done), "{:?}", plan.entries);
    let baseline = project.root().join(".specify/specs/greeting/spec.md");
    let content = fs::read_to_string(&baseline).expect("baseline spec written");
    assert!(content.contains("ID: REQ-001"), "{content}");
    assert!(content.contains("Sources: intent"), "{content}");

    let requests = invoker.provider().model().requests();
    assert_eq!(requests.len(), 8, "survey, reconcile, extract, synthesis, and four build legs");
    assert!(requests[4].lend_workspace, "the omnia generation leg lends the workspace");
    invoker.provider().model().assert_exhausted();
}

#[tokio::test]
async fn intent_pilot_refines() {
    let scenario = Scenario::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../quality/scenarios/intent-only.yaml"),
    )
    .expect("canonical intent scenario");
    assert_eq!(
        scenario.workflow.iter().map(|step| step.id.as_str()).collect::<Vec<_>>(),
        ["author", "approve", "claim", "refine"]
    );

    let project = common::Project::new();
    let invoker = Invoker::new(
        "specify",
        Provider::new(
            project.root(),
            Harness::new(Scripted::answers(scripted_answers().into_iter().take(4))),
        ),
    );
    run::<plan::handlers::Author, _>(
        &invoker,
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: bindings(),
            intent: None,
        },
    )
    .await
    .expect("author");
    run::<plan::handlers::Transition, _>(
        &invoker,
        plan::handlers::TransitionInput {
            name: "demo".to_string(),
            target: Some("approved".to_string()),
            undo: false,
            actor: "operator".to_string(),
        },
    )
    .await
    .expect("approve");
    run::<plan::handlers::Next, _>(&invoker, plan::handlers::NextInput {}).await.expect("claim");
    let refined = run::<workflow::slice::handlers::Refine, _>(
        &invoker,
        workflow::slice::handlers::RefineInput {
            name: "feature-x".to_string(),
        },
    )
    .await
    .expect("refine");

    assert_eq!(refined.slice, "feature-x");
    assert!(refined.artifacts.iter().any(|artifact| artifact.ends_with("spec.md")));
    assert_eq!(invoker.provider().model().requests().len(), 4);
    invoker.provider().model().assert_exhausted();
}

#[tokio::test]
async fn failure_pilot_resumes() {
    let scenario = Scenario::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../quality/scenarios/execute-fail-resume.yaml"),
    )
    .expect("canonical failure scenario");
    assert_eq!(
        scenario.workflow.iter().map(|step| step.id.as_str()).collect::<Vec<_>>(),
        ["author", "approve", "execute-fails", "build-resumes", "execute-resumes"]
    );

    let mut answers = scripted_answers().into_iter().map(str::to_string).collect::<Vec<_>>();
    answers[7] = r#"{"status":"failure","findings":[]}"#.to_string();
    answers.extend(
        [
            r#"{"applicable":true,"summary":"generation resumed"}"#,
            r#"{"applicable":true,"summary":"review resumed"}"#,
            r#"{"applicable":false,"summary":"no captures binding"}"#,
            r#"{"status":"success","findings":[]}"#,
        ]
        .map(str::to_string),
    );

    let project = common::Project::new();
    let invoker = Invoker::new(
        "specify",
        Provider::new(project.root(), Harness::new(Scripted::answers(answers))),
    );
    run::<plan::handlers::Author, _>(
        &invoker,
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: bindings(),
            intent: None,
        },
    )
    .await
    .expect("author");
    run::<plan::handlers::Transition, _>(
        &invoker,
        plan::handlers::TransitionInput {
            name: "demo".to_string(),
            target: Some("approved".to_string()),
            undo: false,
            actor: "operator".to_string(),
        },
    )
    .await
    .expect("approve");

    let stopped = run::<plan::handlers::Execute, _>(&invoker, plan::handlers::ExecuteInput {})
        .await
        .expect_err("first execute parks");
    assert!(stopped.to_string().contains("build-failed"), "{stopped}");

    let rebuilt = run::<workflow::slice::handlers::Build, _>(
        &invoker,
        workflow::slice::handlers::BuildInput {
            name: "feature-x".to_string(),
        },
    )
    .await
    .expect("breakout build resumes");
    assert_eq!(rebuilt.slice, "feature-x");

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
