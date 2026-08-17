//! `plan correct`: durable operator correction — fact-only on a parked
//! author (honored at `plan author` re-entry), fact + inert boundary
//! proposal on an authored plan.

use change::plan;
use change::plan::handlers::{CorrectBody, CorrectInput};
use mock::definition::{Scope, Spec, mint};
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::EventKind;
use project::plan::{Plan, Proposal};
use serde_json::json;

fn seed_target(root: &std::path::Path, name: &str) -> std::path::PathBuf {
    let target = root.join(name);
    std::fs::create_dir_all(target.join(".emery")).expect("target .emery");
    std::fs::write(
        target.join(".emery/project.yaml"),
        format!("name: {name}\nadapter: omnia\nrules: {{}}\n"),
    )
    .expect("project.yaml");
    target
}

async fn author(
    session: &Session, from: &std::path::Path, wave: &str,
) -> Result<plan::handlers::AuthorBody, project::handler::Error> {
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            from: from.to_path_buf(),
            wave: wave.to_string(),
            force: false,
        },
    )
    .await
}

async fn correct(
    session: &Session, input: CorrectInput,
) -> Result<CorrectBody, project::handler::Error> {
    run::<plan::handlers::Correct, _, _>(session.provider(), input).await
}

fn intent_only(intent: &str) -> CorrectInput {
    CorrectInput {
        domain: None,
        constraint: None,
        child: Vec::new(),
        intent: intent.to_string(),
    }
}

fn mint_greeting(session: &Session) -> (std::path::PathBuf, String) {
    let target = seed_target(session.root(), "target-app");
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    (definition, spec.wave)
}

fn live_bytes(layout: Layout<'_>) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    (
        std::fs::read(layout.leads_path()).expect("leads.md"),
        std::fs::read(layout.decomposition_path()).expect("decomposition.yaml"),
        std::fs::read(layout.plan_path()).expect("plan.yaml"),
    )
}

fn assert_live_unchanged(layout: Layout<'_>, before: &(Vec<u8>, Vec<u8>, Vec<u8>)) {
    assert_eq!(std::fs::read(layout.leads_path()).expect("leads.md"), before.0);
    assert_eq!(std::fs::read(layout.decomposition_path()).expect("decomposition.yaml"), before.1);
    assert_eq!(std::fs::read(layout.plan_path()).expect("plan.yaml"), before.2);
}

fn correction_facts(layout: Layout<'_>) -> Vec<(String, Option<String>)> {
    project::journal::read_union(layout)
        .expect("events")
        .into_iter()
        .filter_map(|event| match event.kind {
            EventKind::PlanCorrectionRecorded { domain, proposal, .. } => {
                Some((domain, proposal.map(|digest| digest.to_string())))
            }
            _ => None,
        })
        .collect()
}

// A parked author takes the fact-only path: no model call, no
// proposal, live artifacts byte-identical. Re-entry carries the
// correction into the parked domain's partition request; the
// `close-as-leaf` constraint refuses a split answer in the tail, and
// the repaired cut closes as directed.
#[tokio::test]
async fn parked_then_resume() {
    let session = Session::scripted(
        "mock",
        vec![
            // Park: a loud leaf whose boundary review is unready.
            mock::answers::greeting_leaf_loud(),
            mock::answers::greeting_unready(),
            // ---- resume answers ----
            // A split answer the close-as-leaf constraint refuses.
            mock::answers::greeting_overlap(),
            // The repaired quiet leaf closes without review.
            mock::answers::greeting_leaf(),
            mock::answers::greeting_change(),
        ],
    );
    let (definition, wave) = mint_greeting(&session);
    let err = author(&session, &definition, &wave).await.expect_err("parked");
    assert!(err.to_string().contains("plan-author-stopped"), "{err}");

    let layout = Layout::new(session.root());
    let before = live_bytes(layout);
    let body = correct(
        &session,
        CorrectInput {
            domain: None,
            constraint: Some("close-as-leaf".into()),
            child: Vec::new(),
            intent: "The greeting is one acceptance unit; close it as a single slice.".into(),
        },
    )
    .await
    .expect("fact-only correction");
    assert_eq!(body.status, "recorded");
    assert_eq!(body.domain, "root", "the sole parked domain resolves implicitly");
    assert!(body.proposal.is_none());
    assert_live_unchanged(layout, &before);
    assert!(Proposal::load_all(layout).expect("proposals").is_empty(), "no proposal on a park");
    assert_eq!(correction_facts(layout), [("root".to_string(), None)]);

    // Status: parked, with author as the resume verb.
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    let status = project::plan::plan_status_body(&plan, layout).expect("status");
    let status_json = serde_json::to_string(&status).expect("status json");
    assert!(status_json.contains("partition-parked"), "{status_json}");
    assert!(status_json.contains("emery plan author"), "{status_json}");

    // Re-entry honors the correction.
    let body = author(&session, &definition, &wave).await.expect("resume completes");
    assert_eq!(body.slices, vec!["greeting"]);
    session.model().assert_exhausted();

    let requests = session.model().requests();
    let prompts: Vec<&str> =
        requests.iter().map(|request| request.messages[0].content.as_str()).collect();
    assert!(
        prompts.iter().any(|prompt| prompt.contains("\"corrections\"")
            && prompt.contains("close it as a single slice")),
        "the partition request carries the correction"
    );
    assert!(
        prompts
            .iter()
            .any(|prompt| prompt.contains("the correction requires `root` to close as a leaf")),
        "the tail refused the split answer and the repair prompt names the constraint"
    );
}

