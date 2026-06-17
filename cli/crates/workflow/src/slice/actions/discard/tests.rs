use tempfile::tempdir;

use super::*;
use crate::journal::test_timestamp;

/// Seed a `<slice_dir>/metadata.yaml` in the given lifecycle `status`.
fn seed(slice_dir: &Path, status: LifecycleStatus) {
    let meta = SliceMetadata {
        target: "omnia".to_string(),
        status,
        created_at: None,
        defined_at: None,
        completed_at: None,
        merged_at: None,
        dropped_at: None,
        drop_reason: None,
        touched_specs: vec![],
        outcome: None,
    };
    meta.save(slice_dir).expect("seed metadata");
}

#[test]
fn discard_from_terminal_errors() {
    // `discard` is legal from any *non-terminal* state; from `Merged` or
    // `Dropped` the lifecycle machine refuses the `* -> Dropped` edge.
    // The refusal must be a `lifecycle` diag and must leave the slice
    // unstamped and unarchived (the transition fails before either side
    // effect).
    for status in [LifecycleStatus::Merged, LifecycleStatus::Dropped] {
        let tmp = tempdir().expect("tempdir");
        let slice_dir = tmp.path().join("checkout");
        std::fs::create_dir_all(&slice_dir).expect("mkdir slice");
        seed(&slice_dir, status);
        let archive_dir = tmp.path().join("archive");

        let now = test_timestamp("2026-04-24T12:00:00Z");
        match discard(&slice_dir, &archive_dir, Some("too late"), now) {
            Err(Error::Diag { code, detail }) => {
                assert_eq!(code, "lifecycle", "{status} discard reject code");
                assert!(
                    detail.contains(&format!("{status:?}")),
                    "{status}: detail names the offending state: {detail}"
                );
            }
            other => panic!("{status}: expected a lifecycle diag, got {other:?}"),
        }

        let after = SliceMetadata::load(&slice_dir).expect("metadata still present");
        assert_eq!(after.status, status, "{status}: refused discard must not transition");
        assert!(after.drop_reason.is_none(), "{status}: refused discard must not stamp a reason");
        assert!(!archive_dir.exists(), "{status}: refused discard must not create the archive dir");
    }
}
