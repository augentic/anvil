//! Typed adapter failures crossing the seam: each mock failure
//! profile surfaces at the public operation boundary — an author
//! abort, a parked refine or build in the execute loop, or the
//! outputs-exist gate — with the adapter's typed detail preserved.

mod support;

use change::plan;
use mock::invoke::run;
use mock::session::Session;

async fn author(
    session: &Session, source_adapter: &str,
) -> Result<plan::handlers::AuthorBody, project::handler::Error> {
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding_for(source_adapter),
            intent: None,
            force: false,
        },
    )
    .await
}

async fn execute_err(session: &Session) -> String {
    run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect_err("the failing phase parks the loop")
    .to_string()
}

#[tokio::test]
async fn survey_failure_aborts_author() {
    let session = Session::scripted("mock", Vec::new());

    let err = author(&session, "mock-fail-survey").await.expect_err("survey fails");
    let detail = err.to_string();
    assert!(detail.contains("survey"), "{detail}");
    assert!(detail.contains("mock survey failure"), "typed detail preserved: {detail}");
    // The failure aborted before any judgment dispatch: the script is
    // empty, so a dispatch would have surfaced `model script exhausted`
    // instead of the survey failure above.
}

#[tokio::test]
async fn unknown_fails_before_scaffold() {
    let session = Session::scripted("mock", Vec::new());

    // Ensure runs over every binding before the plan scaffold write, so
    // an unlinked adapter aborts with nothing on disk — not a seam
    // failure halfway through the survey fan-out.
    let err = author(&session, "no-such-adapter").await.expect_err("ensure fails");
    let detail = err.to_string();
    assert!(detail.contains("adapter-not-linked"), "{detail}");
    assert!(!session.root().join("plan.yaml").exists(), "nothing scaffolded");
}

#[tokio::test]
async fn mismatched_pin_refused() {
    let session = Session::scripted("mock", Vec::new());

    // `<name>@<semver>` parses into the first-party package pin
    // (implicit `emery` namespace); native ensure succeeds only on
    // the exact compiled identity, so a mismatched pin refuses before
    // survey.
    let err = author(&session, "mock@1.0.0").await.expect_err("ensure refuses the pin");
    let detail = err.to_string();
    assert!(detail.contains("adapter-not-linked"), "{detail}");
    assert!(detail.contains("emery:mock@1.0.0"), "pin parsed into the package form: {detail}");
    assert!(!session.root().join("plan.yaml").exists(), "nothing scaffolded");
}

#[tokio::test]
async fn placeholder_pin_refused() {
    let session = Session::scripted("mock", Vec::new());

    // The mock source compiles with the `0.0.0` development
    // placeholder, which remains a bare-only identity: even the
    // matching "exact" pin refuses before survey. (Exact-pin success
    // against a published identity is covered by the native provider
    // suite.)
    let err = author(&session, "mock@0.0.0").await.expect_err("ensure refuses the pin");
    let detail = err.to_string();
    assert!(detail.contains("adapter-not-linked"), "{detail}");
    assert!(detail.contains("placeholder"), "{detail}");
    assert!(!session.root().join("plan.yaml").exists(), "nothing scaffolded");
}

#[tokio::test]
async fn survey_ensures_pinned_binding() {
    let session = Session::scripted("mock", Vec::new());

    // A pinned binding already on disk (the plan schema has carried
    // `version` all along) must be ensured at dispatch time too — the
    // breakout `source survey` path, not just plan author.
    let mut bindings = std::collections::BTreeMap::new();
    bindings.insert(
        "main".to_string(),
        project::plan::SourceBinding {
            adapter: "mock".to_string(),
            version: Some(semver::Version::new(1, 0, 0)),
            path: None,
            value: Some("The greeting service.".to_string()),
            cid: None,
        },
    );
    let plan_path = session.root().join("plan.yaml");
    project::plan::scaffold(&plan_path, "demo", bindings, false)
        .expect("scaffold")
        .save(&plan_path)
        .expect("save plan.yaml");

    let err = run::<change::source::Survey, _, _>(
        session.provider(),
        change::source::SurveyInput {
            source: "main".to_string(),
            plan: None,
        },
    )
    .await
    .expect_err("ensure refuses the pin before dispatch");
    let detail = err.to_string();
    assert!(detail.contains("adapter-not-linked"), "{detail}");
    assert!(detail.contains("emery:mock@1.0.0"), "{detail}");
}

#[tokio::test]
async fn malformed_token_parse_refused() {
    let session = Session::scripted("mock", Vec::new());

    // A non-semver `@` suffix is neither a bare name nor a first-party
    // pin, so the wire parse refuses it before ensure or scaffold.
    let err = author(&session, "mock@not-semver").await.expect_err("parse refuses the token");
    let detail = err.to_string();
    assert!(detail.contains("plan-source-adapter-invalid"), "{detail}");
    assert!(!session.root().join("plan.yaml").exists(), "nothing scaffolded");
}

mod bare_bindings {
    use super::*;

