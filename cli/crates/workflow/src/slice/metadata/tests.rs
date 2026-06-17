use tempfile::tempdir;

use super::*;
use crate::journal::test_timestamp;
use crate::slice::LifecycleStatus;

fn sample() -> SliceMetadata {
    SliceMetadata {
        target: "omnia".to_string(),
        status: LifecycleStatus::Refined,
        created_at: Some(test_timestamp("2024-08-01T10:00:00Z")),
        defined_at: Some(test_timestamp("2024-08-01T12:00:00Z")),
        completed_at: Some(test_timestamp("2024-08-03T15:45:00Z")),
        merged_at: None,
        dropped_at: None,
        drop_reason: None,
        touched_specs: vec![
            TouchedSpec {
                name: "login".to_string(),
                kind: SpecKind::Modified,
            },
            TouchedSpec {
                name: "oauth".to_string(),
                kind: SpecKind::New,
            },
        ],
        outcome: None,
    }
}

#[test]
fn save_load_round_trips() {
    let dir = tempdir().expect("tempdir");
    let meta = sample();
    meta.save(dir.path()).expect("save ok");
    let loaded = SliceMetadata::load(dir.path()).expect("load ok");
    assert_eq!(loaded, meta);
}

#[test]
fn round_trips_with_outcome() {
    // The `outcome` block carries the `#[serde(rename = "outcome")]`
    // `kind` field and an `Option<context>`; round-trip it so a rename
    // or skip-rule regression is caught at the unit level.
    let dir = tempdir().expect("tempdir");
    let mut meta = sample();
    meta.outcome = Some(Outcome {
        phase: TargetOperation::Build,
        kind: OutcomeKind::Failure,
        at: test_timestamp("2026-01-02T03:04:05Z"),
        summary: "cargo check failed".to_string(),
        context: Some("error[E0382]".to_string()),
    });
    meta.save(dir.path()).expect("save ok");
    assert_eq!(SliceMetadata::load(dir.path()).expect("load ok"), meta);
}

#[test]
fn load_missing_errors() {
    // The "not a slice directory" signal `slice list` / `/spec:execute`
    // rely on: an absent file is `ArtifactNotFound`, never a panic.
    let dir = tempdir().expect("tempdir");
    match SliceMetadata::load(dir.path()) {
        Err(Error::ArtifactNotFound { kind, .. }) => assert_eq!(kind, "metadata.yaml"),
        other => panic!("absent metadata must be ArtifactNotFound, got {other:?}"),
    }
}

#[test]
fn load_malformed_errors() {
    // A closed-enum violation (here an unknown `status`) must surface as
    // a deserialisation error, not a silent default.
    let dir = tempdir().expect("tempdir");
    std::fs::write(SliceMetadata::path(dir.path()), "target: omnia\nstatus: not-a-state\n")
        .expect("write malformed metadata");
    let err = SliceMetadata::load(dir.path()).expect_err("malformed metadata must error");
    assert!(matches!(err, Error::YamlDe(_)), "expected YamlDe, got {err:?}");
}

#[test]
fn omits_none_fields_on_disk() {
    // Optional timestamps and `outcome` use `skip_serializing_if`; a
    // minimal slice must not emit empty `merged-at:` / `outcome:` keys.
    let dir = tempdir().expect("tempdir");
    let meta = SliceMetadata {
        target: "omnia".to_string(),
        status: LifecycleStatus::Refining,
        created_at: None,
        defined_at: None,
        completed_at: None,
        merged_at: None,
        dropped_at: None,
        drop_reason: None,
        touched_specs: vec![],
        outcome: None,
    };
    meta.save(dir.path()).expect("save ok");
    let raw = std::fs::read_to_string(SliceMetadata::path(dir.path())).expect("read back");
    assert!(!raw.contains("merged-at"), "absent timestamp must be omitted, got:\n{raw}");
    assert!(!raw.contains("outcome"), "absent outcome must be omitted, got:\n{raw}");
    assert_eq!(SliceMetadata::load(dir.path()).expect("load ok"), meta);
}
