//! The judgment repair loop through the public authoring surface: a
//! malformed reconciliation answer is repaired in-loop with the
//! findings inlined, and an unrepairable answer exhausts the budget
//! and surfaces the last failure.
//!
//! Stays scripted rather than replayed: the loop only engages when the
//! model emits schema-violating answers, which the format-gated replay
//! engine refuses to serve by design (it behaves like a
//! schema-enforcing backend).

use change::plan;
use testkit::{ScriptedProvider, answers, run};

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
    provider: &ScriptedProvider,
) -> Result<plan::handlers::AuthorBody, project::handler::Error> {
    run::<plan::handlers::Author, _, _>(
        provider,
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
    let provider = ScriptedProvider::scripted(
        "fixture",
        vec![malformed_answer(), answers::greeting_grouping()],
    );

    let authored = author(&provider).await.expect("the repaired answer lands");
    assert_eq!(authored.slices, ["greeting"]);

    // Two dispatches: the failed answer and its repair. The repair
    // prompt re-presents the failed answer with the findings inlined.
    let requests = provider.model().requests();
    assert_eq!(requests.len(), 2);
    let repair = &requests[1].messages[0].content;
    assert!(repair.contains("Previous answer (failed validation)"), "{repair}");
    assert!(repair.contains(r#"{"version":1,"kind":"response"}"#), "{repair}");
    provider.model().assert_exhausted();
}

#[tokio::test]
async fn unrepairable_exhausts_budget() {
    let provider = ScriptedProvider::scripted("fixture", vec![malformed_answer(); JUDGMENT_BUDGET]);

    let err = author(&provider).await.expect_err("the budget exhausts");
    let detail = err.to_string();
    assert!(
        detail.contains("plan-propose-response-parse"),
        "the last schema failure surfaces: {detail}"
    );

    // One initial dispatch plus MAX_REPAIRS re-prompts, then the leg
    // gives up — no further call.
    assert_eq!(provider.model().requests().len(), JUDGMENT_BUDGET);
    provider.model().assert_exhausted();
}
