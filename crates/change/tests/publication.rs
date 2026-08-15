//! Publication materialization inside the execute loop (RFC-95 D11):
//! the fact predicate, the deduped `plan.publication.materialized`
//! fact, the typed publication stops, the status milestone, and the
//! topology-edit lock — over the scripted worktree double, no host Git.

mod support;

use std::sync::{Arc, Mutex};

use change::plan;
use mock::invoke::run;
use mock::session::{ForgeScript, Session, WorktreeScript};
use project::journal::EventKind;
use project::seam::{PrState, PullRequest, WorktreeError, WorktreeRequest, WorktreeState};

const PARENT: &str = "0123456789abcdef0123456789abcdef01234567";

fn suite_answers() -> Vec<String> {
    vec![mock::answers::greeting_synthesis()]
}

/// Scaffold the mock-bound greeting project with the target rebound
/// to an exact Git locator — the publication-member shape — then
/// drain `plan refine`.
async fn scaffold(session: &Session) {
    let scaffolded = run::<project::init::handlers::Init, _, _>(
        session.provider(),
        project::init::handlers::InitInput {
            adapter: Some("mock".to_string()),
            name: Some("demo".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("scaffold");
    assert_eq!(scaffolded.adapter_name, "mock");

    support::write_greeting_plan(session.root());
    let plan_path = session.root().join(".emery/change/plan.yaml");
    let mut plan = change::Plan::load(&plan_path).expect("load plan");
    let binding = plan.targets.get_mut("default").expect("default target");
    binding.locator = format!("https://forge.example/acme/demo@{PARENT}");
    plan.save(&plan_path).expect("save plan");

    let refined = support::refine_plan(session).await;
    assert_eq!(refined.refined, ["greeting"]);
}

/// A recording success script: every export answers `created` at a
/// fixed node-local path.
fn recording_script(calls: &Arc<Mutex<Vec<WorktreeRequest>>>) -> WorktreeScript {
    let calls = Arc::clone(calls);
    WorktreeScript::new(move |req| {
        calls.lock().expect("calls lock").push(req.clone());
        Ok(("/work/publication/demo/default".to_string(), WorktreeState::Created))
    })
}

fn materialized_events(root: &std::path::Path) -> Vec<project::journal::Event> {
    project::journal::read_union(project::config::Layout::new(root))
        .expect("journal union")
        .into_iter()
        .filter(|event| matches!(event.kind, EventKind::PublicationMaterialized { .. }))
        .collect()
}

#[tokio::test]
async fn materialize_on_drain() {
    let mut session = Session::bare(suite_answers());
    let calls = Arc::new(Mutex::new(Vec::new()));
    session.script_worktree(recording_script(&calls));
    scaffold(&session).await;

    let executed =
        run::<plan::handlers::Execute, _, _>(session.provider(), plan::handlers::ExecuteInput {})
            .await
            .expect("execute drains and materializes");
    assert_eq!(executed.status, "drained");

    // Exactly one export, carrying the member's Git identity and the
    // single-member in-place allowance.
    let recorded = calls.lock().expect("calls lock").clone();
    assert_eq!(recorded.len(), 1, "{recorded:?}");
    let req = &recorded[0];
    assert_eq!(req.repository, "https://forge.example/acme/demo");
    assert_eq!(req.parent_revision, PARENT);
    assert_eq!(req.branch, "change/demo");
    assert_eq!(req.plan, "demo");
    assert_eq!(req.target, "default");
    assert!(req.allow_in_place, "single member on a non-detached anchoring");

    // One covering fact, deduped on `(target, cid)`.
    let facts = materialized_events(session.root());
    assert_eq!(facts.len(), 1);
    let EventKind::PublicationMaterialized {
        target,
        branch,
        worktree_path,
        parent_revision,
        cid,
        ..
    } = &facts[0].kind
    else {
        unreachable!()
    };
    assert_eq!(target, "default");
    assert_eq!(branch, "change/demo");
    assert_eq!(worktree_path, "/work/publication/demo/default");
    assert_eq!(parent_revision, PARENT);

    // The fact pins the accepted CID from the committed wave chain.
    let layout = project::config::Layout::new(session.root());
    let events = project::plan::collect_events(layout).expect("events");
    let accepted = project::wave::accepted_cid(layout, &events, "default")
        .expect("accepted chain")
        .expect("accepted CID");
    assert_eq!(*cid, accepted);

    // Status projects the materialized milestone with the operator's
    // next Git step.
    let plan_model = change::Plan::load(&layout.plan_path()).expect("plan");
    let status = project::plan::plan_status_body(&plan_model, layout).expect("status");
    assert_eq!(status.next_action, "drained");
    assert_eq!(status.publication.len(), 1);
    let member = &status.publication[0];
    assert_eq!(member.target, "default");
    assert_eq!(member.branch.as_deref(), Some("change/demo"));
    assert_eq!(member.worktree.as_deref(), Some("/work/publication/demo/default"));
    assert!(member.next.contains("push"), "{}", member.next);

    // Re-running execute is a read-only no-op: no second export, no
    // second fact.
    let again =
        run::<plan::handlers::Execute, _, _>(session.provider(), plan::handlers::ExecuteInput {})
            .await
            .expect("re-entry is a no-op");
    assert_eq!(again.status, "drained");
    assert!(again.phases.is_empty());
    assert_eq!(calls.lock().expect("calls lock").len(), 1);
    assert_eq!(materialized_events(session.root()).len(), 1);
}

#[tokio::test]
async fn dirty_stops_then_resumes() {
    let mut session = Session::bare(suite_answers());
    session.script_worktree(WorktreeScript::new(|_| Err(WorktreeError::Dirty)));
    scaffold(&session).await;

    // The export refusal stops the loop on the typed reason after the
    // merge landed.
    let err =
        run::<plan::handlers::Execute, _, _>(session.provider(), plan::handlers::ExecuteInput {})
            .await
            .expect_err("dirty worktree stops the loop");
    let rendered = format!("{err:?}");
    assert!(rendered.contains("publication-worktree-dirty"), "{rendered}");
    assert!(materialized_events(session.root()).is_empty());

    // The plan holds at the materialize step, resumable through execute.
    let root = session.root().to_path_buf();
    let layout = project::config::Layout::new(&root);
    let plan_model = change::Plan::load(&layout.plan_path()).expect("plan");
    let status = project::plan::plan_status_body(&plan_model, layout).expect("status");
    assert_eq!(status.next_action, "materialize default");
    assert_eq!(status.resume.as_deref(), Some("emery plan execute"));
    assert_eq!(status.publication.len(), 1);
    assert!(status.publication[0].next.contains("emery plan execute"));

    // Only the first run opened an authorization epoch.
    let started = |events: &[project::journal::Event]| {
        events
            .iter()
            .filter(|event| matches!(event.kind, EventKind::PlanExecuteStarted { .. }))
            .count()
    };
    let before = started(&project::plan::collect_events(layout).expect("events"));

    // Fixing the worktree and re-running execute materializes without
    // opening a new epoch.
    let calls = Arc::new(Mutex::new(Vec::new()));
    session.script_worktree(recording_script(&calls));
    let resumed =
        run::<plan::handlers::Execute, _, _>(session.provider(), plan::handlers::ExecuteInput {})
            .await
            .expect("resume materializes");
    assert_eq!(resumed.status, "drained");
    assert!(resumed.phases.is_empty());
    assert_eq!(calls.lock().expect("calls lock").len(), 1);
    assert_eq!(materialized_events(session.root()).len(), 1);
    let after = started(&project::plan::collect_events(layout).expect("events"));
    assert_eq!(before, after, "materialize opens no new epoch");
}

#[tokio::test]
async fn provision_failure_stops() {
    let mut session = Session::bare(suite_answers());
    session.script_worktree(WorktreeScript::new(|_| Err(WorktreeError::BranchDiverged)));
    scaffold(&session).await;

    let err =
        run::<plan::handlers::Execute, _, _>(session.provider(), plan::handlers::ExecuteInput {})
            .await
            .expect_err("provisioning refusal stops the loop");
    let rendered = format!("{err:?}");
    assert!(rendered.contains("publication-provision-failed"), "{rendered}");
    assert!(rendered.contains("diverged"), "{rendered}");
    assert!(materialized_events(session.root()).is_empty());
}

#[tokio::test]
async fn topology_lock() {
    let mut session = Session::bare(suite_answers());
    let calls = Arc::new(Mutex::new(Vec::new()));
    session.script_worktree(recording_script(&calls));
    scaffold(&session).await;
    run::<plan::handlers::Execute, _, _>(session.provider(), plan::handlers::ExecuteInput {})
        .await
        .expect("execute materializes");

    // Adding an entry on the materialized target is rejected.
    let err = run::<plan::handlers::Add, _, _>(
        session.provider(),
        plan::handlers::AddInput {
            name: "late".into(),
            depends_on: Vec::new(),
            sources: vec![plan::wire::BindingArg {
                key: "main".into(),
                lead: Some("greeting".into()),
            }],
            description: None,
            context: Vec::new(),
            authority_override: Vec::new(),
            target: "default".into(),
        },
    )
    .await
    .expect_err("add on a locked target");
    assert!(format!("{err:?}").contains("plan-publication-locked"), "{err:?}");

    // Removing the materialized target's entry is rejected.
    let err = run::<plan::handlers::Remove, _, _>(
        session.provider(),
        plan::handlers::RemoveInput {
            name: "greeting".into(),
        },
    )
    .await
    .expect_err("remove on a locked target");
    assert!(format!("{err:?}").contains("plan-publication-locked"), "{err:?}");
}

/// Drain the scripted-worktree greeting change and return the plan
/// digest the materialized fact recorded — the D3 trailer digest.
async fn drain(session: &Session) -> String {
    scaffold(session).await;
    run::<plan::handlers::Execute, _, _>(session.provider(), plan::handlers::ExecuteInput {})
        .await
        .expect("execute drains");
    let facts = materialized_events(session.root());
    let EventKind::PublicationMaterialized { plan_digest, .. } = &facts[0].kind else {
        unreachable!()
    };
    plan_digest.clone()
}

/// One pull request whose body carries both D3 trailers.
fn pull(state: PrState, merged_at: Option<&str>, digest: &str) -> PullRequest {
    PullRequest {
        url: "https://github.com/acme/demo/pull/7".to_string(),
        body: format!("Change summary.\n\nEmery-Change: demo\nEmery-Change-Digest: {digest}\n"),
        state,
        base: "main".to_string(),
        merged_at: merged_at.map(str::to_string),
        merge_commit: merged_at.map(|_| "8e43c0ffee".to_string()),
    }
}

fn archive_input() -> plan::handlers::ArchiveInput {
    plan::handlers::ArchiveInput::default()
}

fn publication_events(root: &std::path::Path) -> Vec<project::journal::Event> {
    project::journal::read_union(project::config::Layout::new(root))
        .expect("journal union")
        .into_iter()
        .filter(|event| {
            matches!(
                event.kind,
                EventKind::PublicationProjected { .. }
                    | EventKind::PublicationMemberLanded { .. }
                    | EventKind::PublicationUnverified { .. }
            )
        })
        .collect()
}

#[tokio::test]
async fn archive_verified() {
    let mut session = Session::bare(suite_answers());
    let calls = Arc::new(Mutex::new(Vec::new()));
    session.script_worktree(recording_script(&calls));
    let digest = drain(&session).await;
    session.script_forge(ForgeScript::new(move |repository, branch| {
        assert_eq!(repository, "https://forge.example/acme/demo");
        assert_eq!(branch, "change/demo");
        Ok(vec![pull(PrState::Merged, Some("2026-08-15T01:02:03Z"), &digest)])
    }));

    let archived = run::<plan::handlers::Archive, _, _>(session.provider(), archive_input())
        .await
        .expect("verified set archives");
    let publication = archived.publication.expect("publication record");
    assert_eq!(publication.verification, project::plan::publication::Verification::Verified);
    assert!(publication.failures.is_empty());
    assert_eq!(publication.members.len(), 1);
    let member = &publication.members[0];
    assert_eq!(member.target, "default");
    assert_eq!(member.merge_commit.as_deref(), Some("8e43c0ffee"));
    assert_eq!(member.base.as_deref(), Some("main"));
    assert_eq!(member.publication, project::plan::publication::PublicationState::Merged);
    assert!(member.order.is_none(), "single unrelated member carries no rank");

    // The archive journaled the projection and the landed member —
    // and no unverified bypass.
    let facts = publication_events(session.root());
    assert_eq!(facts.len(), 2, "{facts:?}");
    assert!(matches!(&facts[0].kind, EventKind::PublicationProjected { verification, .. }
        if *verification == project::plan::publication::Verification::Verified));
    let EventKind::PublicationMemberLanded {
        target,
        pull_request,
        merge_commit,
        merged_at,
        ..
    } = &facts[1].kind
    else {
        panic!("expected member-landed, got {:?}", facts[1].kind)
    };
    assert_eq!(target, "default");
    assert_eq!(pull_request, "https://github.com/acme/demo/pull/7");
    assert_eq!(merge_commit, "8e43c0ffee");
    assert_eq!(merged_at, "2026-08-15T01:02:03Z");
}

#[tokio::test]
async fn archive_pending_refuses() {
    let mut session = Session::bare(suite_answers());
    let calls = Arc::new(Mutex::new(Vec::new()));
    session.script_worktree(recording_script(&calls));
    let digest = drain(&session).await;
    session
        .script_forge(ForgeScript::new(move |_, _| Ok(vec![pull(PrState::Open, None, &digest)])));

    let err = run::<plan::handlers::Archive, _, _>(session.provider(), archive_input())
        .await
        .expect_err("open pull request refuses");
    let rendered = format!("{err:?}");
    assert!(rendered.contains("publication-unverified"), "{rendered}");
    assert!(rendered.contains("default"), "{rendered}");
    assert!(rendered.contains("unmerged"), "{rendered}");
    assert!(publication_events(session.root()).is_empty(), "refusal journals nothing");
    assert!(session.root().join(".emery/change/plan.yaml").exists(), "the plan never moved");
}

#[tokio::test]
async fn unverified_bypass() {
    let mut session = Session::bare(suite_answers());
    let calls = Arc::new(Mutex::new(Vec::new()));
    session.script_worktree(recording_script(&calls));
    let digest = drain(&session).await;
    session
        .script_forge(ForgeScript::new(move |_, _| Ok(vec![pull(PrState::Open, None, &digest)])));

    let archived = run::<plan::handlers::Archive, _, _>(
        session.provider(),
        plan::handlers::ArchiveInput {
            force: false,
            unverified: true,
        },
    )
    .await
    .expect("--unverified archives");
    let publication = archived.publication.expect("publication record");
    assert_eq!(publication.verification, project::plan::publication::Verification::Pending);

    // Bypass fact first, then the projection snapshot.
    let facts = publication_events(session.root());
    assert!(matches!(facts[0].kind, EventKind::PublicationUnverified { .. }), "{facts:?}");
    assert!(matches!(facts[1].kind, EventKind::PublicationProjected { .. }), "{facts:?}");
}

#[tokio::test]
async fn unpublished_is_pending() {
    let mut session = Session::bare(suite_answers());
    let calls = Arc::new(Mutex::new(Vec::new()));
    session.script_worktree(recording_script(&calls));
    drain(&session).await;
    session.script_forge(ForgeScript::new(|_, _| Ok(Vec::new())));

    let err = run::<plan::handlers::Archive, _, _>(session.provider(), archive_input())
        .await
        .expect_err("unpublished member refuses");
    assert!(format!("{err:?}").contains("unpublished"), "{err:?}");
}

#[tokio::test]
async fn digest_mismatch() {
    let mut session = Session::bare(suite_answers());
    let calls = Arc::new(Mutex::new(Vec::new()));
    session.script_worktree(recording_script(&calls));
    drain(&session).await;
    session.script_forge(ForgeScript::new(|_, _| {
        Ok(vec![pull(PrState::Merged, Some("2026-08-15T01:02:03Z"), "sha256:wrong")])
    }));

    let err = run::<plan::handlers::Archive, _, _>(session.provider(), archive_input())
        .await
        .expect_err("digest mismatch refuses");
    assert!(format!("{err:?}").contains("unpublished"), "{err:?}");
}

#[tokio::test]
async fn several_matches() {
    let mut session = Session::bare(suite_answers());
    let calls = Arc::new(Mutex::new(Vec::new()));
    session.script_worktree(recording_script(&calls));
    let digest = drain(&session).await;
    session.script_forge(ForgeScript::new(move |_, _| {
        Ok(vec![
            pull(PrState::Merged, Some("2026-08-15T01:02:03Z"), &digest),
            pull(PrState::Open, None, &digest),
        ])
    }));

    let err = run::<plan::handlers::Archive, _, _>(session.provider(), archive_input())
        .await
        .expect_err("ambiguous match refuses");
    assert!(format!("{err:?}").contains("2 pull requests match"), "{err:?}");
}

#[tokio::test]
async fn closed_is_unverified() {
    let mut session = Session::bare(suite_answers());
    let calls = Arc::new(Mutex::new(Vec::new()));
    session.script_worktree(recording_script(&calls));
    let digest = drain(&session).await;
    session
        .script_forge(ForgeScript::new(move |_, _| Ok(vec![pull(PrState::Closed, None, &digest)])));

    let err = run::<plan::handlers::Archive, _, _>(session.provider(), archive_input())
        .await
        .expect_err("closed pull request refuses");
    assert!(format!("{err:?}").contains("closed without merging"), "{err:?}");
}

#[tokio::test]
async fn transport_distinct() {
    let mut session = Session::bare(suite_answers());
    let calls = Arc::new(Mutex::new(Vec::new()));
    session.script_worktree(recording_script(&calls));
    drain(&session).await;
    session.script_forge(ForgeScript::new(|_, _| {
        Err(project::seam::ForgeError::Transport("connection refused".to_string()))
    }));

    // A transport failure is its own outcome — never `unverified`, and
    // `--unverified` does not paper over it.
    let err = run::<plan::handlers::Archive, _, _>(
        session.provider(),
        plan::handlers::ArchiveInput {
            force: false,
            unverified: true,
        },
    )
    .await
    .expect_err("transport failure refuses");
    let rendered = format!("{err:?}");
    assert!(rendered.contains("publication-forge-failed"), "{rendered}");
    assert!(!rendered.contains("publication-unverified"), "{rendered}");
}
