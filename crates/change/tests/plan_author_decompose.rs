//! End-to-end detached `plan author`: degenerate leaf, multi-level
//! multi-target, repair, overlap, and judgment-budget parking.

use change::plan;
use mock::definition::{Scope, Spec, Target, mint};
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::handler::Anchor;
use project::plan::{Decomposition, Plan};
use serde_json::json;

#[expect(unsafe_code, reason = "EMERY_POOL is the launcher cap seam; nextest isolates the process")]
fn set_cap(cap: &str) {
    // SAFETY: nextest runs each test in its own process, and the env
    // write happens before any pool dispatch reads the cap.
    unsafe { std::env::set_var("EMERY_POOL", cap) };
}

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

// A boundary review that answers unready parks that domain instead of
// aborting: the partial tree persists, no reconcile fact is written,
// and the typed `plan-author-stopped` halt names the domain.
#[tokio::test]
async fn unready_parks() {
    let session = Session::scripted(
        "mock",
        vec![mock::answers::greeting_leaf_loud(), mock::answers::greeting_unready()],
    );
    let target = seed_target(session.root(), "target-app");
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    let err = author(&session, &definition, &spec.wave).await.expect_err("parked");
    assert!(code(&err).contains("plan-author-stopped"), "{err}");
    assert!(code(&err).contains("plan-author-unready"), "{err}");

    let layout = Layout::new(session.root());
    assert!(layout.decomposition_path().exists(), "partial tree persists on park");
    let plan = Plan::load(&layout.plan_path()).expect("bound stub");
    assert!(plan.entries.is_empty(), "{:?}", plan.entries);
    assert!(plan.decomposition_digest.is_none(), "no digest until reconcile");
    let events = project::journal::read_union(layout).expect("events");
    assert!(
        events.iter().any(|event| matches!(
            &event.kind,
            project::journal::EventKind::PlanAuthorParked { .. }
        )),
        "park fact recorded"
    );
    assert!(
        !events.iter().any(|event| matches!(
            &event.kind,
            project::journal::EventKind::PlanReconcileCompleted { .. }
        )),
        "no reconcile fact on a park"
    );
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

/// Root split into `auth` (app: docs + code) and `greeting` (other:
/// intent). `auth` carries the ownership and single target the
/// fallback leaf needs.
fn root_two_domain_split() -> String {
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
                "target": "app",
                "ownership": ["auth/**"]
            },
            {
                "id": "greeting",
                "sources": [{ "source": "intent", "lead": "intent" }],
                "target": "other"
            }
        ]
    }))
    .expect("root split")
}

/// `auth` splits into `login` (both leads — will tie) and `token`.
fn auth_split() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "split",
        "assessment": quiet(),
        "children": [
            {
                "id": "login",
                "sources": [
                    { "source": "docs", "lead": "login-flow" },
                    { "source": "code", "lead": "login-flow" }
                ],
                "target": "app",
                "ownership": ["auth/login/**"]
            },
            {
                "id": "token",
                "sources": [{ "source": "code", "lead": "login-flow" }],
                "target": "app",
                "ownership": ["auth/token/**"]
            }
        ]
    }))
    .expect("auth split")
}

/// A `login` leaf close carrying `auth`'s full lead set — it ties the
/// reduction measure against its parent on every dimension.
fn tie_leaf() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "leaf",
        "target": "app",
        "slice": "login",
        "ownership": ["auth/login/**"],
        "acceptance": "login is one acceptance unit.",
        "sources": [
            { "source": "docs", "lead": "login-flow" },
            { "source": "code", "lead": "login-flow" }
        ],
        "assessment": quiet()
    }))
    .expect("tie leaf")
}

fn recovered_response(slices: &[(&str, &str, &str, &str)]) -> String {
    let rows: Vec<serde_json::Value> = slices
        .iter()
        .map(|(name, target, source, lead)| {
            json!({
                "name": name,
                "target": target,
                "sources": [{ "source": source, "lead": lead }]
            })
        })
        .collect();
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": rows,
        "gate": { "change": "## Intent\n\nRecovered.\n\n## Scope\n\nThe surviving leaves." }
    }))
    .expect("response")
}

