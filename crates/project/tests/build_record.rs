//! Fact-substrate build records (RFC-86 D27).

use project::build_record::BuildRecord;
use project::config::Layout;
use project::seam::wire::{BuildReport, BuildStatus};
use project::snapshot::{CodePatch, SnapshotId};

fn cid(hex64: char) -> SnapshotId {
    SnapshotId::from_digest(&hex64.to_string().repeat(64))
}

fn sample_report(slice: &str) -> BuildReport {
    BuildReport {
        version: 1,
        slice: slice.into(),
        target: "mock@0.0.0".into(),
        status: BuildStatus::Success,
        findings: vec![],
        outputs: vec![],
        ui_surface: None,
    }
}

#[test]
fn write_and_load_latest_round_trip() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = Layout::new(tmp.path());
    let slice_dir = layout.slice_dir("login-flow");
    std::fs::create_dir_all(&slice_dir).expect("slice dir");

    let record = BuildRecord::from_capture(
        CodePatch {
            base: cid('a'),
            result: cid('b'),
            touched: vec!["src/main.rs".into()],
        },
        cid('c'),
        sample_report("login-flow"),
    );
    let written = record.write(&slice_dir).expect("write");
    assert!(written.path.is_file());
    assert_eq!(written.digest, record.digest().expect("digest"));
    assert!(BuildRecord::present(&slice_dir));

    let loaded = BuildRecord::load_latest(&slice_dir).expect("load");
    assert_eq!(loaded, record);
    assert_eq!(loaded.to_patch().touched, ["src/main.rs"]);
    assert_eq!(layout.slice_build_record_path("login-flow", written.digest.digest()), written.path);
}

#[test]
fn missing_record_is_typed() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let slice_dir = Layout::new(tmp.path()).slice_dir("empty");
    std::fs::create_dir_all(&slice_dir).expect("slice dir");
    assert!(!BuildRecord::present(&slice_dir));
    let err = BuildRecord::load_latest(&slice_dir).expect_err("missing");
    assert!(err.to_string().contains("slice-build-record-missing"), "{err}");
}
