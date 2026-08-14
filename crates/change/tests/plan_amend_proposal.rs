//! Amendment proposals: compare-and-set application, preservation,
//! domain-mutation lowering, and epoch invalidation (RFC-88 D8).

use std::collections::BTreeMap;

use artifacts::leads::{Lead, Leads};
use change::plan;
use change::plan::handlers::{AddInput, AmendBody, AmendInput, RemoveInput};
use mock::definition::{Spec, mint};
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{ClosedPlanCoverage, Event, EventKind, FactEpochRef, append_one};
use project::plan::decomposition::{Decomposition, Kind, Node, Scope};
use project::plan::epoch::{self, EpochFreshness};
use project::plan::{
    Envelope, FocusParent, Frontiers, Ownership, PROPOSAL_VERSION, Plan, ProfileRef, Proposal,
    Repair, Revision,
};
use project::profile::Assessment;
use project::snapshot::SnapshotId;

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

fn rebind_linked_mock(layout: Layout<'_>) {
    let pin = project::adapter::catalog::Pin::parse("emery:mock@0.0.0").expect("mock pin");
    let mut plan = Plan::load(&layout.plan_path()).expect("plan");
    for binding in plan.sources.values_mut() {
        binding.adapter = pin.clone();
    }
    for row in plan.targets.values_mut() {
        row.adapter = pin.clone();
    }
    plan.save(&layout.plan_path()).expect("rebind");
}

async fn author_greeting(session: &Session) {
    let target = seed_target(session.root(), "target-app");
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            from: definition,
            wave: spec.wave,
            force: false,
        },
    )
    .await
    .expect("author");
    rebind_linked_mock(Layout::new(session.root()));
}

async fn authored() -> Session {
    let session = Session::scripted("mock", mock::answers::greeting_author());
    author_greeting(&session).await;
    session.model().assert_exhausted();
    session
}

fn cid(ch: char) -> SnapshotId {
    SnapshotId::from_digest(&ch.to_string().repeat(64))
}

const fn loud() -> Assessment {
    Assessment {
        behavioural_breadth: 10,
        coupling: 10,
        uncertainty: 10,
        context_volume: 10,
        verification_surface: 10,
    }
}

fn profile_of(plan: &Plan, tree: &Decomposition) -> ProfileRef {
    let target = &plan.entries[0].target;
    if let Some(row) = plan.targets.get(target)
        && let Some(profile) = &row.model_capability_profile
    {
        return profile.clone();
    }
    let bound = tree.profiles.get(target).expect("bound profile");
    ProfileRef {
        id: bound.id.clone(),
        digest: bound.digest.clone(),
    }
}

fn child_lead(id: &str) -> Lead {
    let mut lead = Lead::new(id, "intent", format!("{id} surface."));
    lead.parent = Some("intent".into());
    lead
}

fn child_leaf(id: &str, target: &str, parent: &str) -> Node {
    let mut node = Node::leaf(target, id);
    node.parent = Some(parent.into());
    node.kind = Some(Kind::Leaf);
    node.sources = vec![Scope::new("intent", id)];
    node.ownership = vec![format!("{id}/**")];
    node.acceptance = Some(id.into());
    node
}

/// Candidate two-leaf split of the unary greeting tree.
fn split_greeting(layout: Layout<'_>) -> (Vec<Lead>, Decomposition) {
    let mut tree = Decomposition::load(&layout.decomposition_path()).expect("tree");
    let greeting = tree.leaf_id("greeting").expect("greeting leaf").to_string();
    let node = tree.node(&greeting).expect("greeting node").clone();
    let target = node.target.expect("greeting target");
    let parent = node.parent.expect("greeting parent");

    tree.nodes.remove(&greeting);
    tree.nodes.insert("hello".into(), child_leaf("hello", &target, &parent));
    tree.nodes.insert("world".into(), child_leaf("world", &target, &parent));
    let root = tree.node_mut(&parent).expect("parent");
    root.children.retain(|child| child != &greeting);
    root.children.extend(["hello".into(), "world".into()]);
    root.sources = vec![Scope::new("intent", "hello"), Scope::new("intent", "world")];
    tree.check().expect("split tree is valid");

    let mut leads = Leads::load(&layout.leads_path()).expect("leads").into_leads();
    leads.push(child_lead("hello"));
    leads.push(child_lead("world"));
    let catalog = Leads::from_leads(leads.clone());
    tree.leads_digest = SnapshotId::from_digest(&catalog.digest_hex().expect("digest"));
    (leads, tree)
}

