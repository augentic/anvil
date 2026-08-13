//! RFC-86a Acceptance #6 / D5 — debt conservation through the
//! plan-owned loop: deferred rows fold into the baseline with the
//! self-describing note, the wave-commit fact snapshots the member
//! set, and `plan archive` renders the carried-debt summary without
//! ever blocking on debt.

mod support;

use std::fs;

use artifacts::spec::provenance::RequirementStatus;
use change::plan;
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{
    DEFAULT_WRITER, DeferredMember, Event, EventKind, FactEpochRef, append_for, read_union,
};
use project::snapshot::SnapshotId;

fn cid(ch: char) -> SnapshotId {
    SnapshotId::from_digest(&ch.to_string().repeat(64))
}

/// The minimal profile whose refine mints one `[unknown]` row
/// (`greeting/REQ-001`).
fn unknown_session() -> Session {
    Session::bare(vec![mock::answers::greeting_grouping(), mock::answers::greeting_unknown_synth()])
}

async fn scaffold(session: &Session) {
    run::<project::init::handlers::Init, _, _>(
        session.provider(),
        project::init::handlers::InitInput {
            adapter: Some("mock".to_string()),
            name: Some("demo".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("init");
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding(),
            intent: None,
            from: None,
            wave: None,
            force: false,
        },
    )
    .await
    .expect("author");
}

async fn execute(session: &Session) {
    let drained = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect("execute drains");
    assert_eq!(drained.status, "drained");
}

/// The deferred member set on the newest wave-commit fact for `slice`.
fn wave_deferred(root: &std::path::Path, slice: &str) -> Vec<DeferredMember> {
    read_union(Layout::new(root))
        .expect("union")
        .into_iter()
        .rev()
        .find_map(|event| match event.kind {
            EventKind::TargetMergeWaveCommitted {
                members, deferred, ..
            } if members.iter().any(|m| m.as_str() == slice) => Some(deferred),
            _ => None,
        })
        .expect("target.merge.wave-committed")
}

fn render_archive(body: &plan::handlers::ArchiveBody) -> String {
    let mut out = Vec::new();
    project::handler::Render::render(body, &mut out).expect("render");
    String::from_utf8(out).expect("utf8")
}

/// Acceptance 6: the deferred unknown drains through
/// merge; the baseline debt row is self-describing; the wave fact
/// names the accepted debt; the unforced archive succeeds and renders
/// the summary.
#[tokio::test]
async fn gate_debt_conserved() {
    let session = unknown_session();
    let root = session.root().to_path_buf();
    scaffold(&session).await;
    support::refine(&session, "greeting").await.expect("refine");
    execute(&session).await;

    // The folded baseline row lives on the accepted CID.
    let tree = session.materialize_accepted("demo").await;
    let baseline = fs::read_to_string(tree.path().join(".emery/specs/greeting/spec.md"))
        .expect("merged baseline");
    assert!(baseline.contains("greeting error handling [unknown]"), "{baseline}");
    assert!(baseline.contains("Status: unknown"), "{baseline}");
    assert!(baseline.contains("ID: REQ-001"), "{baseline}");
    assert!(baseline.contains("Note: deferred — change: demo; date: "), "{baseline}");
    assert!(baseline.contains("; reason: deferred at the build gate under epoch "), "{baseline}");

    // The wave-commit fact snapshots exactly the debt it accepted,
    // digest-bound to the covering deferral fact.
    let deferred = wave_deferred(&root, "greeting");
    assert_eq!(deferred.len(), 1, "{deferred:?}");
    assert_eq!(deferred[0].req, "REQ-001");
    assert_eq!(deferred[0].status, RequirementStatus::Unknown);
    let fact_digest = read_union(Layout::new(&root))
        .expect("union")
        .into_iter()
        .find_map(|event| match event.kind {
            EventKind::GapDeferred {
                requirement_digest, ..
            } => Some(requirement_digest),
            _ => None,
        })
        .expect("gap.deferred fact");
    assert_eq!(deferred[0].requirement_digest, fact_digest);

    // Archive succeeds with debt (advisory, unforced) and renders the
    // carried-debt summary.
    let archived = run::<plan::handlers::Archive, _, _>(
        session.provider(),
        plan::handlers::ArchiveInput::default(),
    )
    .await
    .expect("archive never blocks on debt");
    assert_eq!(archived.debt.len(), 1, "{:?}", archived.debt);
    let row = &archived.debt[0];
    assert_eq!(row.slice, "greeting");
    assert_eq!(row.req, "REQ-001");
    assert_eq!(row.status, RequirementStatus::Unknown);
    let detail = row.deferral.as_ref().expect("covering deferral detail");
    assert!(
        detail.reason.starts_with("deferred at the build gate under epoch "),
        "{}",
        detail.reason
    );
    let text = render_archive(&archived);
    assert!(text.contains("carried debt (1 deferred):"), "{text}");
    assert!(text.contains("unknown:"), "{text}");
    assert!(text.contains("greeting/REQ-001 — deferred at the build gate under epoch "), "{text}");
}

/// Acceptance 6, pre-covered path: a durable deferral fact minted
/// before execute covers the gate, and the note folds its reason —
/// not a fresh gate-time synthesis.
#[tokio::test]
async fn note_carries_reason() {
    let session = unknown_session();
    let root = session.root().to_path_buf();
    scaffold(&session).await;
    support::refine(&session, "greeting").await.expect("refine");

    // A pre-existing deferral fact minted before execute.
    let gaps = run::<plan::handlers::Gaps, _, _>(session.provider(), plan::handlers::GapsInput {})
        .await
        .expect("gaps");
    let row = gaps
        .rows
        .iter()
        .find(|row| row.slice == "greeting" && row.req == "REQ-001")
        .expect("greeting/REQ-001 gap row");
    let fact = Event::new(
        jiff::Timestamp::now(),
        EventKind::GapDeferred {
            slice: "greeting".into(),
            req: "REQ-001".into(),
            requirement_digest: row.requirement_digest.clone().expect("digest-bearing row"),
            reason: "carried to the next change".into(),
        },
    );
    append_for(Layout::new(&root), DEFAULT_WRITER, &[fact]).expect("append deferral");

    // The durable deferral covers the gate — no new mint.
    execute(&session).await;

    let tree = session.materialize_accepted("demo").await;
    let baseline = fs::read_to_string(tree.path().join(".emery/specs/greeting/spec.md"))
        .expect("merged baseline");
    assert!(baseline.contains("Note: deferred — change: demo; date: "), "{baseline}");
    assert!(baseline.contains("; reason: carried to the next change"), "{baseline}");
    let deferred = wave_deferred(&root, "greeting");
    assert_eq!(deferred.len(), 1, "{deferred:?}");
}

/// Acceptance 7 (RFC-86a D9): after a debt-carrying merge, `emery
/// debt` reads the baseline alone and lists the carried row with every
/// note field — reason, originating change, and age — and the
/// next `plan author` renders the same inventory into the `change.md`
/// review prose it authors.
#[tokio::test]
async fn debt_after_merge() {
    // Three judgment answers: the first author's grouping, the refine
    // synthesis that mints the unknown, and the corrective change's
    // author grouping.
    let session = Session::bare(vec![
        mock::answers::greeting_grouping(),
        mock::answers::greeting_unknown_synth(),
        mock::answers::greeting_grouping(),
    ]);
    scaffold(&session).await;
    support::refine(&session, "greeting").await.expect("refine");
    execute(&session).await;

    // Folded note lives on the accepted CID. `emery debt` still reads
    // checkout `.emery/specs/` (dark until step 18 re-homes it).
    let tree = session.materialize_accepted("demo").await;
    let rows = slice::debt::baseline(&tree.path().join(".emery/specs"), jiff::Timestamp::now())
        .expect("debt from accepted CID");
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row.domain, "greeting");
    assert_eq!(row.req, "REQ-001");
    assert_eq!(row.status, RequirementStatus::Unknown);
    let note = row.deferral.as_ref().expect("self-describing note");
    assert_eq!(note.change, "demo");
    assert!(note.reason.starts_with("deferred at the build gate under epoch "), "{}", note.reason);
    assert!(note.age_days <= 1, "the deferral happened just now: {note:?}");

    // Close the change, then author the next one. `plan author` still
    // reads checkout `.emery/specs/` for `## Carried debt` (dark until
    // step 18 re-homes it onto the accepted CID).
    run::<plan::handlers::Archive, _, _>(
        session.provider(),
        plan::handlers::ArchiveInput::default(),
    )
    .await
    .expect("archive");
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding(),
            intent: None,
            from: None,
            wave: None,
            force: false,
        },
    )
    .await
    .expect("corrective author");
}

