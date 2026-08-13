//! End-to-end detached `plan author`: degenerate leaf, multi-level
//! multi-target, repair, overlap, and judgment-budget parking.

use change::plan;
use mock::definition::{Spec, mint};
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::definition::{Scope, Target};
use project::handler::Anchor;
use project::plan::{Decomposition, Plan};
use project::snapshot::SnapshotId;
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

fn code(err: &impl std::fmt::Display) -> String {
    err.to_string()
}

fn pin(digit: u8) -> SnapshotId {
    SnapshotId::from_digest(&format!("{digit:x}").repeat(64))
}

fn quiet() -> serde_json::Value {
    json!({
        "behavioural-breadth": 1,
        "coupling": 1,
        "uncertainty": 1,
        "context-volume": 1,
        "verification-surface": 1
    })
}

fn mint_two_target(root: &std::path::Path, intent: &str) -> (std::path::PathBuf, String) {
    let app = seed_target(root, "app-tree");
    let other = seed_target(root, "other-tree");
    let docs_src = root.join("docs");
    let code_src = root.join("code");
    std::fs::create_dir_all(&docs_src).expect("docs src");
    std::fs::create_dir_all(&code_src).expect("code src");
    std::fs::write(docs_src.join("README.md"), "docs\n").expect("docs file");
    std::fs::write(code_src.join("main.ts"), "code\n").expect("code file");
    let definition = root.join("definition");
    let mut spec = Spec::degenerate(intent);
    spec.targets[0].id = "app".into();
    spec.targets[0].locator = app.display().to_string();
    spec.targets.push(Target {
        id: "other".into(),
        locator: other.display().to_string(),
        adapter: "emery:omnia@1.0.0".into(),
    });
    spec.scopes.push(Scope {
        source: "docs".into(),
        source_cid: Some(pin(0x1)),
        value: None,
        adapter: Some("emery:mock-docs@0.0.0".into()),
        locator: Some(docs_src.display().to_string()),
        lead: "login-flow".into(),
        evidence_digest: pin(0x2),
    });
    spec.scopes.push(Scope {
        source: "code".into(),
        source_cid: Some(pin(0x3)),
        value: None,
        adapter: Some("emery:mock-code@0.0.0".into()),
        locator: Some(code_src.display().to_string()),
        lead: "login-flow".into(),
        evidence_digest: pin(0x4),
    });
    mint(&definition, &spec).expect("mint");
    (definition, spec.wave)
}

fn leaf(slice: &str, source: &str, lead: &str, target: &str) -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "leaf",
        "target": target,
        "slice": slice,
        "ownership": [format!("{slice}/**")],
        "acceptance": format!("{slice} is one acceptance unit."),
        "sources": [{ "source": source, "lead": lead }],
        "assessment": quiet()
    }))
    .expect("leaf")
}

#[tokio::test]
async fn single_leaf_degenerate() {
    let session = Session::scripted("mock", mock::answers::greeting_author());
    let target = seed_target(session.root(), "target-app");
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");

    let body = author(&session, &definition, &spec.wave).await.expect("author");
    assert_eq!(body.slices, vec!["greeting"]);

    let layout = Layout::new(session.root());
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    let tree = Decomposition::load(&layout.decomposition_path()).expect("tree");
    assert_eq!(tree.root, "root");
    assert_eq!(tree.nodes.len(), 2, "root → leaf");
    assert!(tree.nodes["root"].children.contains(&"greeting".into()));
    tree.check().expect("complete tree");
    project::plan::decomposition::matches_plan(&tree, &plan).expect("projection");
    assert_eq!(plan.decomposition_digest.as_ref(), Some(&tree.digest().expect("digest")));

    let change = std::fs::read_to_string(layout.change_brief_path()).expect("change.md");
    assert!(change.starts_with("# Change — demo\n"), "{change}");
    assert!(change.contains("Sources: 1. Leads: 1."), "{change}");
    assert!(change.contains("## Intent"), "{change}");
    session.model().assert_exhausted();
}

