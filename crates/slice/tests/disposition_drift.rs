//! RFC-86a D4 — the disposition-drift comparison runs over unique
//! digests: identical requirement bodies legally share one digest
//! (D2), so two identical-bodied deferred rows must read as fresh
//! against a build record that consumed the digest once.

use jiff::Timestamp;
use mock::session::Session;
use project::build_record::BuildRecord;
use project::config::Layout;
use project::journal::{DEFAULT_WRITER, DeferralOrigin, Event, EventKind, append_for};
use project::name::SliceName;
use project::seam::wire::{BuildReport, BuildStatus};
use project::slice::RequirementBody;
use project::snapshot::{CodePatch, SnapshotId};
use project::wave::{EpochRef, Wave};

fn ts() -> Timestamp {
    Timestamp::from_second(1_700_000_000).expect("valid timestamp")
}

const TITLE: &str = "greeting error handling";
const STATEMENT: &str = "behaviour is not evidenced";

fn body_digest() -> String {
    RequirementBody {
        title: TITLE,
        statement: STATEMENT,
        scenarios: &[],
        notes: None,
    }
    .digest()
}

#[tokio::test]
async fn same_body_no_drift() {
    let session = Session::scripted("mock", Vec::new());
    let root = session.root().to_path_buf();

    // Plan entry covering the slice — the live projection reads it.
    std::fs::write(root.join("plan.yaml"), "name: demo\nslices:\n  - name: greeting\n")
        .expect("plan.yaml");

    // Two requirements with identical bodies mint one digest (D2), so
    // the single deferral fact covers both rows.
    let slice_dir = Layout::new(&root).slice_dir("greeting");
    std::fs::create_dir_all(&slice_dir).expect("slice dir");
    std::fs::write(slice_dir.join("metadata.yaml"), "target: mock\n").expect("metadata");
    std::fs::write(
        slice_dir.join("model.yaml"),
        format!(
            "requirements:\n  - id: REQ-001\n    title: {TITLE}\n    statement: {STATEMENT}\n    \
             status: unknown\n  - id: REQ-002\n    title: {TITLE}\n    statement: {STATEMENT}\n    \
             status: unknown\n"
        ),
    )
    .expect("model.yaml");

    let digest = body_digest();
    append_for(
        Layout::new(&root),
        DEFAULT_WRITER,
        &[Event::new(
            ts(),
            EventKind::GapDeferred {
                slice: "greeting".into(),
                req: "REQ-001".into(),
                requirement_digest: digest.clone(),
                reason: "carried to the next change".into(),
                origin: DeferralOrigin::Operator,
            },
        )],
    )
    .expect("deferral fact");

    // The wave-authorized record consumed the digest once (deduped on
    // entry); the live projection's two identical rows dedup to the
    // same set.
    let base = SnapshotId::from_digest(&"a".repeat(64));
    let opened = Wave::one_member(
        "demo",
        base.clone(),
        SliceName::from("greeting"),
        SnapshotId::from_digest(&"b".repeat(64)),
        vec![],
        EpochRef {
            writer: "local".into(),
            sequence: 0,
        },
    )
    .open(Layout::new(&root), ts())
    .expect("open wave");
    let record = BuildRecord::from_capture(
        CodePatch {
            base: base.clone(),
            result: base,
            touched: vec![],
        },
        opened.digest,
        BuildReport {
            version: 1,
            slice: "greeting".into(),
            target: "mock@0.0.0".into(),
            status: BuildStatus::Success,
            findings: vec![],
            outputs: vec![],
            ui_surface: None,
            covered: vec![],
        },
        vec![digest.clone(), digest.clone()],
    );
    assert_eq!(record.deferred, [digest], "record binds the unique digest set");
    record.write(&slice_dir).expect("write record");

    assert!(
        !slice::dispositions_drifted(Layout::new(&root), &slice_dir, "greeting").expect("probe"),
        "identical bodies share one digest — no drift"
    );
}
