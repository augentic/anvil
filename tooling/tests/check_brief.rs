use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use tooling::check::BriefCheck;
use tooling::finding::Check;
use tooling::Context;

fn scaffold_framework_root(base: &Path) -> PathBuf {
    let root = base.join("framework");
    fs::create_dir_all(root.join("tooling")).expect("tooling dir");
    fs::write(root.join("tooling/Cargo.toml"), "stub").expect("tooling manifest");
    fs::create_dir_all(root.join("plugins/spec")).expect("plugins dir");
    fs::create_dir_all(root.join("adapters/targets/demo/briefs/build"))
        .expect("targets briefs dir");
    root
}

fn write_brief(root: &Path, rel: &str, content: &str) -> PathBuf {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("brief parent dir");
    }
    fs::write(&path, content).expect("write brief");
    path
}

fn context_for(root: &Path) -> Context {
    Context::from_manifest_dir(root.join("tooling")).expect("framework root resolves")
}

fn line_body(prefix: &str, count: usize) -> String {
    (0..count)
        .map(|i| format!("{prefix} line {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn parent_brief_exceeding_cap_emits_size_finding() {
    let tmp = TempDir::new().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    write_brief(
        &root,
        "adapters/targets/demo/briefs/build.md",
        &format!("# Build\n\n{}", line_body("line", 150)),
    );

    let findings = BriefCheck.run(&context_for(&root));
    let size = findings
        .iter()
        .find(|f| f.rule_id == "brief.exceeds-size-limit")
        .expect("expected size finding");
    assert!(size.message.contains("parent brief is 151 non-blank lines"));
    assert!(size.message.contains("exceeds hard cap 150"));
}

#[test]
fn phase_sub_brief_exceeding_hard_cap_emits_size_finding() {
    let tmp = TempDir::new().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    write_brief(
        &root,
        "adapters/targets/demo/briefs/build/phase.md",
        &format!("# Phase\n\n{}", line_body("line", 800)),
    );

    let findings = BriefCheck.run(&context_for(&root));
    let size = findings
        .iter()
        .find(|f| f.rule_id == "brief.exceeds-size-limit")
        .expect("expected size finding");
    assert!(size.message.contains("phase sub-brief is 801 non-blank lines"));
    assert!(size.message.contains("exceeds hard cap 800"));
}

#[test]
fn brief_with_frontmatter_emits_frontmatter_finding() {
    let tmp = TempDir::new().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    write_brief(
        &root,
        "adapters/targets/demo/briefs/extract.md",
        "---\ndescription: drift\n---\n\n# Extract\n",
    );

    let findings = BriefCheck.run(&context_for(&root));
    let fm = findings
        .iter()
        .find(|f| f.rule_id == "brief.frontmatter-forbidden")
        .expect("expected frontmatter finding");
    assert!(fm.message.contains("brief has YAML frontmatter"));
    assert!(fm.message.contains("docs/standards/skill-authoring.md#brief-authoring"));
}
