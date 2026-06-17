use super::*;

#[test]
fn include_set_accepts_documented_prefixes() {
    assert!(is_included("adapters/sources/intent/adapter.yaml"));
    assert!(is_included("plugins/spec/skills/init/SKILL.md"));
    assert!(is_included("docs/standards/style.md"));
    assert!(is_included(".cursor/rules/project.mdc"));
    assert!(is_included(".cursor-plugin/marketplace.json"));
    assert!(is_included("rfcs/roadmap.md"));
    assert!(is_included("scripts/foo.sh"));
    assert!(is_included("schemas/lint/workspace-model.schema.json"));
}

#[test]
fn include_set_accepts_top_level_markers() {
    assert!(is_included("AGENTS.md"));
    assert!(is_included("crates/standards/AGENTS.md"));
    assert!(is_included("crates/standards/REVIEW.md"));
}

#[test]
fn include_set_accepts_specify_toml() {
    assert!(is_included("Specify.toml"));
}

#[test]
fn include_set_accepts_root_readme_only() {
    // Root README.md is a lintable documentation surface; nested
    // readmes outside the documented prefixes stay excluded.
    assert!(is_included("README.md"));
    assert!(!is_included("crates/standards/README.md"));
}

#[test]
fn include_set_rejects_unrelated_paths() {
    assert!(!is_included("Cargo.toml"));
    assert!(!is_included("src/lib.rs"));
}
