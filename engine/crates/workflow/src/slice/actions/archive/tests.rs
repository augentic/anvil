use std::path::Path;

use jiff::Timestamp;
use specify_error::Error;

use super::archive;

fn ts(raw: &str) -> Timestamp {
    raw.parse().expect("valid timestamp")
}

#[test]
fn moves_dir_to_dated_target() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let slice_dir = tmp.path().join("slices").join("alpha");
    std::fs::create_dir_all(&slice_dir).expect("mkdir slice");
    std::fs::write(slice_dir.join("marker.txt"), b"keep me").expect("write marker");
    let archive_dir = tmp.path().join("archive");

    let target = archive(&slice_dir, &archive_dir, ts("2026-06-01T00:00:00Z")).expect("archive");

    assert_eq!(target, archive_dir.join("2026-06-01-alpha"), "dated, named target path");
    assert!(!slice_dir.exists(), "slice dir leaves slices/");
    assert!(target.is_dir(), "slice lands under archive/");
    assert_eq!(
        std::fs::read_to_string(target.join("marker.txt")).expect("read marker"),
        "keep me",
        "archive move preserves slice contents",
    );
}

#[test]
fn no_basename_errors() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let archive_dir = tmp.path().join("archive");

    let err = archive(Path::new("/"), &archive_dir, ts("2026-06-01T00:00:00Z"))
        .expect_err("root has no basename");
    assert!(
        matches!(
            err,
            Error::Diag {
                code: "slice-dir-no-basename",
                ..
            }
        ),
        "got {err:?}"
    );
    assert!(!archive_dir.exists(), "error path creates no archive dir");
}
