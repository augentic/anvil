//! Integration tests for `specify slice merge preview` and
//! `specify slice merge conflict-check`.
//!
//! These are the two no-write counterparts to `specify slice merge run`
//! used by the merge-skill rewrite: `preview` computes the operation
//! list without touching disk; `conflict-check` flags `type: modified`
//! baselines that have drifted since `defined_at`.

use std::fs::{self, File, FileTimes};
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::common::{Project, copy_dir, parse_json, repo_root, specify_cmd};

/// Stamp `path` with a fixed mtime comfortably after the 2020
/// `defined_at` the drift tests seed, so `slice merge conflict-check`'s
/// `mtime > defined_at` comparison fires deterministically — regardless
/// of filesystem mtime granularity or host clock. Replaces the former
/// `sleep`-then-rewrite, which leaned on the live clock advancing past a
/// coarse fs mtime resolution (flaky on fast machines / coarse FSes).
fn stamp_mtime_after_defined_at(path: &Path) {
    // 2023-11-14T22:13:20Z — strictly after the seeded `defined_at`
    // of 2020-01-01 and before the far-future 2099 used by the
    // "older" no-drift case.
    let when = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    File::options()
        .write(true)
        .open(path)
        .expect("open baseline to set mtime")
        .set_times(FileTimes::new().set_modified(when))
        .expect("set explicit baseline mtime");
}

/// Stage the two-spec fixture content into a fresh slice and drive it to
/// `refined` through the real CLI verbs (`slice create` →
/// `slice transition`), instead of staging the `built` fixture and
/// rewriting `metadata.yaml` by hand (testing.md:45). The merge surface
/// reads the slice's `specs/` tree, so only the fixture's spec content is
/// copied in; its `built` `metadata.yaml` is left behind.
fn stage_refined_slice(project: &Project) {
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "my-slice"])
        .assert()
        .success();
    let slice_dir = project.slices_dir().join("my-slice");
    copy_dir(
        &repo_root().join("tests/fixtures/e2e/merge-two-spec-slice/specs"),
        &slice_dir.join("specs"),
    );
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "transition", "my-slice", "refined"])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// slice merge preview
// ---------------------------------------------------------------------------

#[test]
fn preview_reports_operations() {
    let project = Project::init().with_schemas();
    let slice_dir = project.stage_slice("merge-two-spec-slice");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "merge", "preview", "my-slice"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);

    let specs = value["specs"].as_array().expect("specs array");
    // Two-spec fixture: each spec uses `## ADDED Requirements` with one
    // REQ-001 block, producing exactly one `added` op per spec. The
    // `created-baseline` op only fires for verbatim copies without
    // delta headers (see merge-two-spec.json golden).
    let names: Vec<&str> = specs.iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["login", "oauth"]);
    for spec in specs {
        let ops = spec["operations"].as_array().unwrap();
        assert_eq!(ops.len(), 1, "expected one op per spec, got {ops:?}");
        let kind = ops[0]["kind"].as_str().unwrap();
        assert!(
            ["added", "modified", "removed", "renamed", "created-baseline"].contains(&kind),
            "merge-op `kind` must be kebab-case v3 contract, got {kind:?}"
        );
        assert_eq!(kind, "added");
        assert_eq!(ops[0]["id"], "REQ-001");
        assert!(spec["baseline-path"].is_string());
    }

    // No filesystem mutation: no archive, slice dir still in place,
    // baselines under .specify/specs/ untouched.
    assert!(slice_dir.is_dir(), "preview must not archive the slice");
    let archive = project.root().join(".specify/archive");
    assert!(
        !archive.exists() || fs::read_dir(&archive).unwrap().next().is_none(),
        "preview must not create archive entries",
    );
    assert!(
        !project.root().join(".specify/specs/login/spec.md").exists(),
        "preview must not write baselines",
    );
    assert!(
        !project.root().join(".specify/specs/oauth/spec.md").exists(),
        "preview must not write baselines",
    );
}