// An authored plan takes the escalate-shaped path: the fact plus an
// inert boundary proposal; live artifacts stay unchanged until
// `plan amend --proposal` applies it, and author re-entry stays a
// read-only no-op afterwards.
#[tokio::test]
async fn authored_mints_proposal() {
    let mut answers = mock::answers::greeting_author();
    // The correction re-cut: the reopened domain closes as the same leaf.
    answers.push(mock::answers::greeting_leaf());
    let session = Session::scripted("mock", answers);
    let (definition, wave) = mint_greeting(&session);
    author(&session, &definition, &wave).await.expect("author");

    let layout = Layout::new(session.root());
    let before = live_bytes(layout);
    let mut input = intent_only("Keep the greeting one slice.");
    input.domain = Some("greeting".into());
    let body = correct(&session, input).await.expect("proposal minted");
    assert_eq!(body.status, "proposed");
    assert_eq!(body.domain, "greeting", "a node id resolves verbatim");
    let digest = body.proposal.expect("proposal digest");
    assert_live_unchanged(layout, &before);

    let proposals = Proposal::load_all(layout).expect("proposals");
    assert_eq!(proposals.len(), 1);
    let Proposal::Boundary(boundary) = &proposals[0].1 else {
        panic!("expected boundary proposal");
    };
    assert_eq!(boundary.failed_leaf.as_str(), "greeting");
    assert_eq!(boundary.rationale, "Keep the greeting one slice.");
    assert_eq!(correction_facts(layout), [("greeting".to_string(), Some(digest.clone()))]);

    // Apply, then author re-entry stays a read-only no-op.
    run::<plan::handlers::Amend, _, _>(
        session.provider(),
        plan::handlers::AmendInput {
            name: None,
            proposal: Some(digest),
            depends_on: None,
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
    .expect("amend applies the proposal");
    let body = author(&session, &definition, &wave).await.expect("no-op re-entry");
    assert_eq!(body.slices, vec!["greeting"]);
    session.model().assert_exhausted();
}

// A correction-driven re-cut that uncovers a parent lead refuses with
// `plan-correction-non-reducing` and never mutates the live tree.
#[tokio::test]
async fn uncovering_recut_refuses() {
    fn all_sources_leaf() -> String {
        serde_json::to_string(&json!({
            "version": 1,
            "kind": "leaf",
            "target": "app",
            "slice": "greeting",
            "ownership": ["."],
            "acceptance": "One acceptance unit over intent, docs, and code.",
            "sources": [
                { "source": "intent", "lead": "intent" },
                { "source": "docs", "lead": "login-flow" },
                { "source": "code", "lead": "login-flow" }
            ],
            "assessment": {
                "behavioural-breadth": 1,
                "coupling": 1,
                "uncertainty": 1,
                "context-volume": 1,
                "verification-surface": 1
            }
        }))
        .expect("leaf")
    }
    // A split covering docs and code but dropping the intent lead —
    // `decomposition-lead-uncovered` on every trial.
    fn uncovering_split() -> String {
        serde_json::to_string(&json!({
            "version": 1,
            "kind": "split",
            "assessment": {
                "behavioural-breadth": 1,
                "coupling": 1,
                "uncertainty": 1,
                "context-volume": 1,
                "verification-surface": 1
            },
            "children": [
                {
                    "id": "docs-side",
                    "sources": [{ "source": "docs", "lead": "login-flow" }],
                    "target": "app",
                    "ownership": ["docs-side/**"]
                },
                {
                    "id": "code-side",
                    "sources": [{ "source": "code", "lead": "login-flow" }],
                    "target": "app",
                    "ownership": ["code-side/**"]
                }
            ]
        }))
        .expect("split")
    }
    let reconcile = serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": [{
            "name": "greeting",
            "target": "app",
            "sources": [
                { "source": "intent", "lead": "intent" },
                { "source": "docs", "lead": "login-flow" },
                { "source": "code", "lead": "login-flow" }
            ]
        }],
        "gate": { "change": "## Intent\n\nOne slice.\n\n## Scope\n\nAll three leads." }
    }))
    .expect("response");

    let session = Session::scripted(
        "mock",
        vec![
            all_sources_leaf(),
            reconcile,
            // The correction re-cut: initial + MAX_REPAIRS, all uncovering.
            uncovering_split(),
            uncovering_split(),
            uncovering_split(),
        ],
    );
    let target = seed_target(session.root(), "target-app");
    let docs_src = session.root().join("docs");
    let code_src = session.root().join("code");
    std::fs::create_dir_all(&docs_src).expect("docs src");
    std::fs::create_dir_all(&code_src).expect("code src");
    std::fs::write(docs_src.join("README.md"), "docs\n").expect("docs file");
    std::fs::write(code_src.join("main.ts"), "code\n").expect("code file");
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    spec.scopes.push(Scope {
        source: "docs".into(),
        adapter: "emery:mock-docs@0.0.0".into(),
        location: docs_src.display().to_string(),
        lead: "login-flow".into(),
        value: None,
    });
    spec.scopes.push(Scope {
        source: "code".into(),
        adapter: "emery:mock-code@0.0.0".into(),
        location: code_src.display().to_string(),
        lead: "login-flow".into(),
        value: None,
    });
    mint(&definition, &spec).expect("mint");
    author(&session, &definition, &spec.wave).await.expect("author");

    let layout = Layout::new(session.root());
    let before = live_bytes(layout);
    let mut input = intent_only("Split docs and code delivery.");
    input.domain = Some("greeting".into());
    let err = correct(&session, input).await.expect_err("uncovering re-cut refuses");
    assert!(err.to_string().contains("plan-correction-non-reducing"), "{err}");
    assert_live_unchanged(layout, &before);
    assert!(correction_facts(layout).is_empty(), "no fact on a refused re-cut");
    session.model().assert_exhausted();
}

