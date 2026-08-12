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
use project::GapPolicy;
use project::config::Layout;
use project::journal::{
    DEFAULT_WRITER, DeferralOrigin, DeferredMember, Event, EventKind, FactEpochRef, append_for,
    read_union,
};

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
            force: false,
        },
    )
    .await
    .expect("author");
}

async fn execute(session: &Session, gap_policy: Option<GapPolicy>) {
    let drained = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput { gap_policy },
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
                slice_name, deferred, ..
            } if slice_name.as_str() == slice => Some(deferred),
            _ => None,
        })
        .expect("target.merge.wave-committed")
}

fn render_archive(body: &plan::handlers::ArchiveBody) -> String {
    let mut out = Vec::new();
    project::handler::Render::render(body, &mut out).expect("render");
    String::from_utf8(out).expect("utf8")
}

/// Acceptance 6, policy path: the deferred unknown drains through
/// merge; the baseline debt row is self-describing; the wave fact
/// names the accepted debt; the unforced archive succeeds and renders
/// the summary.
#[tokio::test]
async fn policy_debt_conserved() {
    let session = unknown_session();
    let root = session.root().to_path_buf();
    scaffold(&session).await;
    execute(&session, Some(GapPolicy::Defer)).await;

    // The folded baseline row: status preserved, final baseline id,
    // and the appended self-describing note (origin, change, date,
    // reason — reason last so free text stays parseable).
    let baseline =
        fs::read_to_string(root.join(".emery/specs/greeting/spec.md")).expect("merged baseline");
    assert!(baseline.contains("greeting error handling [unknown]"), "{baseline}");
    assert!(baseline.contains("Status: unknown"), "{baseline}");
    assert!(baseline.contains("ID: REQ-001"), "{baseline}");
    assert!(
        baseline.contains("Note: deferred — origin: policy; change: demo; date: "),
        "{baseline}"
    );
    assert!(baseline.contains("; reason: deferred by gap-policy under epoch "), "{baseline}");

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
    assert_eq!(detail.origin, DeferralOrigin::Policy);
    assert!(detail.reason.starts_with("deferred by gap-policy under epoch "), "{}", detail.reason);
    let text = render_archive(&archived);
    assert!(text.contains("carried debt (1 deferred):"), "{text}");
    assert!(text.contains("unknown:"), "{text}");
    assert!(text.contains("greeting/REQ-001 — deferred by gap-policy under epoch "), "{text}");
}

/// Acceptance 6, operator path: the note carries the operator's
/// reason and origin.
#[tokio::test]
async fn note_reason_origin() {
    let session = unknown_session();
    let root = session.root().to_path_buf();
    scaffold(&session).await;
    support::refine(&session, "greeting").await.expect("refine");
    run::<plan::handlers::Defer, _, _>(
        session.provider(),
        plan::handlers::DeferInput {
            selectors: vec![plan::handlers::DeferSelector {
                slice: "greeting".into(),
                req: "REQ-001".into(),
            }],
            reason: Some("carried to the next change".to_string()),
            retract: false,
        },
    )
    .await
    .expect("defer");

    // Strict default: the durable deferral covers the gate.
    execute(&session, None).await;

    let baseline =
        fs::read_to_string(root.join(".emery/specs/greeting/spec.md")).expect("merged baseline");
    assert!(
        baseline.contains("Note: deferred — origin: operator; change: demo; date: "),
        "{baseline}"
    );
    assert!(baseline.contains("; reason: carried to the next change"), "{baseline}");
    let deferred = wave_deferred(&root, "greeting");
    assert_eq!(deferred.len(), 1, "{deferred:?}");
}

/// Acceptance 7 (RFC-86a D9): after a debt-carrying merge, `emery
/// debt` reads the baseline alone and lists the carried row with every
/// note field — reason, origin, originating change, and age — and the
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
    let root = session.root().to_path_buf();
    scaffold(&session).await;
    execute(&session, Some(GapPolicy::Defer)).await;

    // The projection reads the merged baseline note, never fact logs.
    let debt =
        run::<slice::handlers::Debt, _, _>(session.provider(), slice::handlers::DebtInput {})
            .await
            .expect("debt projects");
    assert_eq!(debt.rows.len(), 1, "{:?}", debt.rows);
    let row = &debt.rows[0];
    assert_eq!(row.domain, "greeting");
    assert_eq!(row.req, "REQ-001");
    assert_eq!(row.status, RequirementStatus::Unknown);
    let note = row.deferral.as_ref().expect("self-describing note");
    assert_eq!(note.origin, DeferralOrigin::Policy);
    assert_eq!(note.change, "demo");
    assert!(note.reason.starts_with("deferred by gap-policy under epoch "), "{}", note.reason);
    assert!(note.age_days <= 1, "the deferral happened just now: {note:?}");

    // Close the change, then author the corrective one: the review
    // prose carries the backlog.
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
            force: false,
        },
    )
    .await
    .expect("corrective author");

    let brief = fs::read_to_string(root.join("change.md")).expect("change.md");
    assert!(brief.contains("## Carried debt"), "{brief}");
    assert!(brief.contains("Unknowns:"), "{brief}");
    assert!(
        brief.contains("- greeting/REQ-001 greeting error handling — deferred by gap-policy"),
        "{brief}"
    );
    assert!(brief.contains("(policy, change demo, "), "{brief}");
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
                origin: DeferralOrigin::Operator,
            },
        )
    };
    let committed = Event::new(
        ts,
        EventKind::TargetMergeWaveCommitted {
            target: "mock".into(),
            digest: "sha256:abc".into(),
            slice_name: "a".into(),
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
            slice_name: "a".into(),
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
