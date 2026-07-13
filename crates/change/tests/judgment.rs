//! The judgment repair loop through the public authoring surface: a
//! malformed reconciliation answer is repaired in-loop with the
//! findings inlined, and an unrepairable answer exhausts the budget
//! and surfaces the last failure.

use change::plan;
use omnia_guest::api::invoke::Invoker;

mod common;

use common::answers;
use common::fixture::{ScriptedProvider, run, scripted_invoker, scripted_project};

/// One initial dispatch plus every repair attempt. Mirrors the
/// private `project::judgment::MAX_REPAIRS` (2) — kept local rather
/// than widening the module for tests; a budget change shows up here
/// as an off-by-one request count.
const JUDGMENT_BUDGET: usize = 3;

/// Schema-invalid on purpose: the envelope misses `slices[]` and the
/// gate prose entirely.
fn malformed_answer() -> String {
    r#"{"version":1,"kind":"response"}"#.to_string()
}

async fn author(
    invoker: &Invoker<ScriptedProvider>,
) -> Result<plan::handlers::AuthorBody, project::handler::Error> {
    run::<plan::handlers::Author, _>(
        invoker,
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: answers::greeting_binding(),
            intent: None,
        },
    )
    .await
}

#[tokio::test]
async fn malformed_repaired_in_loop() {
    let (_tmp, root, _cache) = scripted_project("fixture");
    let invoker = scripted_invoker(&root, vec![malformed_answer(), answers::greeting_grouping()]);

    let authored = author(&invoker).await.expect("the repaired answer lands");
    assert_eq!(authored.slices, ["greeting"]);

    // Two dispatches: the failed answer and its repair. The repair
    // prompt re-presents the failed answer with the findings inlined.
    let requests = invoker.provider().model().requests();
    assert_eq!(requests.len(), 2);
    let repair = &requests[1].messages[0].content;
    assert!(repair.contains("Previous answer (failed validation)"), "{repair}");
    assert!(repair.contains(r#"{"version":1,"kind":"response"}"#), "{repair}");
    invoker.provider().model().assert_exhausted();
}

#[tokio::test]
async fn unrepairable_exhausts_budget() {
    let (_tmp, root, _cache) = scripted_project("fixture");
    let invoker = scripted_invoker(&root, vec![malformed_answer(); JUDGMENT_BUDGET]);

    let err = author(&invoker).await.expect_err("the budget exhausts");
    let detail = err.to_string();
    assert!(detail.contains("proposal-schema"), "the last schema failure surfaces: {detail}");

    // One initial dispatch plus MAX_REPAIRS re-prompts, then the leg
    // gives up — no further call.
    assert_eq!(invoker.provider().model().requests().len(), JUDGMENT_BUDGET);
    invoker.provider().model().assert_exhausted();
}
