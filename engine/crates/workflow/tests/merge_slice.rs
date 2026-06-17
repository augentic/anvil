//! End-to-end filesystem tests for `specify_workflow::merge::slice::commit`.
//!
//! Each test builds a throw-away project under `tempfile::TempDir`, seeds a
//! slice directory with delta specs at `specs/<name>/spec.md`, and drives
//! `slice::commit` through its happy + sad paths. Discovery is
//! convention-based — no schema or `generates` directive is needed.

use std::fs;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use regex::Regex;
use specify_error::Error;
use specify_workflow::adapter::TargetOperation;
use specify_workflow::merge::{ArtifactClass, MergeStrategy, OpaqueAction, slice};
use specify_workflow::slice::{
    LifecycleStatus, Outcome, OutcomeKind, SLICES_DIR_NAME, SliceMetadata,
};

fn parse_stamp(raw: &str) -> Timestamp {
    raw.parse().expect("valid RFC3339 timestamp in test fixture")
}
use tempfile::TempDir;

const SLICE_NAME: &str = "feature-x";

struct Project {
    _tmp: TempDir,
    root: PathBuf,
}

impl Project {
    fn slice_dir(&self) -> PathBuf {
        self.root.join(".specify").join(SLICES_DIR_NAME).join(SLICE_NAME)
    }

    fn specs_dir(&self) -> PathBuf {
        self.root.join(".specify/specs")
    }

    fn contracts_dir(&self) -> PathBuf {
        self.root.join("contracts")
    }

    fn archive_dir(&self) -> PathBuf {
        self.root.join(".specify/archive")
    }
}

/// Build the omnia-shaped artefact-class slice the engine consumes for
/// these tests. Tests are allowed to use literal class names per
/// the metadata summary contract (and `make checks` ignores `#[cfg(test)]` blocks).
fn omnia_classes(slice_dir: &Path, project_root: &Path) -> Vec<ArtifactClass> {
    vec![
        ArtifactClass {
            name: "specs".to_string(),
            staged_dir: slice_dir.join("specs"),
            baseline_dir: project_root.join(".specify/specs"),
            strategy: MergeStrategy::ThreeWayMerge,
        },
        ArtifactClass {
            name: "contracts".to_string(),
            staged_dir: slice_dir.join("contracts"),
            baseline_dir: project_root.join("contracts"),
            strategy: MergeStrategy::OpaqueReplace,
        },
    ]
}

/// Build a fixture project with delta specs at the conventional path.
/// No schema directory or merge brief is needed — discovery scans
/// `<slice_dir>/specs/*/spec.md` directly.
fn build_project() -> Project {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();

    let slices_subdir = format!(".specify/{SLICES_DIR_NAME}");
    for sub in [slices_subdir.as_str(), ".specify/specs", ".specify/archive"] {
        fs::create_dir_all(root.join(sub)).expect("mkdir");
    }

    let slice_dir = root.join(&slices_subdir).join(SLICE_NAME);
    fs::create_dir_all(slice_dir.join("specs/login")).expect("mkdir login");
    fs::create_dir_all(slice_dir.join("specs/oauth")).expect("mkdir oauth");
    fs::write(slice_dir.join("proposal.md"), "# proposal\n").expect("write proposal");
    fs::write(slice_dir.join("specs/login/spec.md"), include_str!("data/delta-login.md"))
        .expect("write login delta");
    fs::write(slice_dir.join("specs/oauth/spec.md"), include_str!("data/delta-oauth.md"))
        .expect("write oauth delta");

    let metadata = SliceMetadata {
        target: "omnia".to_string(),
        status: LifecycleStatus::Built,
        created_at: Some(parse_stamp("2024-08-01T10:00:00Z")),
        defined_at: Some(parse_stamp("2024-08-01T12:00:00Z")),
        completed_at: None,
        merged_at: None,
        dropped_at: None,
        drop_reason: None,
        touched_specs: vec![],
        outcome: None,
    };
    metadata.save(&slice_dir).expect("save metadata");

    Project { _tmp: tmp, root }
}

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn happy_path_writes_flips_and_archives() {
    let project = build_project();
    let slice_dir = project.slice_dir();
    let specs_dir = project.specs_dir();
    let archive_dir = project.archive_dir();
    let classes = omnia_classes(&slice_dir, &project.root);

    let merged = slice::commit(&slice_dir, &classes, &archive_dir, Timestamp::now(), false)
        .expect("slice::commit should succeed");

    // Results sorted by (class_name, name).
    let names: Vec<&str> = merged.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["login", "oauth"]);

    // Baselines now exist.
    let login_baseline = specs_dir.join("login/spec.md");
    let oauth_baseline = specs_dir.join("oauth/spec.md");
    assert!(login_baseline.is_file(), "{} missing", login_baseline.display());
    assert!(oauth_baseline.is_file(), "{} missing", oauth_baseline.display());
    let login_text = fs::read_to_string(&login_baseline).unwrap();
    assert!(login_text.contains("REQ-001"));
    assert!(login_text.contains("### Requirement:"));

    // Slice directory has moved under archive/.
    assert!(!slice_dir.exists(), "{} should be gone", slice_dir.display());
    let archive_re = Regex::new(r"^\d{4}-\d{2}-\d{2}-feature-x$").unwrap();
    let archived: Vec<_> = fs::read_dir(&archive_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .collect();
    assert_eq!(archived.len(), 1);
    let archived_name = archived[0].file_name().to_string_lossy().to_string();
    assert!(
        archive_re.is_match(&archived_name),
        "archive dir `{archived_name}` does not match YYYY-MM-DD-feature-x"
    );

    // metadata.yaml inside archive should be `Merged` with completed_at set.
    let archived_slice_dir = archived[0].path();
    let new_meta = SliceMetadata::load(&archived_slice_dir).expect("load archived metadata");
    assert_eq!(new_meta.status, LifecycleStatus::Merged);
    assert!(new_meta.completed_at.is_some(), "expected completed_at to be set after merge");

    // slice::commit stamps the phase outcome before archiving. Per
    // the metadata summary contract the summary is generic — it lists each
    // contributing class name and entry count.
    let outcome = new_meta.outcome.expect("expected outcome to be stamped by slice::commit");
    assert_eq!(outcome.phase, TargetOperation::Merge);
    assert_eq!(outcome.kind, OutcomeKind::Success);
    assert!(outcome.summary.contains("2 specs"), "unexpected summary: {}", outcome.summary);
}

