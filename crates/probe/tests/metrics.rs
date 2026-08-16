//! RFC-96 D11: the coordination-cost projection over fabricated
//! facts, build records, and request telemetry — the report a
//! cap-comparison pair reads. Cost stays `unknown` until RFC-92.

use std::collections::BTreeMap;

use probe::metrics::{self, Accepted};
use project::config::Layout;
use project::journal::{ClosedPlanCoverage, Event, EventKind, FactEpochRef};
use project::snapshot::SnapshotId;
use tempfile::TempDir;

fn digest(fill: char) -> SnapshotId {
    SnapshotId::from_digest(&fill.to_string().repeat(64))
}

fn at(timestamp: &str, kind: EventKind) -> Event {
    Event::new(timestamp.parse().expect("timestamp"), kind)
}

fn committed(target: &str) -> EventKind {
    EventKind::TargetMergeWaveCommitted {
        target: target.into(),
        digest: digest('a').to_string(),
        members: vec!["alpha".into()],
        base: digest('b'),
        result: digest('c'),
        commit_authorization: FactEpochRef {
            writer: "local".into(),
            sequence: 1,
        },
        identity_maps: Vec::new(),
        baseline: None,
        deferred: Vec::new(),
    }
}

fn events() -> Vec<Event> {
    vec![
        at(
            "2026-08-16T10:00:00Z",
            EventKind::PlanExecuteStarted {
                coverage: ClosedPlanCoverage::ClosedPlan {
                    plan_digest: digest('d').to_string(),
                    refinements: BTreeMap::new(),
                },
                discovery_digest: None,
            },
        ),
        at(
            "2026-08-16T10:00:05Z",
            EventKind::TargetWaveOpened {
                target: "default".into(),
                digest: digest('a').to_string(),
                members: vec!["alpha".into(), "beta".into()],
            },
        ),
        at(
            "2026-08-16T10:00:05Z",
            EventKind::SliceBuildStarted {
                slice_name: "alpha".into(),
            },
        ),
        at(
            "2026-08-16T10:00:06Z",
            EventKind::SliceBuildStarted {
                slice_name: "beta".into(),
            },
        ),
        // A rebuild: the same slice starts a second attempt.
        at(
            "2026-08-16T10:00:20Z",
            EventKind::SliceBuildStarted {
                slice_name: "alpha".into(),
            },
        ),
        at("2026-08-16T10:00:42Z", committed("default")),
        // A later commit must not displace the first-accepted time.
        at("2026-08-16T10:03:00Z", committed("default")),
    ]
}

#[test]
fn coordination_projects() {
    let tmp = TempDir::new().expect("tempdir");
    let layout = Layout::new(tmp.path());
    let requests = BTreeMap::from([("synthesis".to_string(), 3), ("report".to_string(), 2)]);
    let accepted = vec![Accepted {
        target: "default".into(),
        cid: digest('c'),
        requirements: 4,
        bytes: 1024,
    }];

    let report =
        metrics::coordination(layout, &events(), requests, accepted).expect("the report projects");

    assert_eq!(report.first_accepted.map(|d| d.as_secs()), Some(42), "{report:?}");
    assert_eq!(report.builds, 3);
    assert_eq!(report.rebuilds, 1, "three starts over two distinct slices");
    assert_eq!(report.waves.get("default"), Some(&1));
    assert_eq!(report.requests.get("synthesis"), Some(&3));
    assert_eq!(report.accepted.len(), 1);
    assert_eq!(report.accepted[0].requirements, 4);
    assert!(report.cost.is_none(), "cost stays unknown until RFC-92 usage facts land");
    assert!(report.heat.is_empty(), "no build records, no heat");
}

// An empty fact log projects a well-formed all-unknown report rather
// than failing — the runner renders it after every completed case.
#[test]
fn empty_facts_project() {
    let tmp = TempDir::new().expect("tempdir");
    let layout = Layout::new(tmp.path());

    let report = metrics::coordination(layout, &[], BTreeMap::new(), Vec::new())
        .expect("an empty run projects");

    assert!(report.first_accepted.is_none());
    assert_eq!(report.builds, 0);
    assert_eq!(report.rebuilds, 0);
    assert!(report.waves.is_empty());
    assert!(report.heat.is_empty());
}

#[test]
fn heat_counts_touches() {
    use project::build_record::BuildRecord;
    use project::seam::wire::{BUILD_VERSION, BuildReport, BuildStatus};

    let tmp = TempDir::new().expect("tempdir");
    let layout = Layout::new(tmp.path());
    let record = |touched: &[&str], wave: char| BuildRecord {
        base: digest('b'),
        result: digest('c'),
        touched: touched.iter().map(ToString::to_string).collect(),
        wave: digest(wave),
        report: BuildReport {
            version: BUILD_VERSION,
            slice: "alpha".into(),
            target: "mock".into(),
            status: BuildStatus::Success,
            findings: Vec::new(),
            outputs: Vec::new(),
            ui_surface: None,
            covered: Vec::new(),
        },
        deferred: Vec::new(),
    };
    let alpha = layout.slice_dir("alpha");
    std::fs::create_dir_all(&alpha).expect("mkdir slice");
    record(&["src/lib.rs", "src/hot.rs"], '1').write(&alpha).expect("first record");
    record(&["src/hot.rs"], '2').write(&alpha).expect("second record");

    let report = metrics::coordination(layout, &[], BTreeMap::new(), Vec::new())
        .expect("the report projects");

    assert_eq!(report.heat.get("src/hot.rs"), Some(&2));
    assert_eq!(report.heat.get("src/lib.rs"), Some(&1));
}
