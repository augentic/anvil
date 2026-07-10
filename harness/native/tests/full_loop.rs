//! The native loop end to end: `plan author`
//! → the operator's `approved` stamp → `plan execute`, driven through
//! the same transport-neutral verb handlers the `specify-dev` binary
//! dispatches, against one [`NativeProvider`] — so the *real* intent
//! and omnia adapter operations (prompts, schema gates, validation
//! tails) run in-process with only the model scripted. No wasm builds.

use std::fs;

use omnia_guest::api::Handler;
use serde_json::json;
use specify_dev::provider::NativeProvider;
use testkit::MockModel;
use workflow::change::plan::handlers::SourceAssign;
use workflow::change::{LoopStep, Status, plan};
use workflow::orchestrate;

mod common;

/// Drive one verb handler against the provider, returning the typed
/// body or the verb-layer failure — the same path both transports take.
async fn run<R, B>(
    provider: &NativeProvider<MockModel>, input: R::Input,
) -> Result<B, workflow::handler::Error>
where
    R: Handler<
            NativeProvider<MockModel>,
            Output = workflow::handler::Out<B>,
            Error = workflow::handler::Error,
        >,
    B: Send + Sync,
{
    let reply = R::handler(input)?.owner("specify").provider(provider).handle().await?;
    Ok(reply.body.0)
}

/// The single raw intent binding `plan author` carries on the wire
/// (`from_input` desugars it into the structured sources map).
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
    let project = common::Project::new();
    let provider = NativeProvider::new(project.root(), MockModel::answering(scripted_answers()));

    // `plan author` — survey through the real intent adapter, the
    // reconciliation judgment leg, Gate 1 prose — exits at `pending`.
    let authored = run::<orchestrate::handlers::Author, _>(
        &provider,
        orchestrate::handlers::AuthorInput {
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

    // Gate 1 — the operator-only stamp, through the same verb the CLI
    // routes.
    run::<plan::handlers::Transition, _>(
        &provider,
        plan::handlers::TransitionInput {
            name: "demo".to_string(),
            target: Some("approved".to_string()),
            undo: false,
            actor: "operator".to_string(),
        },
    )
    .await
    .expect("the operator stamps Gate 1");

    // `plan execute` — the drained refine → build → merge loop over
    // the real adapter operations.
    let executed =
        run::<orchestrate::handlers::Execute, _>(&provider, orchestrate::handlers::ExecuteInput {})
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

    // The merge stamped the entry `done` and folded the baseline spec.
    let plan =
        workflow::change::Plan::load(&project.root().join("plan.yaml")).expect("load plan.yaml");
    assert!(plan.entries.iter().all(|entry| entry.status == Status::Done), "{:?}", plan.entries);
    let baseline = project.root().join(".specify/specs/greeting/spec.md");
    let content = fs::read_to_string(&baseline).expect("baseline spec written");
    assert!(content.contains("ID: REQ-001"), "{content}");
    assert!(content.contains("Sources: intent"), "{content}");

    // Every scripted leg consumed — the real adapters dispatched
    // exactly the expected judgment cadence.
    let requests = provider.model().requests();
    assert_eq!(requests.len(), 8, "survey, reconcile, extract, synthesis, and four build legs");
    assert!(requests[4].lend_workspace, "the omnia generation leg lends the workspace");
}
