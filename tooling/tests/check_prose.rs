use std::fs;
use std::path::{Path, PathBuf};

use tooling::check::{InvocationPositional, OperationalVocabulary, SkillNumericCaps};
use tooling::finding::Check;
use tooling::Context;

fn fixture_root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/prose")
        .join(name)
}

fn scaffold_framework_root(root: &Path) {
    fs::create_dir_all(root.join("plugins")).expect("plugins dir");
    fs::create_dir_all(root.join("adapters")).expect("adapters dir");
    fs::create_dir_all(root.join("tooling")).expect("tooling dir");
    fs::write(root.join("tooling/Cargo.toml"), "").expect("tooling manifest");
}

fn context_for_fixture(name: &str) -> Context {
    let root = fixture_root(name);
    scaffold_framework_root(&root);
    Context::from_manifest_dir(root.join("tooling")).expect("framework root resolves")
}

#[test]
fn operational_vocabulary_flags_stale_terms() {
    let ctx = context_for_fixture("stale-vocabulary");
    let findings = OperationalVocabulary.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "prose.operational-vocabulary");
    assert!(findings[0].message.contains("specify validate"));
    assert!(findings[0].message.contains("specify slice validate"));
}

#[test]
fn operational_vocabulary_allows_rfcs_prefix() {
    let ctx = context_for_fixture("allowed-rfc");
    let findings = OperationalVocabulary.run(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn invocation_positionals_flags_flag_after_skill() {
    let ctx = context_for_fixture("flag-after-skill");
    let findings = InvocationPositional.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "prose.invocation-positional");
    assert!(findings[0].message.contains("docs/bad.md"));
}

#[test]
fn invocation_positionals_flags_continued_invocation() {
    let ctx = context_for_fixture("flag-after-skill-continued");
    let findings = InvocationPositional.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "prose.invocation-positional");
    assert!(findings[0].message.contains("3-4"));
}

#[test]
fn invocation_positionals_allows_cli_flags() {
    let ctx = context_for_fixture("cli-flag-allowed");
    let findings = InvocationPositional.run(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn skill_numeric_caps_detects_drift() {
    let ctx = context_for_fixture("cap-drift");
    let findings = SkillNumericCaps.run(&ctx);
    assert_eq!(findings.len(), 4);
    assert!(findings.iter().all(|f| f.rule_id == "prose.numeric-cap-exceeded"));
    assert!(findings
        .iter()
        .any(|f| f.message.contains("description cap drift")));
    assert!(findings.iter().any(|f| f.message.contains("body cap drift")));
}

#[test]
fn skill_numeric_caps_passes_when_synced() {
    let ctx = context_for_fixture("caps-ok");
    let findings = SkillNumericCaps.run(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn prose_checks_pass_on_real_repo() {
    let ctx = Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root");
    let mut findings = Vec::new();
    findings.extend(OperationalVocabulary.run(&ctx));
    findings.extend(SkillNumericCaps.run(&ctx));
    findings.extend(InvocationPositional.run(&ctx));
    assert!(
        findings.is_empty(),
        "expected clean prose checks, got: {findings:?}"
    );
}