// `--domain` omitted refuses when nothing is parked; unknown domains
// and incoherent constraints are typed refusals.
#[tokio::test]
async fn typed_refusals() {
    // Bound-not-authored with no park fact: a fatal first author.
    let mut answers = vec!["garbage".to_string(), "garbage".to_string(), "garbage".to_string()];
    answers.extend(mock::answers::greeting_author());
    let session = Session::scripted("mock", answers);
    let (definition, wave) = mint_greeting(&session);
    author(&session, &definition, &wave).await.expect_err("unparseable is fatal");

    let err =
        correct(&session, intent_only("Fix it.")).await.expect_err("no parked domain to resolve");
    assert!(err.to_string().contains("plan-correct-domain-required"), "{err}");

    let mut unknown = intent_only("Fix it.");
    unknown.domain = Some("no-such-domain".into());
    // No tree exists yet, so the explicit domain is accepted verbatim
    // on the record path; empty intent and dangling children refuse.
    let err = correct(&session, intent_only("   ")).await.expect_err("empty intent refuses");
    assert!(err.to_string().contains("intent"), "{err}");
    let mut dangling = intent_only("Fix it.");
    dangling.domain = Some("root".into());
    dangling.child = vec!["a".into()];
    let err = correct(&session, dangling).await.expect_err("children need a split constraint");
    assert!(err.to_string().contains("split"), "{err}");
    let mut bad = intent_only("Fix it.");
    bad.constraint = Some("merge".into());
    let err = correct(&session, bad).await.expect_err("unknown constraint refuses");
    assert!(err.to_string().contains("close-as-leaf"), "{err}");

    // Authoring completes; an omitted domain now refuses on the
    // authored path, and an unknown domain is typed.
    author(&session, &definition, &wave).await.expect("re-entry authors");
    let err = correct(&session, intent_only("Split the greeting."))
        .await
        .expect_err("authored path requires a domain");
    assert!(err.to_string().contains("plan-correct-domain-required"), "{err}");
    let mut unknown = intent_only("Split the greeting.");
    unknown.domain = Some("no-such-domain".into());
    let err = correct(&session, unknown).await.expect_err("unknown domain refuses");
    assert!(err.to_string().contains("decomposition-node-unknown"), "{err}");
    session.model().assert_exhausted();
}
