//! Integration coverage for the CH-14 resolved-rules envelope
//! (`specify_standards::build_resolved_rules`). Re-homed from the former
//! `rules/resolve/sort/tests.rs` unit module: the versioned envelope plus the
//! sorted rule list run through the public builder over a real overlay tree.
//! The path-anchoring and byte-stability properties are asserted end-to-end by
//! `tests/rules.rs::export::{paths_anchored_not_absolute,
//! stable_ordering_byte_identical}`, so those two former tests were deleted.

use std::fs;
use std::path::Path;

use specify_standards::{ResolveInputs, ResolvedRule, ResolvedRules, build_resolved_rules};
use tempfile::TempDir;

/// A minimal frontmatter + body that parses through CH-11 and validates
/// against the codex-rule schema.
fn rule_markdown(id: &str, title: &str, severity: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: {title}\nseverity: {severity}\ntrigger: Synthetic CH-14 build_resolved_rules fixture trigger sentence long enough for schema.\n---\n\n## Rule\n\nBody for {id}.\n"
    )
}

fn write_rule(path: &Path, id: &str, title: &str, severity: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, rule_markdown(id, title, severity)).expect("write rule fixture");
}

fn run_build(rules_root: &Path, project_dir: &Path) -> ResolvedRules {
    let sources: Vec<String> = Vec::new();
    let inputs = ResolveInputs {
        project_dir,
        rules_root: Some(rules_root),
        target_adapter: "demo-target",
        source_adapters: &sources,
        artifact_paths: &[],
        languages: &[],
        include_deprecated: false,
        include_unmatched: false,
    };
    build_resolved_rules(&inputs).expect("build_resolved_rules succeeds")
}

fn ids_of_rules(rules: &[ResolvedRule]) -> Vec<&str> {
    rules.iter().map(|r| r.rule_id.as_str()).collect()
}

/// `build_resolved_rules` integration: the wire envelope is versioned,
/// target/source carry through, and rules emerge sorted per the closed
/// four-tuple.
#[test]
fn build_emits_versioned_envelope() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("codex/rules/universal/uni-002.md"),
        "UNI-002",
        "Important shared",
        "important",
    );
    write_rule(
        &rules_root.path().join("codex/rules/universal/uni-001.md"),
        "UNI-001",
        "Critical shared",
        "critical",
    );
    write_rule(
        &project.path().join("adapters/targets/demo-target/prose/rules/org-001.md"),
        "ORG-001",
        "Important target",
        "important",
    );

    let resolved = run_build(rules_root.path(), project.path());

    assert_eq!(resolved.version, 1);
    assert_eq!(resolved.target_adapter, "demo-target");
    assert!(resolved.source_adapters.is_empty());
    assert_eq!(resolved.rules.len(), 3);
    // UNI-001 is Critical (beats ORG-001 Important); ORG-001 is Important +
    // Target (beats UNI-002 Important + Shared); UNI-002 trails.
    assert_eq!(ids_of_rules(&resolved.rules), vec!["UNI-001", "ORG-001", "UNI-002"]);
}