fn persist_boundary(
    layout: Layout<'_>, tweak: impl FnOnce(&mut Frontiers, &mut Decomposition),
) -> SnapshotId {
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    let (leads, mut tree) = split_greeting(layout);
    let mut expected = Frontiers::live(layout, &plan).expect("frontiers");
    tweak(&mut expected, &mut tree);
    let proposal = Proposal::Boundary(project::plan::BoundaryProposal {
        version: PROPOSAL_VERSION,
        failed_leaf: "greeting".into(),
        assessment: loud(),
        profile: profile_of(&plan, &tree),
        rationale: "Evidence supports separately acceptable child boundaries.".into(),
        affected: vec![FocusParent {
            source: "intent".into(),
            lead: "intent".into(),
        }],
        candidate_leads: leads,
        candidate_decomposition: tree,
        expected,
    });
    proposal.save(layout).expect("save proposal")
}

fn proposal_input(digest: &SnapshotId) -> AmendInput {
    AmendInput {
        name: None,
        proposal: Some(digest.to_string()),
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
    }
}

fn entry_input(name: &str) -> AmendInput {
    AmendInput {
        name: Some(name.into()),
        proposal: None,
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
    }
}

async fn apply_digest(
    session: &Session, digest: &SnapshotId,
) -> Result<AmendBody, project::handler::Error> {
    run::<plan::handlers::Amend, _, _>(session.provider(), proposal_input(digest)).await
}

async fn refuse_stale(
    session: &Session, layout: Layout<'_>, before: &[u8], label: &str,
    mutate: impl FnOnce(&mut Frontiers),
) {
    let digest = persist_boundary(layout, |expected, _| mutate(expected));
    let err = apply_digest(session, &digest).await.expect_err(label).to_string();
    assert!(err.contains("plan-proposal-stale"), "{label}: {err}");
    assert_eq!(std::fs::read(layout.plan_path()).expect("plan"), before, "{label} wrote plan.yaml");
}

fn names(plan: &Plan) -> Vec<&str> {
    plan.entries.iter().map(|entry| entry.name.as_str()).collect()
}

fn stamp_epoch(layout: Layout<'_>) {
    append_one(
        layout,
        &Event::new(
            jiff::Timestamp::now(),
            EventKind::PlanExecuteStarted {
                coverage: ClosedPlanCoverage::ClosedPlan {
                    plan_digest: Plan::file_digest(layout).expect("plan digest"),
                    refinements: BTreeMap::default(),
                },
                discovery_digest: None,
            },
        ),
    )
    .expect("stamp epoch");
}

// Compare-and-set: each expected-frontier mismatch refuses and writes
// nothing. Claims are recorded on Frontiers but are not CAS-equal —
// live work is the separate `plan-proposal-live` check.
#[tokio::test]
async fn cas_mismatch() {
    let session = authored().await;
    let layout = Layout::new(session.root());
    let before = std::fs::read(layout.plan_path()).expect("plan bytes");
    let live =
        Frontiers::live(layout, &Plan::load(&layout.plan_path()).expect("plan")).expect("live");

    refuse_stale(&session, layout, &before, "leads-digest", |f| f.leads_digest = cid('1')).await;
    refuse_stale(&session, layout, &before, "decomposition-digest", |f| {
        f.decomposition_digest = cid('2');
    })
    .await;
    refuse_stale(&session, layout, &before, "discovery-digest", |f| {
        f.discovery_digest = Some(cid('3'));
    })
    .await;
    refuse_stale(&session, layout, &before, "plan-digest", |f| f.plan_digest = cid('4')).await;
    refuse_stale(&session, layout, &before, "accepted", |f| {
        if let Some(got) = f.accepted.values_mut().next() {
            *got = cid('5');
        }
    })
    .await;
    refuse_stale(&session, layout, &before, "committed", |f| {
        f.committed.insert("greeting".into(), cid('6'));
    })
    .await;
    assert_eq!(live.plan_digest.as_str(), Plan::file_digest(layout).expect("digest").as_str());
}

