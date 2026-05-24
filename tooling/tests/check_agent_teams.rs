use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::symlink;

use tooling::check::agent_teams;
use tooling::Context;

const CANONICAL_CONTENT: &str = "# Review team protocol\n\ncanonical body\n";
const CANONICAL_REL: &str = "docs/reference/review-team-protocol.md";

fn scaffold_framework_root(root: &Path) -> PathBuf {
    fs::create_dir_all(root.join("tooling")).expect("tooling dir");
    fs::write(root.join("tooling/Cargo.toml"), "[package]\nname = \"tooling\"\n").expect("cargo.toml");
    fs::create_dir_all(root.join("plugins")).expect("plugins dir");
    fs::create_dir_all(root.join("adapters/targets")).expect("targets dir");
    fs::create_dir_all(root.join("docs/reference")).expect("docs dir");
    fs::write(root.join(CANONICAL_REL), CANONICAL_CONTENT).expect("canonical doc");
    root.to_path_buf()
}

fn overlay_path(root: &Path, target: &str) -> PathBuf {
    root.join("adapters/targets")
        .join(target)
        .join("references")
        .join("agent-teams.md")
}

fn ctx_for(root: &Path) -> Context {
    Context::from_framework_root(root).expect("framework root")
}

#[test]
fn canonical_symlink_overlay_passes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    let target = "good-symlink";
    fs::create_dir_all(root.join("adapters/targets").join(target).join("references")).expect("refs");
    #[cfg(unix)]
    symlink(
        root.join(CANONICAL_REL),
        overlay_path(&root, target),
    )
    .expect("symlink");

    let findings = agent_teams::run(&ctx_for(&root));
    assert!(findings.is_empty(), "expected no findings: {findings:?}");
}

#[test]
fn matching_regular_file_overlay_passes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    let target = "good-copy";
    fs::create_dir_all(root.join("adapters/targets").join(target).join("references")).expect("refs");
    fs::write(overlay_path(&root, target), CANONICAL_CONTENT).expect("overlay copy");

    let findings = agent_teams::run(&ctx_for(&root));
    assert!(findings.is_empty(), "expected no findings: {findings:?}");
}

#[test]
fn drifted_regular_file_overlay_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    let target = "bad-drift";
    fs::create_dir_all(root.join("adapters/targets").join(target).join("references")).expect("refs");
    fs::write(overlay_path(&root, target), "# stale copy\n").expect("drifted overlay");

    let findings = agent_teams::run(&ctx_for(&root));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "agent-teams.non-canonical-overlay");
    assert!(findings[0].message.contains("content drifted"));
}

#[test]
#[cfg(unix)]
fn wrong_symlink_target_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    let wrong = root.join("docs/reference/wrong.md");
    fs::write(&wrong, "not canonical\n").expect("wrong doc");

    let target = "bad-symlink";
    fs::create_dir_all(root.join("adapters/targets").join(target).join("references")).expect("refs");
    symlink(&wrong, overlay_path(&root, target)).expect("symlink");

    let findings = agent_teams::run(&ctx_for(&root));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "agent-teams.non-canonical-overlay");
    assert!(findings[0].message.contains("symlink resolves to"));
}

#[test]
#[cfg(unix)]
fn broken_symlink_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    let target = "broken-symlink";
    fs::create_dir_all(root.join("adapters/targets").join(target).join("references")).expect("refs");
    symlink(
        root.join("docs/reference/missing.md"),
        overlay_path(&root, target),
    )
    .expect("symlink");

    let findings = agent_teams::run(&ctx_for(&root));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "agent-teams.non-canonical-overlay");
    assert!(findings[0].message.contains("symlink does not resolve"));
}

#[test]
fn missing_canonical_doc_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    fs::remove_file(root.join(CANONICAL_REL)).expect("remove canonical");

    let findings = agent_teams::run(&ctx_for(&root));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "agent-teams.missing-canonical");
    assert!(findings[0].message.contains("is missing"));
}

#[test]
fn real_repo_agent_teams_overlays_pass() {
    let ctx = Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root");
    let findings = agent_teams::run(&ctx);
    assert!(
        findings.is_empty(),
        "real repo overlays should be canonical: {findings:?}"
    );
}
