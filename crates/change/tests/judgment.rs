//! The judgment repair loop through the public authoring surface: a
//! malformed reconciliation answer is repaired in-loop with the
//! findings inlined, and an unrepairable answer exhausts the budget
//! and surfaces the last failure. The loop only engages when the model
//! emits schema-violating answers, which the unvalidated script serves
//! verbatim.

mod support;

use change::plan;
use fixture::invoke::run;
use fixture::session::Session;

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

async fn author(session: &Session) -> Result<plan::handlers::AuthorBody, project::handler::Error> {
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding(),
            intent: None,
        },
    )
    .await
}

#[tokio::test]
async fn malformed_repaired_in_loop() {
    let session = Session::scripted(
        "fixture",
        vec![malformed_answer(), fixture::answers::greeting_grouping()],
    );

    let authored = author(&session).await.expect("the repaired answer lands");
    assert_eq!(authored.slices, ["greeting"]);

    // Two dispatches — the failed answer and its repair — drained the
    // two-answer script exactly.
    session.model().assert_exhausted();
}

#[tokio::test]
async fn unrepairable_exhausts_budget() {
    let session = Session::scripted("fixture", vec![malformed_answer(); JUDGMENT_BUDGET]);

    let err = author(&session).await.expect_err("the budget exhausts");
    let detail = err.to_string();
    assert!(
        detail.contains("plan-propose-response-parse"),
        "the last schema failure surfaces: {detail}"
    );

    // One initial dispatch plus MAX_REPAIRS re-prompts drained the
    // budget-sized script, then the leg gave up — no further call.
    session.model().assert_exhausted();
}