/// D6 visibility at the boundary: the archive summary renders
/// deferred conflicts separately from deferred unknowns, joined from
/// staged wave and deferral facts.
#[tokio::test]
async fn archive_splits_kinds() {
    let session = Session::bare(Vec::new());
    let root = session.root().to_path_buf();
    run::<project::init::handlers::Init, _, _>(
        session.provider(),
        project::init::handlers::InitInput {
            adapter: Some("mock".to_string()),
            name: Some("demo".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("init");
    support::plan_with_changes(vec![support::change("a")])
        .save(&Layout::new(&root).plan_path())
        .expect("save plan");

    let ts = jiff::Timestamp::from_second(1_700_000_000).expect("timestamp");
    let deferral = |req: &str, digest: &str, reason: &str| {
        Event::new(
            ts,
            EventKind::GapDeferred {
                slice: "a".into(),
                req: req.into(),
                requirement_digest: digest.into(),
                reason: reason.into(),
            },
        )
    };
    let committed = Event::new(
        ts,
        EventKind::TargetMergeWaveCommitted {
            target: "mock".into(),
            digest: "sha256:abc".into(),
            members: vec!["a".into()],
            base: cid('a'),
            result: cid('b'),
            commit_authorization: FactEpochRef {
                writer: DEFAULT_WRITER.into(),
                sequence: 1,
            },
            identity_maps: vec![],
            baseline: None,
            deferred: vec![
                DeferredMember {
                    req: "REQ-007".into(),
                    status: RequirementStatus::Unknown,
                    requirement_digest: "sha256:d1".into(),
                },
                DeferredMember {
                    req: "REQ-008".into(),
                    status: RequirementStatus::Conflict,
                    requirement_digest: "sha256:d2".into(),
                },
            ],
        },
    );
    append_for(
        Layout::new(&root),
        DEFAULT_WRITER,
        &[
            deferral("REQ-001", "sha256:d1", "reset path deferred"),
            deferral("REQ-002", "sha256:d2", "TTL tie deferred"),
            committed,
        ],
    )
    .expect("stage facts");

    // Entries never ran: `--force` archives anyway — debt is advisory
    // and never a gate.
    let archived = run::<plan::handlers::Archive, _, _>(
        session.provider(),
        plan::handlers::ArchiveInput { force: true },
    )
    .await
    .expect("forced archive with staged debt");
    assert_eq!(archived.debt.len(), 2, "{:?}", archived.debt);
    let detail = archived.debt[0].deferral.as_ref().expect("covering deferral detail");
    assert!(detail.age_days > 300, "aged deferral: {:?}", archived.debt[0]);

    let text = render_archive(&archived);
    let unknown_at = text.find("unknown:").expect("unknown heading");
    let conflict_at = text.find("conflict:").expect("conflict heading");
    assert!(unknown_at < conflict_at, "unknowns render before conflicts: {text}");
    assert!(text.contains("a/REQ-007 — reset path deferred"), "{text}");
    assert!(text.contains("a/REQ-008 — TTL tie deferred"), "{text}");
    let conflict_line = text.lines().find(|l| l.contains("REQ-008")).expect("conflict row");
    assert!(
        text.lines().position(|l| l.contains("conflict:")).expect("conflict heading position")
            < text.lines().position(|l| l == conflict_line).expect("conflict row position"),
        "the conflict row renders under the conflict heading: {text}"
    );
}

/// A wave snapshot member whose covering `gap.deferred` fact is
/// missing (a pruned or damaged journal) still renders in the archive
/// summary — as a placeholder row, never silently dropped.
#[tokio::test]
async fn archive_join_miss() {
    let session = Session::bare(Vec::new());
    let root = session.root().to_path_buf();
    run::<project::init::handlers::Init, _, _>(
        session.provider(),
        project::init::handlers::InitInput {
            adapter: Some("mock".to_string()),
            name: Some("demo".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("init");
    support::plan_with_changes(vec![support::change("a")])
        .save(&Layout::new(&root).plan_path())
        .expect("save plan");

    // A committed wave snapshotting one debt member, with no
    // gap.deferred fact anywhere in the journal.
    let committed = Event::new(
        jiff::Timestamp::from_second(1_700_000_000).expect("timestamp"),
        EventKind::TargetMergeWaveCommitted {
            target: "mock".into(),
            digest: "sha256:abc".into(),
            members: vec!["a".into()],
            base: cid('a'),
            result: cid('b'),
            commit_authorization: FactEpochRef {
                writer: DEFAULT_WRITER.into(),
                sequence: 1,
            },
            identity_maps: vec![],
            baseline: None,
            deferred: vec![DeferredMember {
                req: "REQ-009".into(),
                status: RequirementStatus::Unknown,
                requirement_digest: "sha256:orphan".into(),
            }],
        },
    );
    append_for(Layout::new(&root), DEFAULT_WRITER, &[committed]).expect("stage fact");

    let archived = run::<plan::handlers::Archive, _, _>(
        session.provider(),
        plan::handlers::ArchiveInput { force: true },
    )
    .await
    .expect("archive with an orphan wave member");
    assert_eq!(archived.debt.len(), 1, "{:?}", archived.debt);
    let row = &archived.debt[0];
    assert_eq!(row.slice, "a");
    assert_eq!(row.req, "REQ-009");
    assert_eq!(row.status, RequirementStatus::Unknown);
    assert!(row.deferral.is_none(), "join miss carries no detail: {row:?}");

    let text = render_archive(&archived);
    assert!(text.contains("carried debt (1 deferred):"), "{text}");
    assert!(text.contains("a/REQ-009 — reason unavailable (no covering deferral fact)"), "{text}");
}
