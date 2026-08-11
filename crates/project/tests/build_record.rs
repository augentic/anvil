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
        covered: vec![],
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
        vec![],
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
fn from_capture_dedups_deferred_digests() {
    // Identical requirement bodies legally share one digest (RFC-86a
    // D2) — the record binds the unique, sorted set.
    let first = format!("sha256:{}", "a".repeat(64));
    let second = format!("sha256:{}", "b".repeat(64));
    let record = BuildRecord::from_capture(
        CodePatch {
            base: cid('a'),
            result: cid('b'),
            touched: vec![],
        },
        cid('c'),
        sample_report("login-flow"),
        vec![second.clone(), first.clone(), second.clone()],
    );
    assert_eq!(record.deferred, [first, second]);
}

#[test]
fn load_for_wave_selects_by_wave_not_mtime() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let slice_dir = Layout::new(tmp.path()).slice_dir("login-flow");
    std::fs::create_dir_all(&slice_dir).expect("slice dir");

    let authorized = BuildRecord::from_capture(
        CodePatch {
            base: cid('a'),
            result: cid('b'),
            touched: vec![],
        },
        cid('c'),
        sample_report("login-flow"),
        vec![],
    );
    authorized.write(&slice_dir).expect("write authorized");

    // A decoy under a different wave, written last so it carries the
    // newest mtime.
    let decoy = BuildRecord::from_capture(
        CodePatch {
            base: cid('a'),
            result: cid('d'),
            touched: vec![],
        },
        cid('e'),
        sample_report("login-flow"),
        vec![],
    );
    let written = decoy.write(&slice_dir).expect("write decoy");
    let future = std::time::SystemTime::now() + std::time::Duration::from_hours(1);
    std::fs::File::options()
        .write(true)
        .open(&written.path)
        .expect("open decoy")
        .set_modified(future)
        .expect("set decoy mtime");

    let loaded = BuildRecord::load_for_wave(&slice_dir, &cid('c')).expect("load by wave");
    assert_eq!(loaded, authorized);

    let err = BuildRecord::load_for_wave(&slice_dir, &cid('f')).expect_err("unknown wave");
    assert!(err.to_string().contains("slice-build-record-missing"), "{err}");
}

#[test]
fn duplicate_records_for_one_wave_are_ambiguous() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let slice_dir = Layout::new(tmp.path()).slice_dir("login-flow");
    std::fs::create_dir_all(&slice_dir).expect("slice dir");

    // Two distinct records naming the same wave — selection must
    // refuse rather than silently pick one.
    for result in ['b', 'd'] {
        BuildRecord::from_capture(
            CodePatch {
                base: cid('a'),
                result: cid(result),
                touched: vec![],
            },
            cid('c'),
            sample_report("login-flow"),
            vec![],
        )
        .write(&slice_dir)
        .expect("write record");
    }

    let err = BuildRecord::load_for_wave(&slice_dir, &cid('c')).expect_err("duplicates refuse");
    assert!(err.to_string().contains("slice-build-record-ambiguous"), "{err}");
    let err = BuildRecord::find_for_wave(&slice_dir, &cid('c')).expect_err("probe refuses too");
    assert!(err.to_string().contains("slice-build-record-ambiguous"), "{err}");

    // The probe form reads a missing wave as a state, not a refusal.
    assert_eq!(BuildRecord::find_for_wave(&slice_dir, &cid('f')).expect("probe"), None);
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
