//! `specify archive prune` retention GC — the `--older-than` (max-age)
//! bound exercised through the binary. The pure scan/prune kernel has
//! unit coverage in `crates/workflow/src/slice/actions/prune/tests.rs`;
//! these pin the CLI wiring: flag parsing, the at-least-one-bound
//! requirement, the JSON envelope, and the on-disk removal.
//!
//! Age is derived from the `YYYY-MM-DD` prefix of each archived folder
//! name, so the fixtures seed folders dated relative to `now`. The CLI
//! reads its own `Timestamp::now()` a beat after the test reads its
//! own; the 100-days-vs-30 and 0-days-vs-30 margins absorb any midnight
//! roll between the two clock reads.

use std::fs;

use jiff::{SignedDuration, Timestamp};

use crate::common::{Project, parse_stdout, specify_cmd};

/// Create `.specify/archive/<date>-<slice>/spec.md` where `<date>` is
/// `days_ago` days before now, and return the folder basename.
fn seed_archive(project: &Project, days_ago: i64, slice: &str) -> String {
    let date = Timestamp::now()
        .checked_sub(SignedDuration::from_hours(24 * days_ago))
        .expect("now - days")
        .strftime("%Y-%m-%d")
        .to_string();
    let name = format!("{date}-{slice}");
    let dir = project.root().join(".specify/archive").join(&name);
    fs::create_dir_all(&dir).expect("mkdir archive folder");
    fs::write(dir.join("spec.md"), "# archived\n").expect("seed archived file");
    name
}

/// Folder basenames listed in the prune envelope's `pruned` array.
fn pruned_names(value: &serde_json::Value) -> Vec<&str> {
    value["pruned"]
        .as_array()
        .expect("pruned array")
        .iter()
        .map(|v| v.as_str().expect("pruned name"))
        .collect()
}

#[test]
fn prune_removes_aged_keeps_fresh() {
    let project = Project::init();
    let ancient = seed_archive(&project, 100, "ancient-slice");
    let fresh = seed_archive(&project, 0, "fresh-slice");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "archive", "prune", "--older-than", "30"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());

    assert_eq!(actual["dry-run"], false);
    assert_eq!(
        pruned_names(&actual),
        vec![ancient.as_str()],
        "only the >30-day folder is pruned, got: {actual}"
    );

    let archive = project.root().join(".specify/archive");
    assert!(!archive.join(&ancient).exists(), "the aged folder must be removed from disk");
    assert!(archive.join(&fresh).is_dir(), "the fresh folder must survive the prune");
}

#[test]
fn prune_dry_run_reports_without_removing() {
    let project = Project::init();
    let ancient = seed_archive(&project, 100, "ancient-slice");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "archive", "prune", "--older-than", "30", "--dry-run"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());

    assert_eq!(actual["dry-run"], true);
    assert_eq!(
        pruned_names(&actual),
        vec![ancient.as_str()],
        "dry-run still reports the prune candidate, got: {actual}"
    );
    assert!(
        project.root().join(".specify/archive").join(&ancient).is_dir(),
        "dry-run must not remove anything"
    );
}

#[test]
fn prune_requires_a_retention_bound() {
    // Neither `--keep` nor `--older-than` supplied: the handler rejects
    // the invocation with an argument error (exit 2) rather than
    // silently pruning nothing.
    let project = Project::init();
    let assert =
        specify_cmd().current_dir(project.root()).args(["archive", "prune"]).assert().failure();
    assert_eq!(
        assert.get_output().status.code(),
        Some(2),
        "a prune with no retention bound is an argument error"
    );
}
