//! Integration test for the `WorkspaceModel` file scan project indexer.
//!
//! Drives `lint::index::build` against the checked-in
//! `fixtures/lint/minimal/` tree, augmenting it at runtime with the
//! few entries that cannot be committed cleanly (a `.gitignore`-ignored
//! sibling, an `always-ignore`-globbed `target/` directory, and a
//! relative symlink — committed symlinks are fragile across operating
//! systems). The fixture's `README.md` documents the rationale.
//!
//! Two invariants are asserted:
//!
//! 1. The produced [`WorkspaceModel`] validates against the embedded
//!    [`WORKSPACE_MODEL_JSON_SCHEMA`] and matches the checked-in
//!    golden once the tempdir prefix is normalised to `<TEMPDIR>`.
//! 2. Two consecutive invocations produce byte-identical pretty-printed
//!    JSON envelopes — the §"Stability" guarantee from `WorkspaceModel` stability.
//!
//! Regenerate the golden with
//! `REGENERATE_GOLDENS=1 cargo nextest run -p specify-standards --test lint_index`
//! after a deliberate model change; see [`docs/standards/testing.md`].

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use specify_schema::{ValidationStatus, WORKSPACE_MODEL_JSON_SCHEMA, validate_value};
use specify_standards::lint::ScanProfile;
use specify_standards::lint::index::build;
use tempfile::TempDir;

const FIXTURE_NAME: &str = "minimal";

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_src() -> PathBuf {
    crate_root().join("tests/fixtures/lint").join(FIXTURE_NAME)
}

fn golden_path() -> PathBuf {
    crate_root().join("tests/fixtures/lint").join(format!("{FIXTURE_NAME}_workspace_model.json"))
}

fn stage_fixture() -> TempDir {
    let tempdir = tempfile::tempdir().expect("tempdir");
    crate::common::copy_dir(&fixture_src(), tempdir.path());

    // `.gitignore` + ignored.md cannot be committed cleanly inside
    // the fixture (the .gitignore would cause git to skip the
    // sibling), so they are minted at test time.
    fs::write(tempdir.path().join(".gitignore"), "ignored.md\n").expect("write .gitignore");
    fs::write(tempdir.path().join("ignored.md"), "# Should be ignored\n")
        .expect("write ignored.md");

    // `target/**` must be filtered out by the always-ignore globs.
    fs::create_dir_all(tempdir.path().join("target")).expect("create target dir");
    fs::write(tempdir.path().join("target/build.rs"), "// excluded\n").expect("write target file");

    // Relative symlink that exercises the symlink-fact recorder.
    #[cfg(unix)]
    std::os::unix::fs::symlink("doc.md", tempdir.path().join("link.md"))
        .expect("create unix symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file("doc.md", tempdir.path().join("link.md"))
        .expect("create windows symlink");

    tempdir
}

fn normalise(value: Value, tempdir: &Path) -> Value {
    match value {
        Value::String(s) => {
            let prefix = tempdir.to_string_lossy().into_owned();
            if s == prefix { Value::String("<TEMPDIR>".into()) } else { Value::String(s) }
        }
        Value::Array(items) => {
            Value::Array(items.into_iter().map(|v| normalise(v, tempdir)).collect())
        }
        Value::Object(map) => {
            Value::Object(map.into_iter().map(|(k, v)| (k, normalise(v, tempdir))).collect())
        }
        other => other,
    }
}

fn assert_schema_valid(value: &Value) {
    let summaries = validate_value(
        value,
        WORKSPACE_MODEL_JSON_SCHEMA,
        "workspace-model",
        "consumer-indexer fixture",
    );
    let failures: Vec<_> =
        summaries.iter().filter(|s| matches!(s.status, ValidationStatus::Fail)).collect();
    assert!(failures.is_empty(), "WorkspaceModel must validate; got {failures:#?}");
}

#[test]
fn minimal_fixture_matches_golden() {
    let tempdir = stage_fixture();
    let model = build(tempdir.path(), ScanProfile::Project, &[], &[]).expect("build ok");
    let value = serde_json::to_value(&model).expect("serialise");
    assert_schema_valid(&value);

    let normalised = normalise(value, tempdir.path());
    let pretty = serde_json::to_string_pretty(&normalised).expect("pretty");

    let path = golden_path();
    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        fs::write(&path, format!("{pretty}\n")).expect("write golden");
        return;
    }

    let expected = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "missing golden {}: {err}; regenerate with \
             REGENERATE_GOLDENS=1 cargo nextest run -p specify-standards --test lint_index",
            path.display()
        )
    });
    let expected_value: Value = serde_json::from_str(&expected).expect("parse golden");
    assert_eq!(
        normalised, expected_value,
        "WorkspaceModel diverged from golden. Actual:\n{pretty}"
    );
}

#[test]
fn byte_stable_across_runs() {
    let tempdir = stage_fixture();
    let first = build(tempdir.path(), ScanProfile::Project, &[], &[]).expect("first build");
    let second = build(tempdir.path(), ScanProfile::Project, &[], &[]).expect("second build");
    let first_json = serde_json::to_string_pretty(&first).expect("first serialise");
    let second_json = serde_json::to_string_pretty(&second).expect("second serialise");
    assert_eq!(first_json, second_json, "two indexer runs must produce byte-identical JSON");
}

#[test]
fn framework_scan_profile_now_accepted() {
    // `scan_profile: framework` is active; the consumer
    // fixture has no framework-shaped files so the framework walk
    // yields an essentially empty model, but it must no longer
    // surface `IndexError::UnsupportedScanProfile`.
    let tempdir = stage_fixture();
    let model = build(tempdir.path(), ScanProfile::Framework, &[], &[]).expect("framework ok");
    assert_eq!(model.scan_profile, ScanProfile::Framework);
}
