//! Decision validation and promotion through public slice operations.

use std::fs;
use std::path::Path;

use mock::invoke::run;
use mock::session::Session;
use project::seam::Workspaces as _;

/// Rule ids carried by a failing validate operation's report.
fn report_rule_ids(err: &project::handler::Error) -> Vec<String> {
    let project::handler::Error::Report {
        body: project::handler::FailureBody::Findings(body),
        ..
    } = err
    else {
        panic!("expected findings report error, got {err:?}");
    };
    body.report().findings.iter().filter_map(|finding| finding.rule_id.clone()).collect()
}

const BODY: &str = "# Choice\n\n## Context\n\nContext.\n\n## Decision\n\nDecision.\n\n## Consequences\n\nConsequences.\n";

fn write_decision(path: &Path, frontmatter: &str) {
    fs::write(path, format!("---\n{frontmatter}---\n{BODY}")).expect("write decision");
}

#[tokio::test]
async fn orphan_supersede_reported() {
    let project = Session::scripted("mock", Vec::new());
    let decisions = project.root().join(".emery/slices/demo/decisions");
    fs::create_dir_all(&decisions).expect("create slice decisions");
    write_decision(
        &decisions.join("new-choice.md"),
        "slug: new-choice\nstatus: accepted\nsupersedes: [DEC-9999]\n",
    );

    let err = run::<slice::handlers::Validate, _, _>(
        project.provider(),
        slice::handlers::ValidateInput {
            name: "demo".to_string(),
        },
    )
    .await
    .expect_err("orphan supersede must fail validation");

    let ids = report_rule_ids(&err);
    assert!(ids.iter().any(|id| id == "decision-supersede-orphan"), "{ids:?}");
}

#[tokio::test]
async fn merge_promotes_and_supersedes() {
    let project = Session::scripted("mock", Vec::new());
    let slice = project.root().join(".emery/slices/demo");
    let staged = slice.join("decisions");
    let baseline = project.root().join(".emery/decisions");
    fs::create_dir_all(&staged).expect("create staged decisions");
    fs::create_dir_all(&baseline).expect("create baseline decisions");
    fs::write(slice.join("metadata.yaml"), "target: mock\ntouched-specs: []\n")
        .expect("stage built metadata");
    write_decision(
        &baseline.join("DEC-0001-old-choice.md"),
        "id: DEC-0001\nslug: old-choice\nstatus: accepted\nslice: earlier\ndate: 2026-01-01\n",
    );
    write_decision(
        &staged.join("new-choice.md"),
        "slug: new-choice\nstatus: accepted\nsupersedes: [DEC-0001]\n",
    );
    // A merge needs a fact-substrate build record + revalidated
    // one-member wave; this test stages `built` by hand, so freeze the
    // fixture tree as a no-op patch (base == result), open a matching
    // wave, and wrap both in a BuildRecord.
    let snapshot = project.provider().freeze().await.expect("freeze fixture tree");
    let layout = project::config::Layout::new(project.root());
    let wave = project::wave::Wave::one_member(
        "demo",
        snapshot.clone(),
        project::name::SliceName::from("demo"),
        project::snapshot::SnapshotId::from_digest(&"b".repeat(64)),
        vec![],
        project::wave::EpochRef {
            writer: "local".into(),
            sequence: 0,
        },
    );
    let opened = wave
        .open(layout, jiff::Timestamp::from_second(1_700_000_000).expect("ts"))
        .expect("open wave");
    let record = project::build_record::BuildRecord::from_capture(
        project::snapshot::CodePatch {
            base: snapshot.clone(),
            result: snapshot,
            touched: vec![],
        },
        opened.digest,
        project::seam::wire::BuildReport {
            version: 1,
            slice: "demo".into(),
            target: "mock@0.0.0".into(),
            status: project::seam::wire::BuildStatus::Success,
            findings: vec![],
            outputs: vec![],
            ui_surface: None,
        },
    );
    record.write(&slice).expect("stage build record");

    let body = run::<slice::handlers::MergeRun, _, _>(
        project.provider(),
        slice::handlers::MergeRunInput {
            name: "demo".to_string(),
            allow_composition_replace: false,
            preview: false,
            conflict_check: false,
        },
    )
    .await
    .expect("standalone merge succeeds");
    let slice::handlers::MergeRunBody::Merged(body) = body else {
        panic!("default merge mode commits: {body:?}");
    };

    assert_eq!(body.decisions, ["DEC-0002"]);
    assert!(baseline.join("DEC-0002-new-choice.md").is_file());
    let old =
        fs::read_to_string(baseline.join("DEC-0001-old-choice.md")).expect("superseded baseline");
    assert!(old.contains("status: superseded"), "{old}");
    assert!(old.contains("superseded-by: DEC-0002"), "{old}");
}