// ---------------------------------------------------------------------------
// Wrong precondition
// ---------------------------------------------------------------------------

#[test]
fn wrong_precondition_aborts_cleanly() {
    let project = build_project();
    let slice_dir = project.slice_dir();

    // Re-save metadata with status = Refined (merge requires Built).
    let mut meta = SliceMetadata::load(&slice_dir).unwrap();
    meta.status = LifecycleStatus::Refined;
    meta.save(&slice_dir).unwrap();

    let classes = omnia_classes(&slice_dir, &project.root);
    let err = slice::commit(&slice_dir, &classes, &project.archive_dir(), Timestamp::now(), false)
        .expect_err("should refuse on Refined status");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "lifecycle");
            assert!(detail.contains("Built"), "unexpected detail: {detail}");
            assert!(detail.contains("Refined"), "unexpected detail: {detail}");
        }
        other => panic!("expected lifecycle diag, got {other:?}"),
    }

    // Filesystem untouched.
    assert!(slice_dir.exists(), "slice dir must still exist");
    assert!(!project.specs_dir().join("login/spec.md").exists());
    assert!(!project.specs_dir().join("oauth/spec.md").exists());
    assert!(
        fs::read_dir(project.archive_dir()).unwrap().next().is_none(),
        "archive dir must still be empty"
    );
}

// ---------------------------------------------------------------------------
// Coherence failure rollback
// ---------------------------------------------------------------------------

#[test]
fn coherence_failure_rolls_back_all_writes() {
    let project = build_project();
    let slice_dir = project.slice_dir();

    // Overwrite the login delta with one that produces a coherence-invalid
    // baseline: an ADDED block missing its `ID:` line.
    fs::write(
        slice_dir.join("specs/login/spec.md"),
        "## ADDED Requirements\n\n### Requirement: Missing id\n\n#### Scenario: ok\n\n- ok\n",
    )
    .unwrap();

    let classes = omnia_classes(&slice_dir, &project.root);
    let err = slice::commit(&slice_dir, &classes, &project.archive_dir(), Timestamp::now(), false)
        .expect_err("expected coherence failure");
    match err {
        Error::Diag { detail: msg, .. } => {
            assert!(
                msg.contains("login:") && msg.contains("has no ID: line"),
                "unexpected merge error: {msg}"
            );
        }
        other => panic!("expected Error::Diag, got {other:?}"),
    }

    // Nothing on disk has moved or been created.
    assert!(slice_dir.exists(), "slice dir must still exist");
    let meta = SliceMetadata::load(&slice_dir).unwrap();
    assert_eq!(meta.status, LifecycleStatus::Built);
    assert!(!project.specs_dir().join("login/spec.md").exists());
    assert!(!project.specs_dir().join("oauth/spec.md").exists());
    assert!(
        fs::read_dir(project.archive_dir()).unwrap().next().is_none(),
        "archive must remain empty"
    );
}

