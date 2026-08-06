//! RFC-86 Acceptance #2 / D23 — multi-slice, multi-actor fixtures.
//!
//! Two directory copies of one change claim and refine *different*
//! slices, then combine via lossless fact-tree union. Same-slice
//! overlap fails closed as `slice-claim-conflict`. Neither copy needs
//! Git metadata, and plan-wide single-active-entry is not assumed.

use std::fs;
use std::path::Path;

use change::plan;
use error::Error;
use jiff::Timestamp;
use mock::invoke::run;
use mock::session::Session;
use project::config::{Layout, ProjectConfig};
use project::handler::Anchor;
use project::journal::{Event, EventKind, append_for, claim, read_union};
use project::name::SliceName;
use project::plan::{Status, advance_next, project_ladders};
use project::slice::{LifecycleStatus, SliceMetadata};
use serde_json::json;

/// RAII `EMERY_ACTOR` guard — restores the prior value on drop.
struct ActorEnv {
    prev: Option<std::ffi::OsString>,
}

impl ActorEnv {
    #[expect(
        unsafe_code,
        reason = "EMERY_ACTOR is the journal actor seam; nextest isolates the process"
    )]
    fn set(id: &str) -> Self {
        let prev = std::env::var_os("EMERY_ACTOR");
        // SAFETY: nextest isolates processes; Drop restores the prior value.
        unsafe { std::env::set_var("EMERY_ACTOR", id) };
        Self { prev }
    }
}

impl Drop for ActorEnv {
    #[expect(unsafe_code, reason = "restore EMERY_ACTOR after the fixture's actor window")]
    fn drop(&mut self) {
        match &self.prev {
            Some(value) => {
                // SAFETY: pair with `ActorEnv::set`; single-threaded test body.
                unsafe { std::env::set_var("EMERY_ACTOR", value) };
            }
            None => {
                // SAFETY: pair with `ActorEnv::set`; single-threaded test body.
                unsafe { std::env::remove_var("EMERY_ACTOR") };
            }
        }
    }
}

fn adversarial_bindings() -> Vec<plan::wire::SourceAssign> {
    ["docs", "code"]
        .map(|key| {
            serde_json::from_value(json!({
                "key": key,
                "adapter": format!("mock-{key}"),
                "value": format!("The {key} source."),
            }))
            .expect("mock binding parses")
        })
        .to_vec()
}

fn ts(second: i64) -> Timestamp {
    Timestamp::from_second(1_700_000_000 + second).expect("valid timestamp")
}

fn claim_slice(root: &Path, actor: &str, plan_name: &str, slice: &str, second: i64) {
    append_for(
        Layout::new(root),
        actor,
        &[
            Event::new(
                ts(second),
                EventKind::SliceClaimed {
                    slice_name: slice.into(),
                },
            ),
            Event::new(
                ts(second),
                EventKind::PlanEntryAdvanced {
                    plan_name: plan_name.into(),
                    slice_name: slice.into(),
                },
            ),
        ],
    )
    .expect("claim + advance facts");
}

fn mirror_tree(from: &Path, to: &Path) {
    if to.exists() {
        fs::remove_dir_all(to).expect("clear destination");
    }
    copy_recursive(from, to);
}

fn copy_recursive(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("mkdir dest");
    for entry in fs::read_dir(from).expect("read_dir") {
        let entry = entry.expect("entry");
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() {
            copy_recursive(&src, &dst);
        } else {
            fs::copy(&src, &dst).expect("copy file");
        }
    }
}

fn assert_no_git(root: &Path) {
    assert!(
        !root.join(".git").exists(),
        "fixture must not require Git metadata: {}",
        root.display()
    );
}

fn project_lifecycle(root: &Path, slice: &str) -> LifecycleStatus {
    let slice_dir = root.join(".emery/slices").join(slice);
    let metadata = SliceMetadata::load(&slice_dir).expect("metadata.yaml");
    LifecycleStatus::project(&slice_dir, &metadata)
}

async fn author_adversarial(session: &Session) {
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "auth".to_string(),
            sources: adversarial_bindings(),
            intent: None,
            force: false,
        },
    )
    .await
    .expect("author walks to pending");
}

