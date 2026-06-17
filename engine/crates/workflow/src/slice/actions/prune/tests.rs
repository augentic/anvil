use super::*;

fn now() -> Timestamp {
    "2026-06-01T00:00:00Z".parse().expect("valid timestamp")
}

fn stage(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::create_dir_all(&p).expect("mkdir");
    p
}

#[test]
fn keep_count_prunes_oldest() {
    let tmp = tempfile::tempdir().expect("tempdir");
    stage(tmp.path(), "2026-05-01-alpha");
    stage(tmp.path(), "2026-05-20-beta");
    stage(tmp.path(), "2026-05-30-gamma");
    let retention = Retention {
        keep: Some(2),
        max_age_days: None,
    };
    let prune_set = scan(tmp.path(), retention, now()).expect("scan");
    let names: Vec<&str> = prune_set.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["2026-05-01-alpha"]);
}

#[test]
fn max_age_prunes_old() {
    let tmp = tempfile::tempdir().expect("tempdir");
    stage(tmp.path(), "2026-01-01-ancient");
    stage(tmp.path(), "2026-05-30-fresh");
    let retention = Retention {
        keep: None,
        max_age_days: Some(30),
    };
    let prune_set = scan(tmp.path(), retention, now()).expect("scan");
    let names: Vec<&str> = prune_set.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["2026-01-01-ancient"]);
}

#[test]
fn prune_removes_dirs() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let old = stage(tmp.path(), "2026-01-01-ancient");
    stage(tmp.path(), "2026-05-30-fresh");
    let retention = Retention {
        keep: Some(1),
        max_age_days: None,
    };
    let prune_set = scan(tmp.path(), retention, now()).expect("scan");
    prune(&prune_set).expect("prune");
    assert!(!old.exists(), "pruned folder must be gone");
    assert!(tmp.path().join("2026-05-30-fresh").exists(), "kept folder must remain");
}

#[test]
fn missing_archive_is_noop() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("does-not-exist");
    let retention = Retention {
        keep: Some(1),
        max_age_days: None,
    };
    let prune_set = scan(&missing, retention, now()).expect("scan");
    assert!(prune_set.is_empty());
}

#[test]
fn bad_entry_name_errors() {
    let tmp = tempfile::tempdir().expect("tempdir");
    stage(tmp.path(), "not-a-date");
    let retention = Retention {
        keep: Some(1),
        max_age_days: None,
    };
    let err = scan(tmp.path(), retention, now()).expect_err("must reject bad name");
    assert!(matches!(err, Error::Validation { .. }));
}
