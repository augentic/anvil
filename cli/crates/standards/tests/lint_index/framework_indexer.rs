//! Integration test for the `scan_profile: framework` indexer.
//!
//! Drives `lint::index::build` against the checked-in
//! `fixtures/lint/framework_minimal/` tree, minting one relative
//! symlink at test time (`agent-teams.md` → `docs/reference/review-team-protocol.md`)
//! because committed relative symlinks survive `git` poorly across
//! operating systems.
//!
//! Two invariants the framework profile owes:
//!
//! 1. The produced [`WorkspaceModel`] validates against the embedded
//!    [`WORKSPACE_MODEL_JSON_SCHEMA`] under the framework profile —
//!    every new framework-only entity family round-trips through the
//!    schema.
//! 2. Every framework extractor (`skill`, `adapter`, `marketplace`,
//!    `brief`) emits at least one fact against the
//!    minimal fixture, and the followed `agent-teams.md` symlink
//!    surfaces both endpoints plus a SHA-256 of the resolved target's
//!    bytes per the standards-layer contract §F1.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use specify_schema::{ValidationStatus, WORKSPACE_MODEL_JSON_SCHEMA, validate_value};
use specify_standards::lint::ScanProfile;
use specify_standards::lint::index::build;
use tempfile::TempDir;

const FIXTURE_NAME: &str = "framework_minimal";

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_src() -> PathBuf {
    crate_root().join("tests/fixtures/lint").join(FIXTURE_NAME)
}

/// Stage the fixture into a tempdir and add the followable
/// `agent-teams.md` symlink at runtime.
fn stage_fixture() -> TempDir {
    let tempdir = tempfile::tempdir().expect("tempdir");
    crate::common::copy_dir(&fixture_src(), tempdir.path());

    // `agent-teams.md` symlink in `adapters/targets/omnia/references/`
    // pointing at the canonical `docs/reference/review-team-protocol.md`.
    // The `references/` parent isn't checked in (git doesn't track empty
    // dirs); create it before placing the link so the cross-platform
    // symlink calls below have a valid parent.
    let link_dir = tempdir.path().join("adapters/targets/omnia/references");
    fs::create_dir_all(&link_dir).expect("create link parent");
    let link_path = link_dir.join("agent-teams.md");
    let link_target = "../../../../docs/reference/review-team-protocol.md";
    #[cfg(unix)]
    std::os::unix::fs::symlink(link_target, &link_path).expect("create unix symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(link_target, &link_path).expect("create windows symlink");

    tempdir
}

fn assert_schema_valid(value: &Value) {
    let summaries = validate_value(
        value,
        WORKSPACE_MODEL_JSON_SCHEMA,
        "workspace-model",
        "framework-indexer fixture",
    );
    let failures: Vec<_> =
        summaries.iter().filter(|s| matches!(s.status, ValidationStatus::Fail)).collect();
    assert!(failures.is_empty(), "WorkspaceModel must validate; got {failures:#?}");
}

#[test]
fn extractors_emit_facts() {
    let tempdir = stage_fixture();
    let model = build(tempdir.path(), ScanProfile::Framework, &[], &[]).expect("build ok");
    let value = serde_json::to_value(&model).expect("serialise");
    assert_schema_valid(&value);

    assert_eq!(model.scan_profile, ScanProfile::Framework);

    assert!(!model.skills.is_empty(), "skill extractor must emit at least one fact");
    let skill = &model.skills[0];
    assert_eq!(skill.name, "specify-init");
    assert_eq!(skill.plugin, "spec");
    assert!(skill.body_line_count.unwrap_or(0) >= 1);

    assert!(
        model.adapter_manifests.len() >= 2,
        "adapter extractor must emit one fact per `adapter.yaml` (sources + targets)"
    );
    let names: Vec<&str> = model.adapter_manifests.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"intent"));
    assert!(names.contains(&"omnia"));

    assert!(
        model.briefs.len() >= 2,
        "brief extractor must emit one fact per `briefs/*.md` (sources + targets)"
    );
    assert!(model.briefs.iter().any(|b| b.operation == "survey"));
    assert!(model.briefs.iter().any(|b| b.operation == "shape"));
    let survey_brief = model.briefs.iter().find(|b| b.operation == "survey").expect("survey brief");
    assert_eq!(survey_brief.sections, vec!["Inputs".to_string(), "Output contract".to_string()]);
}

#[test]
fn symlink_records_endpoint_and_sha256() {
    let tempdir = stage_fixture();
    let model = build(tempdir.path(), ScanProfile::Framework, &[], &[]).expect("build ok");

    let symlink = model
        .symlinks
        .iter()
        .find(|s| s.path == "adapters/targets/omnia/references/agent-teams.md")
        .expect("symlink fact recorded");
    assert!(!symlink.broken);
    assert_eq!(
        symlink.resolved_target.as_deref(),
        Some("docs/reference/review-team-protocol.md"),
        "framework symlink fact records the canonical endpoint"
    );
}

#[test]
fn walk_byte_stable() {
    let tempdir = stage_fixture();
    let first = build(tempdir.path(), ScanProfile::Framework, &[], &[]).expect("first build");
    let second = build(tempdir.path(), ScanProfile::Framework, &[], &[]).expect("second build");
    let first_json = serde_json::to_string_pretty(&first).expect("first serialise");
    let second_json = serde_json::to_string_pretty(&second).expect("second serialise");
    assert_eq!(
        first_json, second_json,
        "two framework indexer runs must produce byte-identical JSON"
    );
}
