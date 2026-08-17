//! Cut B3 (RFC-96 D8): domain rounds in the execute drain. A
//! converged plan records passing frontier + complete rounds and
//! drains; a failed frontier round parks the wave as
//! `domain-frontier-failed`; a failed complete round blocks drain as
//! `domain-complete-failed`; restart reuses recorded rounds without
//! re-verifying.

mod support;

use std::fs;

use change::plan::handlers::{Execute, ExecuteInput};
use mock::invoke::run;
use mock::session::Session;
use project::adapter::catalog::Pin;
use project::domain::{DomainRound, RoundKind, Verdict};
use project::handler::Anchor as _;
use project::journal::{EventKind, read_union};
use project::plan::decomposition::{Decomposition, Kind, Node};
use project::plan::{Plan, TargetBinding};
use project::snapshot::SnapshotId;

#[expect(unsafe_code, reason = "EMERY_POOL is the launcher cap seam; nextest isolates the process")]
fn set_cap(cap: &str) {
    // SAFETY: nextest runs each test in its own process, and the env
    // write happens before any pool dispatch reads the cap.
    unsafe { std::env::set_var("EMERY_POOL", cap) };
}

async fn seed_cid(session: &Session, name: &str) -> SnapshotId {
    let home = session.root().parent().expect("session home");
    let seed = home.join(name);
    fs::create_dir_all(seed.join(".emery")).expect("seed .emery");
    fs::write(seed.join("README.md"), format!("{name}\n")).expect("seed file");
    session.store().snapshot(&seed).await.expect("snapshot seed")
}

/// Two slices on one target `app`, plus a root/two-leaf
/// `decomposition.yaml` binding the plan (digest pinned). `depend`
/// makes beta depend on alpha (two one-member waves instead of one
/// frozen pair).
async fn write_domain_fixture(session: &Session, depend: bool) {
    support::write_plan_fixture(
        session.root(),
        "demo",
        &[("main", "mock", "The greeting service.")],
        &[("alpha", "main", "greeting"), ("beta", "main", "greeting")],
    );
    let app = seed_cid(session, "seed-app").await;
    let layout = session.provider().paths().layout();
    let mut plan = Plan::load(&layout.plan_path()).expect("plan");
    plan.targets.insert(
        "app".into(),
        TargetBinding::new(Pin::emery("mock", semver::Version::new(0, 0, 0)), ".", app),
    );
    plan.targets.remove("default");
    for entry in &mut plan.entries {
        entry.target = "app".into();
        if depend && entry.name.as_str() == "beta" {
            entry.depends_on = vec!["alpha".into()];
        }
    }

    let leads_digest = project::plan::retain_leads(layout).expect("retain leads");
    let scope = project::plan::decomposition::Scope::new("main", "greeting");
    let mut alpha = Node::leaf("app", "alpha");
    alpha.parent = Some("root".into());
    alpha.sources = vec![scope.clone()];
    let mut beta = Node::leaf("app", "beta");
    beta.parent = Some("root".into());
    beta.sources = vec![scope];
    if depend {
        beta.depends_on = vec!["leaf-alpha".into()];
    }
    let tree = Decomposition {
        version: project::plan::decomposition::VERSION,
        leads_digest: leads_digest.clone(),
        profiles: std::collections::BTreeMap::new(),
        root: "root".into(),
        nodes: std::collections::BTreeMap::from([
            (
                "root".to_string(),
                Node {
                    children: vec!["leaf-alpha".into(), "leaf-beta".into()],
                    kind: Some(Kind::Split),
                    ..Node::default()
                },
            ),
            ("leaf-alpha".to_string(), alpha),
            ("leaf-beta".to_string(), beta),
        ]),
    };
    tree.save(&layout.decomposition_path()).expect("decomposition.yaml");
    plan.decomposition_digest =
        Some(project::plan::decomposition::retain(layout).expect("retain revision"));
    plan.leads_digest = Some(leads_digest);
    plan.save(&layout.plan_path()).expect("plan.yaml");

    for slice in ["alpha", "beta"] {
        let dir = layout.slice_dir(slice);
        fs::create_dir_all(&dir).expect("slice dir");
        fs::write(
            dir.join("metadata.yaml"),
            "target: mock@0.0.0\ncreated-at: 2026-01-01T00:00:00Z\ndefined-at: \
             2026-01-01T00:00:01Z\n",
        )
        .expect("metadata");
        support::stage_manifest(session.root(), slice);
    }
}