#[test]
fn preview_doesnt_require_built_status() {
    let project = Project::init().with_schemas();
    // `slice merge run` refuses a non-`built` slice but `slice merge
    // preview` must accept one. Reach `refined` through the real verbs
    // rather than rewriting `metadata.yaml` by hand.
    stage_refined_slice(&project);

    specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "merge", "preview", "my-slice"])
        .assert()
        .success();
}

#[test]
fn preview_emits_readable_text() {
    let project = Project::init().with_schemas();
    project.stage_slice("merge-two-spec-slice");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["slice", "merge", "preview", "my-slice"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    assert!(stdout.contains("login:"));
    assert!(stdout.contains("oauth:"));
    assert!(
        stdout.contains("ADDING: REQ-001"),
        "expected ADDING line in text output, got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// slice merge conflict-check
// ---------------------------------------------------------------------------

#[test]
fn conflict_check_no_conflicts_unmodified() {
    let project = Project::init().with_schemas();
    project.stage_slice("merge-two-spec-slice");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    let conflicts = value["conflicts"].as_array().unwrap();
    assert!(conflicts.is_empty(), "fixture has only `new` entries, got {conflicts:?}");
}

#[test]
fn conflict_check_flags_modified_newer() {
    let project = Project::init().with_schemas();
    let slice_dir = project.stage_slice("merge-two-spec-slice");

    // Seed a baseline file under .specify/specs/login/spec.md then rewrite
    // the slice's metadata to mark `login` as `modified` with a historic
    // defined_at. touching the baseline afterwards puts its mtime in the
    // future relative to defined_at, producing a conflict.
    let baseline = project.root().join(".specify/specs/login/spec.md");
    fs::create_dir_all(baseline.parent().unwrap()).unwrap();
    fs::write(&baseline, "# Login baseline\n").unwrap();

    let metadata_path = slice_dir.join("metadata.yaml");
    fs::write(
        &metadata_path,
        "target: omnia\nstatus: built\ndefined-at: \"2020-01-01T00:00:00Z\"\ntouched-specs:\n  - name: login\n    type: modified\n",
    )
    .unwrap();

    // Set an explicit baseline mtime after `defined_at` so the drift
    // check fires deterministically, insensitive to clock skew or
    // filesystem mtime resolution.
    stamp_mtime_after_defined_at(&baseline);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    let conflicts = value["conflicts"].as_array().unwrap();
    assert_eq!(conflicts.len(), 1, "expected one conflict, got {conflicts:?}");
    assert_eq!(conflicts[0]["adapter"], "login");
    assert_eq!(conflicts[0]["defined-at"], "2020-01-01T00:00:00Z");
    assert!(conflicts[0]["baseline-modified-at"].is_string());
}

#[test]
fn conflict_check_no_drift_when_older() {
    let project = Project::init().with_schemas();
    let slice_dir = project.stage_slice("merge-two-spec-slice");

    // Set defined_at to the far future so nothing is "newer".
    let metadata_path = slice_dir.join("metadata.yaml");
    fs::write(
        &metadata_path,
        "target: omnia\nstatus: built\ndefined-at: \"2099-01-01T00:00:00Z\"\ntouched-specs:\n  - name: login\n    type: new\n",
    )
    .unwrap();

    // Seed a baseline contract file (its mtime will be well before 2099).
    let baseline_contract = project.root().join("contracts/schemas/test.yaml");
    fs::create_dir_all(baseline_contract.parent().unwrap()).unwrap();
    fs::write(&baseline_contract, "type: object\n").unwrap();

    // Seed the corresponding slice contract so the drift walker visits it.
    let slice_contract = slice_dir.join("contracts/schemas/test.yaml");
    fs::create_dir_all(slice_contract.parent().unwrap()).unwrap();
    fs::write(&slice_contract, "type: object\nproperties: {}\n").unwrap();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    let conflicts = value["conflicts"].as_array().unwrap();
    assert!(
        conflicts.is_empty(),
        "baseline is older than defined_at, expected no conflicts, got {conflicts:?}"
    );
}

#[test]
fn conflict_check_detects_drift_when_newer() {
    let project = Project::init().with_schemas();
    let slice_dir = project.stage_slice("merge-two-spec-slice");

    // defined_at in the deep past — any real file mtime will be newer.
    let metadata_path = slice_dir.join("metadata.yaml");
    fs::write(
        &metadata_path,
        "target: omnia\nstatus: built\ndefined-at: \"2020-01-01T00:00:00Z\"\ntouched-specs:\n  - name: login\n    type: new\n",
    )
    .unwrap();

    let baseline_contract = project.root().join("contracts/schemas/test.yaml");
    fs::create_dir_all(baseline_contract.parent().unwrap()).unwrap();
    fs::write(&baseline_contract, "type: object\n").unwrap();

    // Set an explicit baseline mtime after `defined_at` so the opaque
    // drift walker reports a conflict deterministically.
    stamp_mtime_after_defined_at(&baseline_contract);

    let slice_contract = slice_dir.join("contracts/schemas/test.yaml");
    fs::create_dir_all(slice_contract.parent().unwrap()).unwrap();
    fs::write(&slice_contract, "type: object\nproperties: {}\n").unwrap();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    let conflicts = value["conflicts"].as_array().unwrap();
    assert_eq!(conflicts.len(), 1, "expected one contract conflict, got {conflicts:?}");
    assert_eq!(conflicts[0]["adapter"], "contracts/schemas/test.yaml");
    assert_eq!(conflicts[0]["defined-at"], "2020-01-01T00:00:00Z");
}

#[test]
fn conflict_check_no_drift_for_new_files() {
    let project = Project::init().with_schemas();
    let slice_dir = project.stage_slice("merge-two-spec-slice");

    let metadata_path = slice_dir.join("metadata.yaml");
    fs::write(
        &metadata_path,
        "target: omnia\nstatus: built\ndefined-at: \"2020-01-01T00:00:00Z\"\ntouched-specs:\n  - name: login\n    type: new\n",
    )
    .unwrap();

    // Slice has a contract file, but no corresponding baseline exists.
    let slice_contract = slice_dir.join("contracts/schemas/new.yaml");
    fs::create_dir_all(slice_contract.parent().unwrap()).unwrap();
    fs::write(&slice_contract, "type: object\n").unwrap();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    let conflicts = value["conflicts"].as_array().unwrap();
    assert!(
        conflicts.is_empty(),
        "new contract files (not in baseline) should not produce conflicts, got {conflicts:?}"
    );
}

#[test]
fn conflict_check_no_drift_no_contracts() {
    let project = Project::init().with_schemas();
    let slice_dir = project.stage_slice("merge-two-spec-slice");

    let metadata_path = slice_dir.join("metadata.yaml");
    fs::write(
        &metadata_path,
        "target: omnia\nstatus: built\ndefined-at: \"2020-01-01T00:00:00Z\"\ntouched-specs:\n  - name: login\n    type: new\n",
    )
    .unwrap();

    // Seed a baseline contract but do NOT create contracts/ in the slice.
    let baseline_contract = project.root().join("contracts/schemas/test.yaml");
    fs::create_dir_all(baseline_contract.parent().unwrap()).unwrap();
    fs::write(&baseline_contract, "type: object\n").unwrap();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    let conflicts = value["conflicts"].as_array().unwrap();
    assert!(
        conflicts.is_empty(),
        "no contracts/ in the slice means no contract drift, got {conflicts:?}"
    );
}

// ---------------------------------------------------------------------------
// slice merge run — outcome-ledger event + archive prune
// ---------------------------------------------------------------------------

#[test]
fn run_archives_and_emits_ledger_event() {
    let project = Project::init().with_schemas();
    project.stage_slice("merge-two-spec-slice");

    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "merge", "run", "my-slice"])
        .assert()
        .success();

    // Slice folder archived under .specify/archive/YYYY-MM-DD-my-slice.
    let archive = project.root().join(".specify/archive");
    let entries: Vec<_> = fs::read_dir(&archive)
        .expect("archive dir exists after merge")
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(entries.len(), 1, "expected one archived slice, got {entries:?}");
    assert!(entries[0].ends_with("-my-slice"), "archive name carries slice, got {entries:?}");

    // Outcome ledger: journal carries one slice.archive.created line.
    let journal = fs::read_to_string(project.root().join(".specify/journal.jsonl"))
        .expect("journal.jsonl written");
    let ledger: Vec<&str> =
        journal.lines().filter(|l| l.contains(r#""event":"slice.archive.created""#)).collect();
    assert_eq!(ledger.len(), 1, "expected one ledger event, got:\n{journal}");
    let line = ledger[0];
    assert!(line.contains(r#""slice-name":"my-slice""#), "ledger names the slice: {line}");
    assert!(line.contains(r#""touched-specs":"#), "ledger lists touched specs: {line}");
    assert!(line.contains(r#""outcome-summary":"#), "ledger carries a summary: {line}");
}

#[test]
fn run_emits_merge_started_then_succeeded() {
    // A successful `slice merge run` brackets the validator
    // outcome with `slice.merge.started` then `slice.merge.succeeded`,
    // with the durable `slice.archive.created` ledger entry in between.
    let project = Project::init().with_schemas();
    project.stage_slice("merge-two-spec-slice");

    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "merge", "run", "my-slice"])
        .assert()
        .success();

    let journal = fs::read_to_string(project.root().join(".specify/journal.jsonl"))
        .expect("journal.jsonl written");
    let merge_events: Vec<&str> =
        journal.lines().filter(|l| l.contains(r#""event":"slice.merge."#)).collect();
    assert_eq!(
        merge_events.len(),
        2,
        "expected slice.merge.started + slice.merge.succeeded, got:\n{journal}"
    );
    assert!(
        merge_events[0].contains(r#""event":"slice.merge.started""#),
        "first merge event must be slice.merge.started, got: {}",
        merge_events[0]
    );
    assert!(
        merge_events[0].contains(r#""slice-name":"my-slice""#),
        "started names the slice: {}",
        merge_events[0]
    );
    assert!(
        merge_events[1].contains(r#""event":"slice.merge.succeeded""#),
        "second merge event must be slice.merge.succeeded, got: {}",
        merge_events[1]
    );
    assert!(
        merge_events[1].contains(r#""slice-name":"my-slice""#),
        "succeeded names the slice: {}",
        merge_events[1]
    );

    // The ledger entry still lands and sits between started and
    // succeeded.
    let ordered_ids: Vec<&str> = journal
        .lines()
        .filter(|l| {
            l.contains(r#""event":"slice.merge."#)
                || l.contains(r#""event":"slice.archive.created""#)
        })
        .collect();
    assert_eq!(
        ordered_ids.len(),
        3,
        "expected started, archive.created, succeeded, got:\n{journal}"
    );
    assert!(ordered_ids[0].contains("slice.merge.started"));
    assert!(ordered_ids[1].contains("slice.archive.created"));
    assert!(ordered_ids[2].contains("slice.merge.succeeded"));
}

#[test]
fn emits_merge_started_then_failed() {
    // A forced validator/commit failure brackets the run with
    // `slice.merge.started` then `slice.merge.failed` (non-empty
    // `reason`), exits non-zero, and emits neither `slice.merge.succeeded`
    // nor the `slice.archive.created` ledger entry. A slice in `refined`
    // makes `slice::commit` reject the non-`Built` status with the
    // `lifecycle` diagnostic; reach that state through the real verbs
    // rather than rewriting `metadata.yaml` by hand.
    let project = Project::init().with_schemas();
    stage_refined_slice(&project);

    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "merge", "run", "my-slice"])
        .assert()
        .failure();

    let journal = fs::read_to_string(project.root().join(".specify/journal.jsonl"))
        .expect("journal.jsonl written");
    let merge_events: Vec<&str> =
        journal.lines().filter(|l| l.contains(r#""event":"slice.merge."#)).collect();
    assert_eq!(
        merge_events.len(),
        2,
        "expected slice.merge.started + slice.merge.failed, got:\n{journal}"
    );
    assert!(
        merge_events[0].contains(r#""event":"slice.merge.started""#),
        "first merge event must be slice.merge.started, got: {}",
        merge_events[0]
    );
    let failed = merge_events[1];
    assert!(
        failed.contains(r#""event":"slice.merge.failed""#),
        "second merge event must be slice.merge.failed, got: {failed}"
    );
    assert!(failed.contains(r#""slice-name":"my-slice""#), "failed names the slice: {failed}");
    let value: serde_json::Value =
        serde_json::from_str(failed).expect("slice.merge.failed line is JSON");
    let reason = value["payload"]["reason"].as_str().expect("reason field present");
    assert!(!reason.is_empty(), "failed event must carry a non-empty reason, got: {failed}");

    assert!(
        !journal.contains(r#""event":"slice.merge.succeeded""#),
        "a failed merge must not emit slice.merge.succeeded:\n{journal}"
    );
    assert!(
        !journal.contains(r#""event":"slice.archive.created""#),
        "a failed merge must not emit the slice.archive.created ledger entry:\n{journal}"
    );
}

#[test]
fn archive_prune_keeps_recent_by_count() {
    let project = Project::init();
    let archive = project.root().join(".specify/archive");
    fs::create_dir_all(&archive).unwrap();
    for name in ["2026-01-01-alpha", "2026-03-01-beta", "2026-05-01-gamma"] {
        fs::create_dir_all(archive.join(name)).unwrap();
    }

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "archive", "prune", "--keep", "2"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    let pruned = value["pruned"].as_array().unwrap();
    assert_eq!(pruned.len(), 1, "keep 2 of 3 prunes the oldest, got {pruned:?}");
    assert_eq!(pruned[0], "2026-01-01-alpha");

    assert!(!archive.join("2026-01-01-alpha").exists(), "oldest must be removed");
    assert!(archive.join("2026-05-01-gamma").exists(), "newest must remain");
}

#[test]
fn archive_prune_dry_run_removes_nothing() {
    let project = Project::init();
    let archive = project.root().join(".specify/archive");
    fs::create_dir_all(archive.join("2026-01-01-alpha")).unwrap();

    specify_cmd()
        .current_dir(project.root())
        .args(["archive", "prune", "--keep", "0", "--dry-run"])
        .assert()
        .success();

    assert!(archive.join("2026-01-01-alpha").exists(), "dry-run must not remove folders");
}

#[test]
fn archive_prune_requires_a_bound() {
    let project = Project::init();
    fs::create_dir_all(project.root().join(".specify/archive")).unwrap();

    specify_cmd().current_dir(project.root()).args(["archive", "prune"]).assert().failure();
}

#[test]
fn conflict_check_ignores_new_entries() {
    // `type: new` baselines are "we're creating this adapter" — even
    // if a file already exists at the baseline path, it is not a drift
    // conflict in the mtime-vs-defined_at sense, just a different kind
    // of integrity issue the caller should handle separately.
    let project = Project::init().with_schemas();
    project.stage_slice("merge-two-spec-slice");
    let baseline = project.root().join(".specify/specs/login/spec.md");
    fs::create_dir_all(baseline.parent().unwrap()).unwrap();
    fs::write(&baseline, "# Login baseline\n").unwrap();

    // touched_specs keeps the fixture's `new` classification; no
    // `defined_at` means conflict_check returns empty regardless.
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    assert!(value["conflicts"].as_array().unwrap().is_empty());
}