// ---------------------------------------------------------------------------
// Archive naming (already covered by happy path; add a stand-alone assertion
// so a regex-only break doesn't hide inside the bigger happy-path test).
// ---------------------------------------------------------------------------

#[test]
fn archive_subdirectory_is_date_prefixed() {
    let project = build_project();
    let classes = omnia_classes(&project.slice_dir(), &project.root);
    slice::commit(&project.slice_dir(), &classes, &project.archive_dir(), Timestamp::now(), false)
        .expect("merge ok");

    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}-feature-x$").unwrap();
    let names: Vec<String> = fs::read_dir(project.archive_dir())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        names.iter().any(|n| re.is_match(n)),
        "archive names {names:?} do not include a YYYY-MM-DD-feature-x entry"
    );
}

// ---------------------------------------------------------------------------
// Contract file copying
// ---------------------------------------------------------------------------

#[test]
fn merge_copies_contract_files_to_baseline() {
    let project = build_project();
    let slice_dir = project.slice_dir();

    fs::create_dir_all(slice_dir.join("contracts/schemas")).expect("mkdir schemas");
    fs::create_dir_all(slice_dir.join("contracts/http")).expect("mkdir http");
    fs::write(slice_dir.join("contracts/schemas/test.yaml"), "adapter: test\n")
        .expect("write schema");
    fs::write(slice_dir.join("contracts/http/api.yaml"), "openapi: 3.1\n").expect("write api");

    let classes = omnia_classes(&slice_dir, &project.root);
    let merged =
        slice::commit(&slice_dir, &classes, &project.archive_dir(), Timestamp::now(), false)
            .expect("merge ok");

    let baseline_contracts = project.contracts_dir();
    assert!(
        baseline_contracts.join("schemas/test.yaml").is_file(),
        "schemas/test.yaml missing from baseline contracts"
    );
    assert!(
        baseline_contracts.join("http/api.yaml").is_file(),
        "http/api.yaml missing from baseline contracts"
    );

    let archived = find_archived_metadata(&project);
    assert!(archived.summary.contains("2 contracts"), "unexpected summary: {}", archived.summary);
    assert!(
        archived.summary.contains(&format!("{} specs", merged.len())),
        "unexpected summary: {}",
        archived.summary
    );
}

#[test]
fn merge_replaces_baseline_contracts() {
    let project = build_project();
    let slice_dir = project.slice_dir();

    let baseline_contracts = project.contracts_dir();
    fs::create_dir_all(baseline_contracts.join("schemas")).expect("mkdir baseline schemas");
    fs::write(baseline_contracts.join("schemas/test.yaml"), "old content\n")
        .expect("write old baseline");

    fs::create_dir_all(slice_dir.join("contracts/schemas")).expect("mkdir slice schemas");
    fs::write(slice_dir.join("contracts/schemas/test.yaml"), "new content\n")
        .expect("write new slice");

    let classes = omnia_classes(&slice_dir, &project.root);
    slice::commit(&slice_dir, &classes, &project.archive_dir(), Timestamp::now(), false)
        .expect("merge ok");

    let content = fs::read_to_string(baseline_contracts.join("schemas/test.yaml")).unwrap();
    assert_eq!(content, "new content\n", "contract file should be replaced");
}

#[test]
fn merge_leaves_untouched_contracts() {
    let project = build_project();
    let slice_dir = project.slice_dir();

    let baseline_contracts = project.contracts_dir();
    fs::create_dir_all(baseline_contracts.join("schemas")).expect("mkdir baseline schemas");
    fs::write(baseline_contracts.join("schemas/existing.yaml"), "existing content\n")
        .expect("write existing");

    fs::create_dir_all(slice_dir.join("contracts/schemas")).expect("mkdir slice schemas");
    fs::write(slice_dir.join("contracts/schemas/new.yaml"), "new content\n").expect("write new");

    let classes = omnia_classes(&slice_dir, &project.root);
    slice::commit(&slice_dir, &classes, &project.archive_dir(), Timestamp::now(), false)
        .expect("merge ok");

    assert!(
        baseline_contracts.join("schemas/existing.yaml").is_file(),
        "existing contract should still be present"
    );
    assert!(
        baseline_contracts.join("schemas/new.yaml").is_file(),
        "new contract should be present"
    );
    let existing = fs::read_to_string(baseline_contracts.join("schemas/existing.yaml")).unwrap();
    assert_eq!(existing, "existing content\n", "existing contract should be untouched");
}