// Envelope and definition-revision documents refuse as non-amendments.
#[tokio::test]
async fn kind_refuses() {
    let session = authored().await;
    let layout = Layout::new(session.root());
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    let expected = Frontiers::live(layout, &plan).expect("frontiers");
    let envelope = Proposal::Envelope(Envelope {
        version: PROPOSAL_VERSION,
        blocking: "src/auth.rs".into(),
        nearest: "root".into(),
        profile: profile_of(
            &plan,
            &Decomposition::load(&layout.decomposition_path()).expect("tree"),
        ),
        expected: expected.clone(),
    });
    let revision = Proposal::Revision(Revision {
        version: PROPOSAL_VERSION,
        handoff: cid('a'),
        detail: "reviewed wave contradicts the bound sources".into(),
        scope: vec!["greeting".into()],
    });
    for proposal in [envelope, revision] {
        let digest = proposal.save(layout).expect("save");
        let err = apply_digest(&session, &digest).await.expect_err("kind").to_string();
        assert!(err.contains("plan-proposal-kind"), "{err}");
    }
}

#[tokio::test]
async fn not_found() {
    let session = authored().await;
    let err = apply_digest(&session, &cid('a')).await.expect_err("missing").to_string();
    assert!(err.contains("plan-proposal-not-found"), "{err}");
}

#[tokio::test]
async fn malformed() {
    let session = authored().await;
    let layout = Layout::new(session.root());
    let digest = persist_boundary(layout, |_, _| {});
    let yaml = std::fs::read(layout.proposal_path(&digest)).expect("yaml");
    let wrong = cid('b');
    std::fs::write(layout.proposal_path(&wrong), yaml).expect("misnamed");
    let err = apply_digest(&session, &wrong).await.expect_err("malformed").to_string();
    assert!(err.contains("plan-proposal-malformed"), "{err}");
}

#[tokio::test]
async fn live_claim() {
    let session = authored().await;
    let layout = Layout::new(session.root());
    append_one(
        layout,
        &Event::new(
            jiff::Timestamp::now(),
            EventKind::SliceClaimed {
                slice_name: "greeting".into(),
            },
        ),
    )
    .expect("claim");
    let digest = persist_boundary(layout, |_, _| {});
    let err = apply_digest(&session, &digest).await.expect_err("live").to_string();
    assert!(err.contains("plan-proposal-live"), "{err}");
}

#[tokio::test]
async fn cycle() {
    let session = authored().await;
    let layout = Layout::new(session.root());
    let digest = persist_boundary(layout, |_, tree| {
        tree.node_mut("hello").expect("hello").depends_on = vec!["world".into()];
        tree.node_mut("world").expect("world").depends_on = vec!["hello".into()];
    });
    let err = apply_digest(&session, &digest).await.expect_err("cycle").to_string();
    assert!(err.contains("plan-proposal-cycle"), "{err}");
}