// A reduction tie on an open child defers (Decision 1); the same tie
// on the closed child after repairs closes the PARENT as a leaf via
// the deterministic fallback: siblings are subsumed, the tree stays,
// the `domain.partition.closed` fact and the change.md caveat record
// the disposition, and authoring completes with a reconcile fact.
#[tokio::test]
async fn non_reducing_fallback_closes_parent() {
    let final_answer = serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": [
            {
                "name": "auth",
                "target": "app",
                "sources": [
                    { "source": "docs", "lead": "login-flow" },
                    { "source": "code", "lead": "login-flow" }
                ]
            },
            {
                "name": "greeting",
                "target": "other",
                "sources": [{ "source": "intent", "lead": "intent" }]
            }
        ],
        "gate": { "change": "## Intent\n\nRecovered.\n\n## Scope\n\nTwo leaves." }
    }))
    .expect("response");
    let session = Session::scripted(
        "mock",
        vec![
            root_two_domain_split(),
            // `auth` splits fine — the tie is deferred while `login`
            // and `token` are open (Decision 1).
            auth_split(),
            leaf("greeting", "intent", "intent", "other"),
            // `login`'s close ties against `auth` on every dimension:
            // initial answer + MAX_REPAIRS identical repairs.
            tie_leaf(),
            tie_leaf(),
            tie_leaf(),
            leaf("token", "code", "login-flow", "app"),
            final_answer,
        ],
    );
    let (definition, wave) = mint_two_target(session.root(), "Ship the greeting.");
    let body = author(&session, &definition, &wave).await.expect("fallback close completes");
    let mut slices = body.slices.clone();
    slices.sort();
    assert_eq!(slices, ["auth", "greeting"], "{:?}", body.slices);

    let layout = Layout::new(session.root());
    let tree = Decomposition::load(&layout.decomposition_path()).expect("tree");
    tree.check().expect("complete tree");
    assert_eq!(tree.nodes["auth"].kind, Some(project::plan::decomposition::Kind::Leaf));
    assert!(!tree.nodes.contains_key("login"), "subsumed child pruned");

    let events = project::journal::read_union(layout).expect("events");
    assert!(
        events.iter().any(|event| matches!(
            &event.kind,
            project::journal::EventKind::DomainPartitionClosed {
                domain,
                reason: project::journal::ClosedReason::NonReducingFallback,
                ..
            } if domain == "auth"
        )),
        "closed-domain fact recorded"
    );
    assert!(
        events.iter().any(|event| matches!(
            &event.kind,
            project::journal::EventKind::PlanReconcileCompleted { .. }
        )),
        "authoring completed"
    );
    let change = std::fs::read_to_string(layout.change_brief_path()).expect("change.md");
    assert!(change.contains("closed as a leaf after a failed cut"), "{change}");
    session.model().assert_exhausted();
}

/// An `auth` cut that keeps overlapping without an order: repair
/// exhaustion on a domain that cannot close as a leaf (two targets).
fn overlapping_auth_split() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "split",
        "assessment": quiet(),
        "children": [
            {
                "id": "left",
                "sources": [{ "source": "docs", "lead": "login-flow" }],
                "target": "app",
                "ownership": ["."]
            },
            {
                "id": "right",
                "sources": [{ "source": "code", "lead": "login-flow" }],
                "target": "other",
                "ownership": ["."]
            }
        ]
    }))
    .expect("overlap split")
}