#[test]
fn merge_without_contracts_dir() {
    let project = build_project();
    let slice_dir = project.slice_dir();

    assert!(!slice_dir.join("contracts").exists(), "precondition: no contracts dir");

    let classes = omnia_classes(&slice_dir, &project.root);
    let merged =
        slice::commit(&slice_dir, &classes, &project.archive_dir(), Timestamp::now(), false)
            .expect("merge ok");
    assert!(!merged.is_empty(), "should still merge specs");

    let baseline_contracts = project.contracts_dir();
    assert!(
        !baseline_contracts.exists(),
        "no root contracts/ should be created when slice has no contracts"
    );
    assert!(
        !project.root.join(".specify/contracts").exists(),
        "merge must not create the legacy .specify/contracts/ either"
    );

    let archived = find_archived_metadata(&project);
    assert!(
        !archived.summary.contains("contract"),
        "summary should not mention contracts: {}",
        archived.summary
    );
}

/// Helper: find the archived `metadata.yaml` and return its phase outcome.
fn find_archived_metadata(project: &Project) -> Outcome {
    let archived: Vec<_> = fs::read_dir(project.archive_dir())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .collect();
    assert_eq!(archived.len(), 1, "expected exactly one archived slice");
    let meta = SliceMetadata::load(&archived[0].path()).expect("load archived metadata");
    meta.outcome.expect("expected outcome to be stamped")
}

// ---------------------------------------------------------------------------
// A3 — composition-overwrite merge gate
// ---------------------------------------------------------------------------

/// Path to the composition baseline (under the first `ThreeWayMerge`
/// class — `specs`).
fn composition_baseline(project: &Project) -> PathBuf {
    project.specs_dir().join("composition.yaml")
}

/// Seed a non-empty composition baseline.
fn seed_composition_baseline(project: &Project) {
    fs::write(composition_baseline(project), "version: 1\nscreens:\n  home:\n    name: Home\n")
        .expect("write composition baseline");
}

/// Write the slice's top-level `composition.yaml`.
fn write_slice_composition(project: &Project, body: &str) {
    fs::write(project.slice_dir().join("composition.yaml"), body).expect("write slice composition");
}

#[test]
fn comp_gate_blocks_overwrite() {
    let project = build_project();
    let slice_dir = project.slice_dir();
    let classes = omnia_classes(&slice_dir, &project.root);

    seed_composition_baseline(&project);
    write_slice_composition(&project, "version: 1\nscreens:\n  settings:\n    name: Settings\n");

    let err = slice::commit(&slice_dir, &classes, &project.archive_dir(), Timestamp::now(), false)
        .expect_err("whole-document replacement over a non-empty baseline must be blocked");
    match err {
        Error::Diag { code, .. } => assert_eq!(code, "composition-baseline-overwrite-blocked"),
        other => panic!("expected composition-baseline-overwrite-blocked diag, got {other:?}"),
    }

    // Gate is a precondition: nothing on disk has moved, and the
    // baseline is untouched.
    assert!(slice_dir.exists(), "slice dir must still exist after a blocked merge");
    let baseline = fs::read_to_string(composition_baseline(&project)).unwrap();
    assert!(baseline.contains("home"), "baseline must be preserved");
    assert!(!baseline.contains("settings"), "baseline must not be overwritten");
}

#[test]
fn comp_gate_allows_with_flag() {
    let project = build_project();
    let slice_dir = project.slice_dir();
    let classes = omnia_classes(&slice_dir, &project.root);

    seed_composition_baseline(&project);
    write_slice_composition(&project, "version: 1\nscreens:\n  settings:\n    name: Settings\n");

    slice::commit(&slice_dir, &classes, &project.archive_dir(), Timestamp::now(), true)
        .expect("override flag authorises the whole-document replacement");

    let baseline = fs::read_to_string(composition_baseline(&project)).unwrap();
    assert!(baseline.contains("settings"), "baseline should be replaced with the slice document");
    assert!(!baseline.contains("home"), "whole-document replacement drops the prior screen");
    assert!(!slice_dir.exists(), "slice should be archived after a successful merge");
}