// Committed greeting cannot be replaced by the split.
#[tokio::test]
async fn preserve() {
    let session = authored().await;
    let layout = Layout::new(session.root());
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    let target = plan.entries[0].target.clone();
    let base = plan.targets[&target].cid.clone();
    append_one(
        layout,
        &Event::new(
            jiff::Timestamp::now(),
            EventKind::TargetMergeWaveCommitted {
                target,
                digest: cid('d').to_string(),
                members: vec!["greeting".into()],
                base: base.clone(),
                result: cid('e'),
                commit_authorization: FactEpochRef {
                    writer: "local".into(),
                    sequence: 1,
                },
                identity_maps: vec![],
                baseline: None,
                deferred: vec![],
            },
        ),
    )
    .expect("commit");
    let digest = persist_boundary(layout, |_, _| {});
    let err = apply_digest(&session, &digest).await.expect_err("preserve").to_string();
    assert!(err.contains("plan-proposal-preserve"), "{err}");
}

// Boundary application activates the candidate catalog and tree,
// reprojects `plan.yaml`, retains revisions, journals the applied
// fact, and invalidates the prior closed-plan epoch.
#[tokio::test]
async fn apply_boundary() {
    let session = authored().await;
    let layout = Layout::new(session.root());
    stamp_epoch(layout);
    let digest = persist_boundary(layout, |_, _| {});
    let body = apply_digest(&session, &digest).await.expect("apply");
    let AmendBody::Applied(applied) = body else {
        panic!("expected applied body");
    };
    assert_eq!(applied.digest, digest);
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    assert_eq!(names(&plan), ["hello", "world"]);
    assert_eq!(plan.leads_digest.as_ref(), Some(&applied.leads_digest));
    assert_eq!(plan.decomposition_digest.as_ref(), Some(&applied.decomposition_digest));
    assert!(layout.leads_revision_path(&applied.leads_digest).is_file());
    assert!(layout.decomp_revision_path(&applied.decomposition_digest).is_file());

    let events = project::plan::collect_events(layout).expect("events");
    assert!(events.iter().any(|event| {
        matches!(&event.kind, EventKind::PlanAmendApplied { digest: got } if *got == digest)
    }));
    match epoch::freshness(layout, &plan, &events).expect("freshness") {
        EpochFreshness::Stale { detail } => {
            assert!(detail.contains("`plan.yaml` digest drifted"), "{detail}");
        }
        other => panic!("expected stale epoch, got {other:?}"),
    }

    let catalog = Leads::load(&layout.leads_path()).expect("leads");
    let leads: Vec<_> = catalog.leads().iter().map(|lead| lead.lead.as_str()).collect();
    assert!(leads.contains(&"hello") && leads.contains(&"world"), "{leads:?}");

    let parked = Proposal::boundary_for(layout, "greeting").expect("boundary_for");
    assert!(parked.is_none(), "applied digest is skipped");
}

#[tokio::test]
async fn ownership_depends() {
    let session = authored().await;
    let layout = Layout::new(session.root());
    let digest = persist_boundary(layout, |_, _| {});
    apply_digest(&session, &digest).await.expect("split");
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    let expected = Frontiers::live(layout, &plan).expect("frontiers");
    let proposal = Proposal::Ownership(Ownership {
        version: PROPOSAL_VERSION,
        nearest: "root".into(),
        repair: Repair::DependsOn {
            predecessor: "hello".into(),
            successor: "world".into(),
        },
        expected,
    });
    let digest = proposal.save(layout).expect("save");
    apply_digest(&session, &digest).await.expect("ownership");
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    let world = plan.entries.iter().find(|entry| entry.name == "world").expect("world");
    assert!(world.depends_on.iter().any(|dep| dep.as_str() == "hello"), "{:?}", world.depends_on);
}

#[tokio::test]
async fn add_overlap_refuses() {
    let session = authored().await;
    let err = run::<plan::handlers::Add, _, _>(
        session.provider(),
        AddInput {
            name: "sibling".into(),
            depends_on: Vec::new(),
            sources: Vec::new(),
            description: None,
            context: vec![".".into()],
            authority_override: Vec::new(),
            target: "app".into(),
        },
    )
    .await
    .expect_err("overlap add")
    .to_string();
    assert!(err.contains("plan-mutation-ambiguous"), "{err}");
}

