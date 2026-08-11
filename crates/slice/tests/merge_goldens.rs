//! Merge-engine goldens over the public merge orchestration: each
//! fixture stages one staged spec (plus optional baseline) and runs
//! the full merge phase — preflight gate, identity finalization (a
//! no-op here: no `model.yaml`), deterministic commit, postflight
//! gate — asserting the merged output or aggregated conflict text
//! against the checked-in goldens under `tests/fixtures/spec-*`.

use std::fs;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use mock::session::Session;
use project::name::SliceName;
use project::seam::Workspaces as _;
use project::snapshot::SnapshotId;
use project::wave::{EpochRef, Wave};
use slice::orchestrate::MergeOutcome;

const SLICE: &str = "golden";

const MERGE_CASES: &[&str] = &[
    "spec-single-req",
    "spec-multi-req",
    "spec-new-baseline",
    "spec-modified",
    "spec-removed",
    "spec-renamed",
    "spec-all-sections",
];

const VALIDATION_CASES: &[&str] = &["spec-validation-ok", "spec-validation-fails"];

fn ts() -> Timestamp {
    Timestamp::from_second(1_700_000_000).expect("valid timestamp")
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Stage the fixture's baseline (when present) and delta spec, plus the
/// minimal `metadata.yaml` the merge completion gate loads.
fn stage(root: &Path, case: &Path, name: &str) {
    let baseline = case.join("baseline.md");
    if baseline.is_file() {
        let destination = root.join(".emery/specs").join(name);
        fs::create_dir_all(&destination).expect("create baseline directory");
        fs::copy(baseline, destination.join("spec.md")).expect("stage baseline");
    }

    let slice_dir = root.join(".emery/slices").join(SLICE);
    let destination = slice_dir.join("specs").join(name);
    fs::create_dir_all(&destination).expect("create delta directory");
    let source = case.join("delta.md");
    if source.is_file() {
        fs::copy(source, destination.join("spec.md")).expect("stage delta");
    } else {
        fs::write(destination.join("spec.md"), "").expect("stage empty delta");
    }

    fs::write(slice_dir.join("metadata.yaml"), "target: mock\n").expect("stage metadata");
}

/// Open the one-member wave and write the build record it names, so
/// the merge phase resolves its authorized build from the fact union.
fn stage_wave_and_record(session: &Session, base: SnapshotId) {
    let layout = project::config::Layout::new(session.root());
    let wave = Wave::one_member(
        "demo",
        base.clone(),
        SliceName::from(SLICE),
        SnapshotId::from_digest(&"b".repeat(64)),
        vec![],
        EpochRef {
            writer: "local".into(),
            sequence: 0,
        },
    );
    let opened = wave.open(layout, ts()).expect("open wave");
    let record = project::build_record::BuildRecord::from_capture(
        project::snapshot::CodePatch {
            base: base.clone(),
            result: base,
            touched: vec![],
        },
        opened.digest,
        project::seam::wire::BuildReport {
            version: 1,
            slice: SLICE.into(),
            target: "mock@0.0.0".into(),
            status: project::seam::wire::BuildStatus::Success,
            findings: vec![],
            outputs: vec![],
            ui_surface: None,
            covered: vec![],
        },
        vec![],
    );
    record.write(&layout.slice_dir(SLICE)).expect("write build record");
}

/// Stage one fixture into a fresh session and run the merge phase.
/// Returns the session alongside the outcome so callers can assert
/// against the written baseline before the tempdir drops.
async fn run_case(name: &str, spec_name: &str) -> (Session, Result<MergeOutcome, error::Error>) {
    let session = Session::scripted("mock", Vec::new());
    let snapshot = session.provider().freeze().await.expect("freeze");
    stage(session.root(), &fixtures().join(name), spec_name);
    stage_wave_and_record(&session, snapshot);
    let layout = project::config::Layout::new(session.root());
    let outcome = slice::orchestrate::merge(session.provider(), layout, ts(), SLICE, false).await;
    (session, outcome)
}

fn assert_golden(path: &Path, actual: &str) {
    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        fs::write(path, actual).expect("regenerate golden");
    }
    let expected = fs::read_to_string(path).expect("read golden");
    assert_eq!(actual, expected, "golden mismatch: {}", path.display());
}

#[tokio::test]
async fn merged_outputs() {
    for name in MERGE_CASES {
        let case = fixtures().join(name);
        let error_golden = case.join("expected-merge-errors.txt");
        let (session, outcome) = run_case(name, SLICE).await;
        match outcome {
            Ok(merged) => {
                if error_golden.is_file() {
                    assert_golden(&error_golden, "");
                }
                assert_eq!(merged.merged.len(), 1, "{name}: expected one merged entry");
                let output = &merged.merged[0].result.output;
                assert_golden(&case.join("expected-merged.md"), output);
                let written = session.root().join(".emery/specs").join(SLICE).join("spec.md");
                let on_disk = fs::read_to_string(written).expect("read written baseline");
                assert_eq!(&on_disk, output, "{name}: written baseline diverges from output");
            }
            Err(err) if error_golden.is_file() => {
                assert_golden(&error_golden, &format!("{err}\n"));
            }
            Err(err) => panic!("{name}: merge failed: {err:?}"),
        }
    }
}

#[tokio::test]
async fn validation_outputs() {
    for name in VALIDATION_CASES {
        let case = fixtures().join(name);
        let (_session, outcome) = run_case(name, "FAIL").await;
        let actual = match outcome {
            Ok(merged) => {
                assert_eq!(merged.merged.len(), 1, "{name}: expected one merged entry");
                String::new()
            }
            Err(error::Error::Diag {
                code: "merge-spec-conflicts",
                detail,
            }) => format!("{detail}\n"),
            Err(other) => panic!("{name}: unexpected merge error: {other:?}"),
        };
        assert_golden(&case.join("expected-validation.txt"), &actual);
    }
}