/// The `(domain, kind, verdict)` rows journaled so far, in fact order.
fn recorded(session: &Session) -> Vec<(String, RoundKind, Verdict)> {
    let layout = session.provider().paths().layout();
    read_union(layout)
        .expect("union")
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::DomainConvergenceRecorded {
                domain,
                kind,
                verdict,
                ..
            } => Some((domain.clone(), *kind, *verdict)),
            _ => None,
        })
        .collect()
}

// A converged plan: the two-member wave's frontier round and the
// root's complete round both record `passed`, the wave commits, and
// the drain completes — the durable rounds live under
// `targets/app/domains/`.
#[tokio::test]
async fn converged_drain() {
    // Frozen wave membership is capped at the pool cap; the default is
    // serial (Phase 0), so the two-member wave needs an explicit cap.
    set_cap("2");
    let session = Session::scripted("mock", Vec::new());
    write_domain_fixture(&session, false).await;

    let executed = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect("execute drains");
    assert_eq!(executed.status, "drained", "{executed:?}");
    assert_eq!(
        recorded(&session),
        [
            ("root".to_string(), RoundKind::Frontier, Verdict::Passed),
            ("root".to_string(), RoundKind::Complete, Verdict::Passed),
        ],
        "one passing frontier and one passing complete round"
    );
    let layout = session.provider().paths().layout();
    let rounds = DomainRound::load_all(layout, "app").expect("rounds");
    assert_eq!(rounds.len(), 2, "both rounds persisted under targets/app/domains/");
    assert!(rounds.iter().all(|round| round.verification_report.is_some()));
}

// A failed frontier round parks the wave: builds succeed, the stop is
// `domain-frontier-failed`, nothing commits, and the re-run reuses
// the recorded round (same stop, no second record).
#[tokio::test]
async fn frontier_failure_parks() {
    // The frontier round only gates a multi-member wave — cap 2 lets
    // the same-target ready pair freeze one (the default is serial).
    set_cap("2");
    let session = Session::scripted("mock", Vec::new());
    write_domain_fixture(&session, false).await;
    support::marker(session.root(), mock::behaviour::DOMAIN_VERIFY_BLOCKED);

    let stopped = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("the failed frontier round parks the run")
        .to_string();
    assert!(stopped.contains("domain-frontier-failed"), "{stopped}");

    let layout = session.provider().paths().layout();
    let events = read_union(layout).expect("union");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event.kind, EventKind::TargetMergeWaveCommitted { .. })),
        "the parked wave never commits"
    );
    assert_eq!(recorded(&session), [("root".to_string(), RoundKind::Frontier, Verdict::Failed)]);

    let again = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("restart reuses the failed round")
        .to_string();
    assert!(again.contains("domain-frontier-failed"), "{again}");
    assert_eq!(
        recorded(&session).len(),
        1,
        "reuse by identity — no second round, no re-verification"
    );
}

// A failed complete round blocks drain: every leaf merges (dependent
// one-member waves skip frontier rounds), the accepted-tree
// verification fails, and the run stops `domain-complete-failed`;
// the re-run reuses the durable failed round.
#[tokio::test]
async fn complete_failure_blocks() {
    let session = Session::scripted("mock", Vec::new());
    write_domain_fixture(&session, true).await;
    support::marker(session.root(), mock::behaviour::DOMAIN_VERIFY_BLOCKED);

    let stopped = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("the failed complete round blocks the drain")
        .to_string();
    assert!(stopped.contains("domain-complete-failed"), "{stopped}");

    let layout = session.provider().paths().layout();
    let events = read_union(layout).expect("union");
    let committed = events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::TargetMergeWaveCommitted { .. }))
        .count();
    assert_eq!(committed, 2, "both merges landed before the gate");
    assert_eq!(recorded(&session), [("root".to_string(), RoundKind::Complete, Verdict::Failed)]);

    let again = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("restart re-projects the durable stop")
        .to_string();
    assert!(again.contains("domain-complete-failed"), "{again}");
    assert_eq!(recorded(&session).len(), 1, "the failed round is reused, not re-run");
}