// A domain that cannot close as a leaf parks; independent domains
// still partition, closed leaves project into `plan.yaml`, topology
// verbs refuse `plan-author-incomplete`, status projects
// `partition-parked`, and `plan author` re-entry (no `--force`)
// resumes only the parked domain and completes.
#[tokio::test]
async fn park_then_resume() {
    // The scripted answer order assumes sibling domains partition in
    // one concurrent round ("independent domains keep draining"); the
    // Phase 0 default cap is serial, where the sibling would instead
    // park as never-run after the first failure.
    set_cap("4");
    let session = Session::scripted(
        "mock",
        vec![
            // Root split: `auth` spans BOTH targets, so the fallback
            // leaf is impossible.
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
                        "targets": ["app", "other"],
                        "ownership": ["auth/**"]
                    },
                    {
                        "id": "greeting",
                        "sources": [{ "source": "intent", "lead": "intent" }],
                        "target": "app"
                    }
                ]
            }))
            .expect("root split"),
            // `auth`'s cut keeps overlapping: initial + MAX_REPAIRS.
            overlapping_auth_split(),
            overlapping_auth_split(),
            overlapping_auth_split(),
            // The independent sibling still closes.
            leaf("greeting", "intent", "intent", "app"),
            // ---- resume answers ----
            serde_json::to_string(&json!({
                "version": 1,
                "kind": "split",
                "assessment": quiet(),
                "children": [
                    {
                        "id": "b-login",
                        "sources": [{ "source": "docs", "lead": "login-flow" }],
                        "target": "app",
                        "ownership": ["auth/login/**"]
                    },
                    {
                        "id": "b-token",
                        "sources": [{ "source": "code", "lead": "login-flow" }],
                        "target": "other",
                        "ownership": ["auth/token/**"]
                    }
                ]
            }))
            .expect("resume split"),
            leaf("b-login", "docs", "login-flow", "app"),
            leaf("b-token", "code", "login-flow", "other"),
            recovered_response(&[
                ("greeting", "app", "intent", "intent"),
                ("b-login", "app", "docs", "login-flow"),
                ("b-token", "other", "code", "login-flow"),
            ]),
        ],
    );
    let (definition, wave) = mint_two_target(session.root(), "Ship the greeting.");
    let err = author(&session, &definition, &wave).await.expect_err("parked");
    assert!(code(&err).contains("plan-author-stopped"), "{err}");
    assert!(code(&err).contains("`auth`"), "the stop names the parked domain: {err}");

    let layout = Layout::new(session.root());
    let tree = Decomposition::load(&layout.decomposition_path()).expect("partial tree persists");
    assert!(tree.nodes["auth"].kind.is_none(), "parked domain stays open");
    assert_eq!(tree.nodes["greeting"].kind, Some(project::plan::decomposition::Kind::Leaf));
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    let named: Vec<&str> = plan.entries.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(named, ["greeting"], "closed leaves project into plan.yaml");
    assert!(plan.decomposition_digest.is_none(), "no digest until reconcile");
    let events = project::journal::read_union(layout).expect("events");
    assert!(
        events.iter().any(|event| matches!(
            &event.kind,
            project::journal::EventKind::PlanAuthorParked { domain, .. } if domain == "auth"
        )),
        "park fact recorded"
    );

    // Reader guards: topology verbs refuse until authoring completes.
    let gaps_err =
        run::<plan::handlers::Gaps, _, _>(session.provider(), plan::handlers::GapsInput::default())
            .await
            .expect_err("gaps refuses on a bound-not-authored home");
    assert!(gaps_err.to_string().contains("plan-author-incomplete"), "{gaps_err}");

    // Status: parked, never drained; author is the resume path.
    let status = project::plan::plan_status_body(&plan, layout).expect("status");
    let status_json = serde_json::to_string(&status).expect("status json");
    assert!(status_json.contains("partition-parked"), "{status_json}");
    assert!(!status_json.contains("\"drained\""), "{status_json}");

    // Re-entry (no --force) resumes only the parked domain.
    let body = author(&session, &definition, &wave).await.expect("resume completes");
    let mut slices = body.slices.clone();
    slices.sort();
    assert_eq!(slices, ["b-login", "b-token", "greeting"], "{:?}", body.slices);
    let tree = Decomposition::load(&layout.decomposition_path()).expect("tree");
    tree.check().expect("complete tree");
    let events = project::journal::read_union(layout).expect("events");
    assert!(
        events.iter().any(|event| matches!(
            &event.kind,
            project::journal::EventKind::PlanReconcileCompleted { .. }
        )),
        "resume reconciles"
    );
    session.model().assert_exhausted();
}

