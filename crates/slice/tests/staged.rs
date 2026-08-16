//! The staged synthesis tree (RFC-96 D10) through the public refine
//! orchestration: promotion happens only when the staged tree
//! validates, a tree that never validates leaves the slice artifacts
//! untouched, and a repair attempt reuses the same stage.

use std::fs;
use std::path::Path;

use mock::session::Session;
use project::handler::Anchor as _;
use serde_json::json;

fn assessment() -> serde_json::Value {
    json!({
        "behavioural-breadth": 1,
        "coupling": 1,
        "uncertainty": 1,
        "context-volume": 1,
        "verification-surface": 1
    })
}

fn envelope() -> serde_json::Value {
    json!({
        "version": 4,
        "kind": "proceed",
        "slice": "greeting",
        "assessment": assessment()
    })
}

fn model() -> serde_json::Value {
    json!({
        "requirements": [{
            "title": "greeting returns the static string",
            "domain": "greeting",
            "claims": [{ "source": "main", "id": "greeting.behaviour", "kind": "requirement" }],
            "statement": "GET /greeting returns the static string 'hello'.",
            "scenarios": ["A request to /greeting receives 'hello'"]
        }],
        "tasks": [
            { "id": "TASK-001", "text": "Implement the greeting endpoint.", "satisfies": ["REQ-001"] }
        ]
    })
}

fn artifacts() -> serde_json::Value {
    json!({
        "proposal": "# greeting\n\n## Why\n\nThe mock source surfaced it.\n\n## Domains\n\n- greeting — the affected surface\n\n## Non-goals\n\n- Nothing else.\n",
        "design": "# Design\n\nHow the greeting slice lands.\n",
        "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Implement the endpoint (TASK-001)\n",
        "specs": [{ "domain": "greeting", "content": "## greeting\nAgent prose body.\n" }]
    })
}

fn answer(extra: &[(&str, serde_json::Value)]) -> String {
    let mut value = envelope();
    for (key, part) in extra {
        value[*key] = part.clone();
    }
    serde_json::to_string(&value).expect("answer serialises")
}

/// The probe/build fixture shape: one greeting entry over the mock
/// source and target, with the catalog lead retained.
fn seed_greeting_plan(root: &Path) {
    use artifacts::leads::{Lead, Leads};
    use project::adapter::catalog::Pin;
    use project::config::Layout;
    use project::plan::{Entry, Plan, SliceSourceBinding, SourceBinding, TargetBinding};
    use project::snapshot::SnapshotId;

    let layout = Layout::new(root);
    fs::create_dir_all(layout.change_root()).expect("change home");
    let mut plan = Plan::named("demo");
    plan.targets.insert(
        "default".into(),
        TargetBinding::new(
            Pin::parse("emery:mock@0.0.0").expect("target pin"),
            ".",
            SnapshotId::from_digest(&"0".repeat(64)),
        ),
    );
    plan.sources.insert(
        "main".into(),
        SourceBinding::intent(
            Pin::parse("emery:mock@0.0.0").expect("source pin"),
            "The greeting service.",
        ),
    );
    let mut entry = Entry::named("greeting", "default");
    entry.sources = vec![SliceSourceBinding::structured("main", "greeting")];
    plan.entries.push(entry);
    plan.save(&layout.plan_path()).expect("plan.yaml");
    Leads::from_leads(vec![Lead::new("greeting", "main", "greeting")])
        .write_atomic(&layout.leads_path())
        .expect("leads.md");
}

async fn refine(session: &Session) -> Result<slice::orchestrate::RefineOutcome, error::Error> {
    let provider = session.provider();
    let caps = slice::orchestrate::Capabilities::provider(provider);
    let paths = provider.paths();
    let layout = paths.layout();
    let plan = project::plan::Plan::load(&layout.plan_path())?;
    let entry = plan.entries.iter().find(|entry| entry.name == "greeting").expect("plan entry");
    let target = project::target_policy::fresh(provider, paths, entry, "greeting", "refining")?;
    let binding = plan.target(&entry.target)?;
    let adapter =
        project::adapter::Resolver::resolve_target(provider, &binding.adapter.selector(), paths)?;
    slice::orchestrate::refine(
        caps,
        paths,
        jiff::Timestamp::now(),
        "greeting",
        &target,
        Vec::new(),
        &adapter.manifest.inputs,
    )
    .await
}

// A fully staged tree validates and promotes: the bundle lands in the
// slice directory and the slice is `refined`.
#[tokio::test]
async fn staged_tree_promotes() {
    let session =
        Session::scripted("mock", vec![answer(&[("model", model()), ("artifacts", artifacts())])]);
    seed_greeting_plan(session.root());

    let outcome = refine(&session).await.expect("refine promotes the staged tree");
    let slice::orchestrate::RefineOutcome::Refined { artifacts, .. } = outcome else {
        panic!("expected a refined outcome, got {outcome:?}");
    };
    assert!(artifacts.iter().any(|path| path == "model.yaml"), "{artifacts:?}");

    let slice_dir = session.root().join(".emery/change/slices/greeting");
    for rel in ["proposal.md", "design.md", "tasks.md", "model.yaml", "specs/greeting/spec.md"] {
        assert!(slice_dir.join(rel).is_file(), "`{rel}` promoted");
    }
    session.model().assert_exhausted();
}

// A stage that never validates (no model.yaml on any attempt) exhausts
// the repair budget, fails typed, and leaves the slice artifacts
// untouched.
#[tokio::test]
async fn failed_stage_untouched() {
    let incomplete = || answer(&[("artifacts", artifacts())]);
    let session = Session::scripted("mock", vec![incomplete(), incomplete(), incomplete()]);
    seed_greeting_plan(session.root());

    let err = refine(&session).await.expect_err("a model-less stage never validates");
    assert!(
        err.to_string().contains("slice-synthesize-stage-missing"),
        "typed staged-tree failure: {err}"
    );

    let slice_dir = session.root().join(".emery/change/slices/greeting");
    for rel in ["proposal.md", "design.md", "tasks.md", "model.yaml"] {
        assert!(!slice_dir.join(rel).exists(), "`{rel}` must not be promoted");
    }
    session.model().assert_exhausted();
}

// The repair loop reuses the same stage: attempt one stages only the
// model, the repair stages only the prose — promotion needs both, so
// success proves the stage persisted across attempts.
#[tokio::test]
async fn repair_reuses_the_stage() {
    let session = Session::scripted(
        "mock",
        vec![answer(&[("model", model())]), answer(&[("artifacts", artifacts())])],
    );
    seed_greeting_plan(session.root());

    let outcome = refine(&session).await.expect("the repaired stage validates");
    assert!(
        matches!(outcome, slice::orchestrate::RefineOutcome::Refined { .. }),
        "expected a refined outcome, got {outcome:?}"
    );
    let model_yaml =
        fs::read_to_string(session.root().join(".emery/change/slices/greeting/model.yaml"))
            .expect("promoted model.yaml");
    assert!(model_yaml.contains("greeting.behaviour"), "{model_yaml}");
    session.model().assert_exhausted();
}
