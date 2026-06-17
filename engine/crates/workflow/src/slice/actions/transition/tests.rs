use std::path::{Path, PathBuf};

use jiff::Timestamp;
use specify_error::Error;

use super::transition;
use crate::config::Layout;
use crate::journal::{self, EventKind};
use crate::slice::{LifecycleStatus, SliceMetadata};

fn ts(raw: &str) -> Timestamp {
    raw.parse().expect("valid timestamp")
}

/// Seed `<root>/.specify/slices/<name>/metadata.yaml` at `status` and
/// return the slice dir. `defined_at` lets a test pre-stamp the field
/// the `Refined` edge would otherwise fill in.
fn seed(
    root: &Path, name: &str, status: LifecycleStatus, defined_at: Option<Timestamp>,
) -> PathBuf {
    let slice_dir = root.join(".specify").join("slices").join(name);
    std::fs::create_dir_all(&slice_dir).expect("mkdir slice");
    let meta = SliceMetadata {
        target: "omnia@1.0.0".to_string(),
        status,
        created_at: Some(ts("2026-05-01T00:00:00Z")),
        defined_at,
        completed_at: None,
        merged_at: None,
        dropped_at: None,
        drop_reason: None,
        touched_specs: Vec::new(),
        outcome: None,
    };
    meta.save(&slice_dir).expect("seed metadata");
    slice_dir
}

#[test]
fn refined_stamps_and_emits() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let now = ts("2026-06-01T00:00:00Z");
    let slice_dir = seed(tmp.path(), "alpha", LifecycleStatus::Refining, None);

    let meta = transition(&slice_dir, LifecycleStatus::Refined, now).expect("legal edge");
    assert_eq!(meta.status, LifecycleStatus::Refined);
    assert_eq!(meta.defined_at, Some(now));

    let reloaded = SliceMetadata::load(&slice_dir).expect("reload");
    assert_eq!(reloaded.status, LifecycleStatus::Refined, "new state persisted");
    assert_eq!(reloaded.defined_at, Some(now));

    let events = journal::read(Layout::new(tmp.path())).expect("read journal");
    let refined: Vec<&str> = events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::SliceTransitionRefined { slice_name } => Some(slice_name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(refined, vec!["alpha"], "one slice.transition.refined for the slice");
}

#[test]
fn illegal_edge_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let now = ts("2026-06-01T00:00:00Z");
    let slice_dir = seed(tmp.path(), "beta", LifecycleStatus::Refining, None);

    let err = transition(&slice_dir, LifecycleStatus::Built, now).expect_err("illegal edge");
    assert!(
        matches!(
            err,
            Error::Diag {
                code: "lifecycle",
                ..
            }
        ),
        "got {err:?}"
    );

    let reloaded = SliceMetadata::load(&slice_dir).expect("reload");
    assert_eq!(reloaded.status, LifecycleStatus::Refining, "disk untouched on reject");
    let events = journal::read(Layout::new(tmp.path())).expect("read journal");
    assert!(events.is_empty(), "illegal edge must not journal");
}

#[test]
fn existing_stamp_preserved() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let earlier = ts("2026-05-15T00:00:00Z");
    let now = ts("2026-06-01T00:00:00Z");
    let slice_dir = seed(tmp.path(), "gamma", LifecycleStatus::Refining, Some(earlier));

    let meta = transition(&slice_dir, LifecycleStatus::Refined, now).expect("legal edge");
    assert_eq!(meta.defined_at, Some(earlier), "pre-set stamp is idempotent");
}
