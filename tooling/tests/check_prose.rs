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
fn invocation_positionals_flags_continued_invocation() {
    let ctx = context_for_fixture("flag-after-skill-continued");
    let findings = InvocationPositional.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "prose.invocation-positional");
    assert!(findings[0].message.contains("3-4"));
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