#[test]
fn comp_delta_accumulates() {
    let project = build_project();
    let slice_dir = project.slice_dir();
    let classes = omnia_classes(&slice_dir, &project.root);

    seed_composition_baseline(&project);
    write_slice_composition(
        &project,
        "version: 1\ndelta:\n  added:\n    settings:\n      name: Settings\n  modified: {}\n  removed: {}\n",
    );

    slice::commit(&slice_dir, &classes, &project.archive_dir(), Timestamp::now(), false)
        .expect("delta-format composition is never gated and accumulates");

    let baseline = fs::read_to_string(composition_baseline(&project)).unwrap();
    assert!(baseline.contains("home"), "prior screen must be retained");
    assert!(baseline.contains("settings"), "new screen must be added");
}

#[test]
fn comp_gate_skips_without_baseline() {
    let project = build_project();
    let slice_dir = project.slice_dir();
    let classes = omnia_classes(&slice_dir, &project.root);

    // No baseline composition exists yet — the gate must not fire.
    assert!(!composition_baseline(&project).exists());
    write_slice_composition(&project, "version: 1\nscreens:\n  home:\n    name: Home\n");

    slice::commit(&slice_dir, &classes, &project.archive_dir(), Timestamp::now(), false)
        .expect("whole-document format establishes the initial baseline when none exists");

    let baseline = fs::read_to_string(composition_baseline(&project)).unwrap();
    assert!(baseline.contains("home"), "initial baseline should be established");
}

// ---------------------------------------------------------------------------
// slice::preview — contract entries
// ---------------------------------------------------------------------------

#[test]
fn preview_no_contracts_returns_empty_list() {
    let project = build_project();
    let classes = omnia_classes(&project.slice_dir(), &project.root);
    let result = slice::preview(&project.slice_dir(), &classes).expect("preview should succeed");
    assert!(!result.three_way.is_empty(), "should have spec entries");
    assert!(result.opaque.is_empty(), "should have no opaque-replace entries");
}

#[test]
fn preview_new_contracts_added() {
    let project = build_project();
    let slice_dir = project.slice_dir();

    fs::create_dir_all(slice_dir.join("contracts/schemas")).expect("mkdir");
    fs::create_dir_all(slice_dir.join("contracts/http")).expect("mkdir");
    fs::write(slice_dir.join("contracts/schemas/user.yaml"), "adapter: user\n").expect("write");
    fs::write(slice_dir.join("contracts/http/api.yaml"), "openapi: 3.1\n").expect("write");

    let classes = omnia_classes(&slice_dir, &project.root);
    let result = slice::preview(&slice_dir, &classes).expect("preview should succeed");

    assert_eq!(result.opaque.len(), 2);
    // Sorted by (class_name, relative_path) — both entries are in the
    // `contracts` class, so the secondary sort by relative_path takes
    // over.
    assert_eq!(result.opaque[0].class_name, "contracts");
    assert_eq!(result.opaque[0].relative_path, "http/api.yaml");
    assert_eq!(result.opaque[0].action, OpaqueAction::Added);
    assert_eq!(result.opaque[1].class_name, "contracts");
    assert_eq!(result.opaque[1].relative_path, "schemas/user.yaml");
    assert_eq!(result.opaque[1].action, OpaqueAction::Added);
}

#[test]
fn preview_baseline_contracts_replaced() {
    let project = build_project();
    let slice_dir = project.slice_dir();

    let baseline_contracts = project.contracts_dir();
    fs::create_dir_all(baseline_contracts.join("schemas")).expect("mkdir baseline");
    fs::write(baseline_contracts.join("schemas/user.yaml"), "old\n").expect("write baseline");

    fs::create_dir_all(slice_dir.join("contracts/schemas")).expect("mkdir slice");
    fs::write(slice_dir.join("contracts/schemas/user.yaml"), "new\n").expect("write slice");
    fs::write(slice_dir.join("contracts/schemas/order.yaml"), "new\n").expect("write slice");

    let classes = omnia_classes(&slice_dir, &project.root);
    let result = slice::preview(&slice_dir, &classes).expect("preview should succeed");

    assert_eq!(result.opaque.len(), 2);
    let order = result.opaque.iter().find(|c| c.relative_path == "schemas/order.yaml").unwrap();
    assert_eq!(order.action, OpaqueAction::Added);
    let user = result.opaque.iter().find(|c| c.relative_path == "schemas/user.yaml").unwrap();
    assert_eq!(user.action, OpaqueAction::Replaced);
}