// A fatal judgment failure before any cut leaves a bound stub: no
// decomposition.yaml, status `stop author-incomplete` (never
// `drained`), and `plan author` re-entry continues the decompose
// without `--force`.
#[tokio::test]
async fn bound_stub_resumes_decompose() {
    let mut answers = vec!["garbage".to_string(), "garbage".to_string(), "garbage".to_string()];
    answers.extend(mock::answers::greeting_author());
    let session = Session::scripted("mock", answers);
    let target = seed_target(session.root(), "target-app");
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");

    author(&session, &definition, &spec.wave).await.expect_err("unparseable is fatal");
    let layout = Layout::new(session.root());
    let plan = Plan::load(&layout.plan_path()).expect("bound stub survives");
    assert!(plan.entries.is_empty());
    assert!(!layout.decomposition_path().exists(), "no cut was ever applied");
    let status = project::plan::plan_status_body(&plan, layout).expect("status");
    let status_json = serde_json::to_string(&status).expect("status json");
    assert!(status_json.contains("author-incomplete"), "{status_json}");
    assert!(!status_json.contains("\"drained\""), "{status_json}");

    let body = author(&session, &definition, &spec.wave).await.expect("re-entry decomposes");
    assert_eq!(body.slices, vec!["greeting"]);
    session.model().assert_exhausted();
}

// A violation that first surfaces on the complete tree routes through
// the reopen ladder: the offending parent is re-judged once with the
// findings inlined instead of aborting. Staged by persisting a
// complete-but-violating tree under a bound stub and resuming.
#[tokio::test]
async fn final_check_reopens() {
    use project::plan::decomposition::{BoundProfile, Node, Scope, VERSION};
    use project::profile::Profiles as _;

    let mut answers = vec!["garbage".to_string(), "garbage".to_string(), "garbage".to_string()];
    answers.extend(mock::answers::greeting_author());
    let session = Session::scripted("mock", answers);
    let target = seed_target(session.root(), "target-app");
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    author(&session, &definition, &spec.wave).await.expect_err("fatal leaves a bound stub");

    // Two sibling leaves tie the reduction measure and overlap in
    // ownership without an order — invalid only as a complete tree.
    let layout = Layout::new(session.root());
    let plan = Plan::load(&layout.plan_path()).expect("bound stub");
    let pin = plan.targets["app"].model_capability_profile.as_ref().expect("pin");
    let bound = BoundProfile::capture(session.provider().profiles().pinned(pin).expect("profile"))
        .expect("capture");
    let scope = Scope::new("intent", "intent");
    let mut root = Node::split(vec!["left".to_string(), "right".to_string()]);
    root.sources = vec![scope.clone()];
    root.targets = vec!["app".to_string()];
    let violating_leaf = |slice: &str| {
        let mut node = Node::leaf("app", slice);
        node.parent = Some("root".to_string());
        node.sources = vec![scope.clone()];
        node.ownership = vec!["same/**".to_string()];
        node.acceptance = Some(format!("{slice} is one acceptance unit."));
        node
    };
    let tree = Decomposition {
        version: VERSION,
        leads_digest: plan.leads_digest.clone().expect("leads digest"),
        profiles: std::collections::BTreeMap::from([("app".to_string(), bound)]),
        root: "root".to_string(),
        nodes: std::collections::BTreeMap::from([
            ("root".to_string(), root),
            ("left".to_string(), violating_leaf("left")),
            ("right".to_string(), violating_leaf("right")),
        ]),
    };
    tree.check().expect_err("the crafted tree violates on purpose");
    tree.save(&layout.decomposition_path()).expect("persist crafted tree");

    let body = author(&session, &definition, &spec.wave).await.expect("reopen re-judgment");
    assert_eq!(body.slices, vec!["greeting"]);
    let tree = Decomposition::load(&layout.decomposition_path()).expect("tree");
    tree.check().expect("complete tree");
    assert!(!tree.nodes.contains_key("left"), "the violating cut is pruned");
    session.model().assert_exhausted();
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