/// Acceptance #2 happy path: disjoint claims on two copies, refine,
/// lossless fact-tree union → both slices refined.
#[tokio::test]
async fn disjoint_refine_after_fact_union() {
    let alice = Session::scripted(
        "mock",
        vec![mock::answers::adversarial_grouping(), mock::answers::login_flow_synthesis()],
    );
    author_adversarial(&alice).await;
    assert_no_git(alice.root());

    // Freeze the authored change, then give bob an independent copy.
    let bob = Session::scripted("mock", vec![mock::answers::password_reset_synthesis()]);
    mirror_tree(alice.root(), bob.root());
    assert_no_git(bob.root());

    // Concurrent work on different slices — claim via append_for (the
    // multi-actor fixture surface), then refine under each actor id.
    claim_slice(alice.root(), "alice", "auth", "login-flow", 1);
    {
        let _actor = ActorEnv::set("alice");
        run::<slice::handlers::Refine, _, _>(
            alice.provider(),
            slice::handlers::RefineInput {
                name: "login-flow".to_string(),
            },
        )
        .await
        .expect("alice refines login-flow")
    };

    claim_slice(bob.root(), "bob", "auth", "password-reset", 1);
    {
        let _actor = ActorEnv::set("bob");
        run::<slice::handlers::Refine, _, _>(
            bob.provider(),
            slice::handlers::RefineInput {
                name: "password-reset".to_string(),
            },
        )
        .await
        .expect("bob refines password-reset")
    };

    // Lossless fact-tree union: fold bob's actor log + refined slice
    // into alice's change tree.
    let union_root = alice.root();
    let bob_events = bob.root().join(".emery/events/bob.jsonl");
    assert!(bob_events.is_file(), "bob wrote a per-actor log");
    fs::create_dir_all(union_root.join(".emery/events")).expect("events dir");
    fs::copy(&bob_events, union_root.join(".emery/events/bob.jsonl")).expect("union bob events");
    mirror_tree(
        &bob.root().join(".emery/slices/password-reset"),
        &union_root.join(".emery/slices/password-reset"),
    );

    let events = read_union(Layout::new(union_root)).expect("union reads");
    let ownership = claim::project(&events);
    assert_eq!(ownership.owner(&"login-flow".into()), Some("alice"));
    assert_eq!(ownership.owner(&"password-reset".into()), Some("bob"));
    assert_eq!(ownership.len(), 2, "disjoint claims survive the union");

    assert_eq!(project_lifecycle(union_root, "login-flow"), LifecycleStatus::Refined);
    assert_eq!(project_lifecycle(union_root, "password-reset"), LifecycleStatus::Refined);

    let plan = project::plan::Plan::load(&Layout::new(union_root).plan_path()).expect("plan");
    let ladders = project_ladders(&plan, &events);
    let login: SliceName = "login-flow".into();
    let reset: SliceName = "password-reset".into();
    let session_policy: SliceName = "session-policy".into();
    assert_eq!(ladders.get(&login).copied(), Some(Status::InProgress));
    assert_eq!(ladders.get(&reset).copied(), Some(Status::InProgress));
    // Three in-flight entries would be illegal under single-active-entry;
    // session-policy stays pending while the two claimed slices progress.
    assert_eq!(ladders.get(&session_policy).copied(), Some(Status::Pending));

    alice.model().assert_exhausted();
    bob.model().assert_exhausted();
}

/// Same slice, two actors → `slice-claim-conflict` (Acceptance #2).
#[tokio::test]
async fn same_slice_claim_conflict() {
    let session = Session::scripted("mock", vec![mock::answers::adversarial_grouping()]);
    author_adversarial(&session).await;
    assert_no_git(session.root());

    claim_slice(session.root(), "alice", "auth", "login-flow", 1);
    let ownership = claim::project(&read_union(Layout::new(session.root())).expect("union"));
    let err = claim::claim(&ownership, "login-flow".into(), "bob").expect_err("conflict");
    match err {
        Error::Validation { code, detail } => {
            assert_eq!(code, "slice-claim-conflict");
            assert!(detail.contains("alice"), "{detail}");
            assert!(detail.contains("bob"), "{detail}");
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

/// Plan-wide single-active-entry is gone: bob may advance a sibling
/// while alice's claim on another slice is still live (D23).
#[tokio::test]
async fn sibling_advance_while_peer_claimed() {
    let session = Session::scripted("mock", vec![mock::answers::adversarial_grouping()]);
    author_adversarial(&session).await;

    claim_slice(session.root(), "alice", "auth", "login-flow", 1);

    let config = ProjectConfig::load(session.root()).expect("project.yaml");
    let now = ts(2);
    let _actor = ActorEnv::set("bob");
    let body = advance_next(session.provider(), session.provider().paths(), now, &config)
        .expect("bob advances a different slice");
    assert_eq!(
        body.advanced.as_deref(),
        Some("session-policy"),
        "next eligible sibling advances without waiting for alice: {body:?}"
    );

    let events = read_union(Layout::new(session.root())).expect("union");
    let ownership = claim::project(&events);
    assert_eq!(ownership.owner(&"login-flow".into()), Some("alice"));
    assert_eq!(ownership.owner(&"session-policy".into()), Some("bob"));

    let plan = project::plan::Plan::load(&Layout::new(session.root()).plan_path()).expect("plan");
    let ladders = project_ladders(&plan, &events);
    let login: SliceName = "login-flow".into();
    let session_policy: SliceName = "session-policy".into();
    assert_eq!(ladders.get(&login).copied(), Some(Status::InProgress));
    assert_eq!(ladders.get(&session_policy).copied(), Some(Status::InProgress));
}