fn multi_level_answers() -> Vec<String> {
    vec![
        serde_json::to_string(&json!({
            "version": 1,
            "kind": "split",
            "assessment": quiet(),
            "children": [
                {
                    "id": "auth",
                    "sources": [
                        { "source": "docs", "lead": "login-flow" },
                        { "source": "code", "lead": "login-flow" }
                    ],
                    "target": "app"
                },
                {
                    "id": "greeting",
                    "sources": [{ "source": "intent", "lead": "intent" }],
                    "target": "other"
                }
            ]
        }))
        .expect("root split"),
        serde_json::to_string(&json!({
            "version": 1,
            "kind": "split",
            "assessment": quiet(),
            "children": [
                {
                    "id": "login",
                    "sources": [{ "source": "docs", "lead": "login-flow" }],
                    "target": "app"
                },
                {
                    "id": "token",
                    "sources": [{ "source": "code", "lead": "login-flow" }],
                    "target": "app"
                }
            ]
        }))
        .expect("auth split"),
        leaf("greeting", "intent", "intent", "other"),
        leaf("login", "docs", "login-flow", "app"),
        leaf("token", "code", "login-flow", "app"),
        serde_json::to_string(&json!({
            "version": 1,
            "kind": "response",
            "slices": [
                {
                    "name": "login",
                    "target": "app",
                    "sources": [{ "source": "docs", "lead": "login-flow" }]
                },
                {
                    "name": "token",
                    "target": "app",
                    "sources": [{ "source": "code", "lead": "login-flow" }]
                },
                {
                    "name": "greeting",
                    "target": "other",
                    "sources": [{ "source": "intent", "lead": "intent" }]
                }
            ],
            "gate": { "change": "## Intent\n\nSplit the wave.\n\n## Scope\n\nThree leaves." }
        }))
        .expect("change"),
    ]
}

#[tokio::test]
async fn multi_level_tree() {
    let session = Session::scripted("mock", multi_level_answers());
    let (definition, wave) = mint_two_target(session.root(), "Ship the greeting.");
    let body = author(&session, &definition, &wave).await.expect("author");
    assert_eq!(body.slices.len(), 3, "{:?}", body.slices);

    let layout = Layout::new(session.root());
    let tree = Decomposition::load(&layout.decomposition_path()).expect("tree");
    assert_eq!(tree.depth("login").expect("depth"), 3);
    tree.check().expect("complete");
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    assert!(plan.targets.contains_key("app"));
    assert!(plan.targets.contains_key("other"));
    project::plan::decomposition::matches_plan(&tree, &plan).expect("projection");
}

#[tokio::test]
async fn invalid_split_repairs() {
    let session = Session::scripted(
        "mock",
        vec![
            mock::answers::greeting_leaf_invalid(),
            mock::answers::greeting_leaf(),
            mock::answers::greeting_change(),
        ],
    );
    let target = seed_target(session.root(), "target-app");
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    let body = author(&session, &definition, &spec.wave).await.expect("repaired");
    assert_eq!(body.slices, vec!["greeting"]);
    session.model().assert_exhausted();
}

fn overlap_split() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "split",
        "assessment": quiet(),
        "children": [
            {
                "id": "left",
                "sources": [
                    { "source": "intent", "lead": "intent" },
                    { "source": "docs", "lead": "login-flow" }
                ],
                "target": "app",
                "ownership": ["."]
            },
            {
                "id": "right",
                "sources": [
                    { "source": "intent", "lead": "intent" },
                    { "source": "code", "lead": "login-flow" }
                ],
                "target": "other",
                "ownership": ["."]
            }
        ]
    }))
    .expect("overlap")
}

#[tokio::test]
async fn overlap_blocks() {
    let overlap = overlap_split();
    let session = Session::scripted("mock", vec![overlap.clone(), overlap.clone(), overlap]);
    let (definition, wave) = mint_two_target(session.root(), "Ship the greeting.");
    let err = author(&session, &definition, &wave).await.expect_err("overlap");
    assert!(code(&err).contains("decomposition-overlap"), "{err}");
}

#[tokio::test]
async fn unready_blocks() {
    let session = Session::scripted(
        "mock",
        vec![mock::answers::greeting_leaf_loud(), mock::answers::greeting_unready()],
    );
    let target = seed_target(session.root(), "target-app");
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    let err = author(&session, &definition, &spec.wave).await.expect_err("unready");
    assert!(code(&err).contains("plan-author-unready"), "{err}");
    assert!(!Layout::new(session.root()).decomposition_path().exists());
}

#[tokio::test]
async fn budget_exhaustion_parks() {
    let mut answers = Vec::new();
    for _ in 0..65 {
        answers.push(mock::answers::greeting_leaf_loud());
        answers.push(mock::answers::greeting_focus());
    }
    let session = Session::scripted("mock", answers);
    let target = seed_target(session.root(), "target-app");
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    let err = author(&session, &definition, &spec.wave).await.expect_err("parked");
    assert!(code(&err).contains("plan-author-budget-exhausted"), "{err}");
    let layout = Layout::new(session.root());
    assert!(
        !layout.decomposition_path().exists(),
        "complete-tree policy: exhaustion does not publish"
    );
}

#[tokio::test]
async fn detached_author() {
    let session = Session::detached(mock::answers::greeting_author());
    let target = seed_target(session.root(), "target-app");
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    let body = author(&session, &definition, &spec.wave).await.expect("detached");
    assert_eq!(body.slices, vec!["greeting"]);
    let layout = session.provider().paths().layout();
    assert!(layout.decomposition_path().is_file());
    assert!(layout.change_brief_path().is_file());
    assert!(!session.root().join(".emery/project.yaml").exists());
}