    #[tokio::test]
    async fn bare_binding_stays_unversioned() {
        // Bindings persist as typed: a bare name carries no version
        // into `plan.yaml` — the deployment resolves it local-first
        // on every dispatch, and only an explicit `name@semver` pin
        // stamps `version` (at the wire parse, covered by the pin
        // tests above).
        let session = Session::scripted("mock", vec![mock::answers::greeting_grouping()]);

        author(&session, "mock").await.expect("author succeeds over the bare binding");

        let plan = project::plan::Plan::load(&session.root().join("plan.yaml")).expect("plan.yaml");
        assert_eq!(plan.sources["main"].version, None, "the bare binding stays unversioned");
    }

    #[tokio::test]
    async fn bare_bindings_stay_unversioned() {
        // Uniform over the desugared binding map: neither bare
        // binding gains a version.
        let session = Session::scripted("mock", vec![mock::answers::adversarial_grouping()]);

        run::<plan::handlers::Author, _, _>(
            session.provider(),
            plan::handlers::AuthorInput {
                name: "demo".to_string(),
                sources: support::adversarial_bindings(),
                intent: None,
                force: false,
            },
        )
        .await
        .expect("author succeeds over the bare bindings");

        let plan = project::plan::Plan::load(&session.root().join("plan.yaml")).expect("plan.yaml");
        for key in ["docs", "code"] {
            assert_eq!(plan.sources[key].version, None, "binding `{key}` stays unversioned");
        }
    }

    #[tokio::test]
    async fn intent_sugar_unlinked_fails() {
        // `--intent` desugars to a bare `intent` binding ensured
        // before the scaffold write; the native catalog links no
        // `intent` adapter, so ensure refuses with nothing on disk.
        let session = Session::scripted("mock", Vec::new());

        let err = run::<plan::handlers::Author, _, _>(
            session.provider(),
            plan::handlers::AuthorInput {
                name: "demo".to_string(),
                sources: Vec::new(),
                intent: Some("Ship the greeting.".to_string()),
                force: false,
            },
        )
        .await
        .expect_err("the bare intent binding is not in the native catalog");
        let detail = err.to_string();
        assert!(detail.contains("adapter-not-linked"), "{detail}");
        assert!(detail.contains("intent"), "{detail}");
        assert!(!session.root().join("plan.yaml").exists(), "nothing scaffolded");
    }
}

#[tokio::test]
async fn extract_failure_parks_refine() {
    let session = Session::scripted("mock", vec![mock::answers::greeting_grouping()]);

    author(&session, "mock-fail-extract").await.expect("survey succeeds for this profile");
    let detail = execute_err(&session).await;
    assert!(detail.contains("refine-failed"), "{detail}");
    assert!(detail.contains("mock extract failure"), "typed detail preserved: {detail}");
}

#[tokio::test]
async fn guidance_failure_parks_refine() {
    let session = Session::scripted("mock-fail-guidance", vec![mock::answers::greeting_grouping()]);

    author(&session, "mock").await.expect("author succeeds");
    let detail = execute_err(&session).await;
    assert!(detail.contains("refine-failed"), "{detail}");
    assert!(detail.contains("mock guidance failure"), "typed detail preserved: {detail}");
}

#[tokio::test]
async fn build_failure_parks() {
    let session = Session::scripted(
        "mock-fail-build",
        vec![mock::answers::greeting_grouping(), mock::answers::greeting_synthesis()],
    );

    author(&session, "mock").await.expect("author succeeds");
    let detail = execute_err(&session).await;
    assert!(detail.contains("build-failed"), "{detail}");
    assert!(detail.contains("mock build failure"), "typed detail preserved: {detail}");
}

#[tokio::test]
async fn merge_failure_parks_built() {
    let session = Session::scripted(
        "mock-fail-merge",
        vec![mock::answers::greeting_grouping(), mock::answers::greeting_synthesis()],
    );

    author(&session, "mock").await.expect("author succeeds");
    // Refine and build succeed; the merge preflight dispatch itself
    // errors (a typed seam failure, not a failed gate report), so the
    // loop parks with the deterministic commit never attempted.
    let detail = execute_err(&session).await;
    assert!(detail.contains("seam-dispatch-failed"), "{detail}");
    assert!(detail.contains("mock merge failure"), "typed detail preserved: {detail}");

    let metadata =
        std::fs::read_to_string(session.root().join(".emery/slices/greeting/metadata.yaml"))
            .expect("slice still present");
    assert!(metadata.contains("completed-at:"), "no commit happened:\n{metadata}");
    assert!(
        session.root().join(".emery/slices/greeting/build/patch.yaml").is_file(),
        "build record must remain when merge parks"
    );
    assert!(!session.root().join(".emery/specs/greeting/spec.md").exists(), "no baseline write");
}

#[tokio::test]
async fn missing_output_aborts() {
    let session = Session::scripted(
        "mock-missing-output",
        vec![mock::answers::greeting_grouping(), mock::answers::greeting_synthesis()],
    );

    author(&session, "mock").await.expect("author succeeds");
    // The mock reports success but never writes its declared
    // output, so the orchestrator's outputs-exist gate aborts.
    let detail = execute_err(&session).await;
    assert!(detail.contains("target-build-output-missing"), "{detail}");
}
