//! Umbrella smoke for the `path-pattern` candidate-set construction.
//!
//! `path-pattern` is a candidate-set filter, not a finder. The contract
//! is verified indirectly through the indexer + umbrella: a rule with
//! `path-pattern: *.rs` plus `regex: fn` MUST emit findings only for
//! the `*.rs` files reachable from the workspace model; exclusion
//! globs subtract from the include union.

use std::fs;

use specify_diagnostics::Severity;
use specify_standards::lint::ScanProfile;
use specify_standards::lint::eval::{EvalEnv, ToolRunner, evaluate_rules};
use specify_standards::lint::index::build;
use specify_standards::rules::HintKind;

use crate::eval_support::{NoToolRunner, hint, make_rule};

#[test]
fn path_pattern_narrows_candidates() {
    let tmp = tempfile::tempdir().expect("tmp");
    fs::write(tmp.path().join("a.rs"), "fn main() {}\n").expect("write a.rs");
    fs::write(tmp.path().join("b.rs"), "fn other() {}\n").expect("write b.rs");
    fs::write(tmp.path().join("readme.md"), "Function-shaped fn-text inside markdown.\n")
        .expect("write readme");

    let model = build(tmp.path(), ScanProfile::Project, &[], &[]).expect("build");
    let rule = make_rule(
        "UNI-901",
        vec![hint(HintKind::PathPattern, "*.rs"), hint(HintKind::Regex, "\\bfn\\b")],
    );
    let runner: &dyn ToolRunner = &NoToolRunner;

    let env = EvalEnv {
        model: &model,
        project_dir: tmp.path(),
        tool_runner: runner,
        cli_contract: None,
    };
    let (findings, _next_id) =
        evaluate_rules(std::slice::from_ref(&rule), env, 1, &[]).expect("evaluate ok");

    assert_eq!(findings.len(), 2, "one finding per matched .rs file");
    let mut paths: Vec<&str> =
        findings.iter().map(|f| f.location.as_ref().expect("location set").path.as_str()).collect();
    paths.sort_unstable();
    assert_eq!(paths, vec!["a.rs", "b.rs"], "regex must only see path-pattern survivors");
    for finding in &findings {
        assert_eq!(finding.severity, Severity::Important);
        assert!(finding.fingerprint.starts_with("sha256:"));
    }
}

#[test]
fn path_pattern_empty_drops_findings() {
    let tmp = tempfile::tempdir().expect("tmp");
    fs::write(tmp.path().join("readme.md"), "fn-shaped content\n").expect("write readme");
    let model = build(tmp.path(), ScanProfile::Project, &[], &[]).expect("build");
    let rule = make_rule(
        "UNI-902",
        vec![hint(HintKind::PathPattern, "*.rs"), hint(HintKind::Regex, "fn")],
    );
    let runner: &dyn ToolRunner = &NoToolRunner;
    let env = EvalEnv {
        model: &model,
        project_dir: tmp.path(),
        tool_runner: runner,
        cli_contract: None,
    };
    let (findings, _next_id) =
        evaluate_rules(std::slice::from_ref(&rule), env, 1, &[]).expect("evaluate ok");
    assert!(findings.is_empty(), "no .rs files survived path-pattern filter");
}

#[test]
fn path_pattern_exclusion_carves_out_paths() {
    let tmp = tempfile::tempdir().expect("tmp");
    fs::create_dir_all(tmp.path().join("docs/explanation")).expect("mkdir");
    fs::write(tmp.path().join("docs/bad.md"), "fn main() {}\n").expect("write bad");
    fs::write(tmp.path().join("docs/explanation/decision-log.md"), "fn other() {}\n")
        .expect("write allowlisted");

    let model = build(tmp.path(), ScanProfile::Project, &[], &[]).expect("build");
    let rule = make_rule(
        "UNI-903",
        vec![
            hint(HintKind::PathPattern, "docs/**/*.md"),
            hint(HintKind::PathPattern, "!docs/explanation/decision-log.md"),
            hint(HintKind::Regex, "\\bfn\\b"),
        ],
    );
    let runner: &dyn ToolRunner = &NoToolRunner;
    let env = EvalEnv {
        model: &model,
        project_dir: tmp.path(),
        tool_runner: runner,
        cli_contract: None,
    };
    let (findings, _next_id) =
        evaluate_rules(std::slice::from_ref(&rule), env, 1, &[]).expect("evaluate ok");

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].location.as_ref().expect("loc").path, "docs/bad.md");
}