#[tokio::test]
async fn add_reprojects() {
    let session = authored().await;
    run::<plan::handlers::Add, _, _>(
        session.provider(),
        AddInput {
            name: "sibling".into(),
            depends_on: Vec::new(),
            sources: Vec::new(),
            description: None,
            context: vec!["sibling/**".into()],
            authority_override: Vec::new(),
            target: "app".into(),
        },
    )
    .await
    .expect("disjoint add");
    let layout = Layout::new(session.root());
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    assert_eq!(names(&plan), ["greeting", "sibling"]);
    let tree = Decomposition::load(&layout.decomposition_path()).expect("tree");
    project::plan::decomposition::matches_plan(&tree, &plan).expect("projection");
}

#[tokio::test]
async fn remove_unary_refuses() {
    let session = authored().await;
    let err = run::<plan::handlers::Remove, _, _>(
        session.provider(),
        RemoveInput {
            name: "greeting".into(),
        },
    )
    .await
    .expect_err("unary remove")
    .to_string();
    assert!(err.contains("plan-mutation-ambiguous"), "{err}");
}

// Review-only amend still writes through `Plan::amend` when a tree
// exists; topology `depends-on` reprojects after a split.
#[tokio::test]
async fn amend_review_and_deps() {
    let session = authored().await;
    let mut input = entry_input("greeting");
    input.description = Some("Sharper greeting.".into());
    run::<plan::handlers::Amend, _, _>(session.provider(), input).await.expect("review amend");
    let layout = Layout::new(session.root());
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    assert_eq!(plan.entries[0].description.as_deref(), Some("Sharper greeting."));

    let digest = persist_boundary(layout, |_, _| {});
    apply_digest(&session, &digest).await.expect("split");
    let mut input = entry_input("world");
    input.depends_on = Some(vec!["hello".into()]);
    run::<plan::handlers::Amend, _, _>(session.provider(), input).await.expect("depends-on");
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    let world = plan.entries.iter().find(|entry| entry.name == "world").expect("world");
    assert!(world.depends_on.iter().any(|dep| dep.as_str() == "hello"));
}

#[tokio::test]
async fn overlap_authors() {
    let session = authored().await;
    let layout = Layout::new(session.root());
    let digest = persist_boundary(layout, |_, _| {});
    apply_digest(&session, &digest).await.expect("split");
    let mut tree = Decomposition::load(&layout.decomposition_path()).expect("tree");
    tree.node_mut("hello").expect("hello").ownership = vec!["src/**".into()];
    tree.node_mut("world").expect("world").ownership = vec!["src/**".into()];
    tree.save(&layout.decomposition_path()).expect("overlap globs");
    for name in ["hello", "world"] {
        append_one(
            layout,
            &Event::new(
                jiff::Timestamp::now(),
                EventKind::SliceClaimed {
                    slice_name: name.into(),
                },
            ),
        )
        .expect("claim");
    }
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    let authored = project::plan::author_overlap(layout, &plan).expect("author");
    let digest = authored.expect("inert ownership proposal");
    let Proposal::Ownership(body) = Proposal::load(layout, &digest).expect("load") else {
        panic!("expected ownership");
    };
    assert_eq!(body.nearest, "root");
    let again = project::plan::author_overlap(layout, &plan).expect("idempotent");
    assert_eq!(again, Some(digest), "existing unapplied digest is returned");
}

#[tokio::test]
async fn combo_refuses() {
    let session = authored().await;
    let layout = Layout::new(session.root());
    let digest = persist_boundary(layout, |_, _| {});
    let mut input = proposal_input(&digest);
    input.description = Some("nope".into());
    let err = run::<plan::handlers::Amend, _, _>(session.provider(), input)
        .await
        .expect_err("combo")
        .to_string();
    assert!(err.contains("--proposal"), "{err}");
}
