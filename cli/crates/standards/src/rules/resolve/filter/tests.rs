use std::path::Path;

use super::*;
use crate::rules::{Applicability, Deprecated, Origin, PathRoot, Severity};

fn make_rule(
    id: &str, applicability: Option<Applicability>, deprecated: Option<Deprecated>,
) -> Rule {
    Rule {
        id: id.into(),
        title: format!("{id} fixture"),
        severity: Severity::Important,
        trigger: "Synthetic CH-13 filter fixture trigger sentence long enough for schema.".into(),
        lint_mode: None,
        applicability,
        rule_hints: None,
        references: None,
        deprecated,
        body: format!("## Rule\n\nBody for {id}.\n"),
    }
}

fn make_entry(
    id: &str, applicability: Option<Applicability>, deprecated: Option<Deprecated>,
) -> ResolvedRuleEntry {
    ResolvedRuleEntry {
        rule: make_rule(id, applicability, deprecated),
        origin: Origin::Shared,
        path_root: PathRoot::RulesRoot,
        path: format!("adapters/shared/rules/universal/{id}.md"),
    }
}

fn applicability_with(
    adapters: Option<Vec<&str>>, languages: Option<Vec<&str>>, artifacts: Option<Vec<&str>>,
    paths: Option<Vec<&str>>,
) -> Applicability {
    Applicability {
        adapters: adapters.map(|v| v.into_iter().map(String::from).collect()),
        languages: languages.map(|v| v.into_iter().map(String::from).collect()),
        artifacts: artifacts.map(|v| v.into_iter().map(String::from).collect()),
        paths: paths.map(|v| v.into_iter().map(String::from).collect()),
    }
}

fn deprecation_meta() -> Deprecated {
    Deprecated {
        reason: "superseded by SEC-001".into(),
        replaced_by: Some("SEC-001".into()),
    }
}

fn make_inputs<'a>(
    target_adapter: &'a str, source_adapters: &'a [String], artifact_paths: &'a [PathBuf],
    languages: &'a [String], include_deprecated: bool, include_unmatched: bool,
) -> ResolveInputs<'a> {
    ResolveInputs {
        project_dir: Path::new("/tmp/filter-tests"),
        rules_root: None,
        target_adapter,
        source_adapters,
        artifact_paths,
        languages,
        include_deprecated,
        include_unmatched,
        include_core: false,
    }
}

fn core_entry(id: &str) -> ResolvedRuleEntry {
    ResolvedRuleEntry {
        rule: make_rule(id, None, None),
        origin: Origin::Core,
        path_root: PathRoot::RulesRoot,
        path: format!("adapters/shared/rules/core/{id}.md"),
    }
}

/// Test 1: a rule with no applicability block survives any inputs.
#[test]
fn no_applicability_passes_through() {
    let entry = make_entry("UNI-001", None, None);
    let inputs = make_inputs("omnia", &[], &[], &[], false, false);
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].rule.id, "UNI-001");
}

/// Test 2: `adapters` matches via the caller's target adapter.
#[test]
fn adapters_match_target() {
    let entry = make_entry(
        "OMNIA-001",
        Some(applicability_with(Some(vec!["omnia"]), None, None, None)),
        None,
    );
    let inputs = make_inputs("omnia", &[], &[], &[], false, false);
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
}

/// Test 3: `adapters` matches via a bound source adapter.
#[test]
fn adapters_match_source() {
    let entry = make_entry(
        "SRC-001",
        Some(applicability_with(Some(vec!["typescript"]), None, None, None)),
        None,
    );
    let sources = vec!["typescript".to_string()];
    let inputs = make_inputs("omnia", &sources, &[], &[], false, false);
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
}

/// Test 4: `adapters` populated but neither target nor any source
/// matches — rule filtered out. Caller input is always present for
/// the adapter dimension, so no `include_unmatched` interaction.
#[test]
fn adapters_no_match_is_filtered() {
    let entry = make_entry(
        "VEC-001",
        Some(applicability_with(Some(vec!["vectis"]), None, None, None)),
        None,
    );
    let inputs = make_inputs("omnia", &[], &[], &[], false, false);
    let out = filter(vec![entry], &inputs);
    assert!(out.is_empty());
}

/// Test 5: `omnia@1.0.0` on the rule side matches a bare `omnia` on
/// the caller side — v1 strips the `@v<major>` suffix.
#[test]
fn adapter_version_suffix_is_stripped() {
    let entry = make_entry(
        "OMNIA-002",
        Some(applicability_with(Some(vec!["omnia@1.0.0"]), None, None, None)),
        None,
    );
    let inputs = make_inputs("omnia", &[], &[], &[], false, false);
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
}

/// Test 6: `languages` populated, caller supplies a matching token.
#[test]
fn languages_match() {
    let entry = make_entry(
        "LANG-001",
        Some(applicability_with(None, Some(vec!["rust"]), None, None)),
        None,
    );
    let languages = vec!["rust".to_string()];
    let inputs = make_inputs("omnia", &[], &[], &languages, false, false);
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
}

/// Test 7: `languages` populated, caller supplies a mismatching
/// token — rule filtered.
#[test]
fn languages_no_match_is_filtered() {
    let entry = make_entry(
        "LANG-002",
        Some(applicability_with(None, Some(vec!["rust"]), None, None)),
        None,
    );
    let languages = vec!["typescript".to_string()];
    let inputs = make_inputs("omnia", &[], &[], &languages, false, false);
    let out = filter(vec![entry], &inputs);
    assert!(out.is_empty());
}

/// Test 8: `languages` populated, caller supplies none, and
/// `include_unmatched` is off — rule filtered.
#[test]
fn languages_absent_excluded() {
    let entry = make_entry(
        "LANG-003",
        Some(applicability_with(None, Some(vec!["rust"]), None, None)),
        None,
    );
    let inputs = make_inputs("omnia", &[], &[], &[], false, false);
    let out = filter(vec![entry], &inputs);
    assert!(out.is_empty());
}

/// Test 9: `languages` populated, caller supplies none, and
/// `include_unmatched` is on — rule passes.
#[test]
fn languages_absent_passes_with_include() {
    let entry = make_entry(
        "LANG-004",
        Some(applicability_with(None, Some(vec!["rust"]), None, None)),
        None,
    );
    let inputs = make_inputs("omnia", &[], &[], &[], false, true);
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
}

/// Test 10: `artifacts` populated — excluded by default because v1
/// has no `--artifact-kind` input.
#[test]
fn artifacts_populated_excluded_by_default() {
    let entry =
        make_entry("ART-001", Some(applicability_with(None, None, Some(vec!["code"]), None)), None);
    let inputs = make_inputs("omnia", &[], &[], &[], false, false);
    let out = filter(vec![entry], &inputs);
    assert!(out.is_empty());
}

/// Test 11: `artifacts` populated + `include_unmatched` — rule
/// passes.
#[test]
fn artifacts_passes_with_include() {
    let entry =
        make_entry("ART-002", Some(applicability_with(None, None, Some(vec!["code"]), None)), None);
    let inputs = make_inputs("omnia", &[], &[], &[], false, true);
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
}

/// Test 12: `paths` matches via the `**` glob across path
/// segments.
#[test]
fn paths_match_double_star_segment() {
    let entry = make_entry(
        "PATH-001",
        Some(applicability_with(None, None, None, Some(vec!["crates/**/src/**/*.rs"]))),
        None,
    );
    let paths = vec![PathBuf::from("crates/billing/src/lib.rs")];
    let inputs = make_inputs("omnia", &[], &paths, &[], false, false);
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
}

/// Test 13: `paths` populated, no caller path matches — rule
/// filtered.
#[test]
fn paths_no_match_is_filtered() {
    let entry = make_entry(
        "PATH-002",
        Some(applicability_with(None, None, None, Some(vec!["crates/**/src/**/*.rs"]))),
        None,
    );
    let paths = vec![PathBuf::from("README.md")];
    let inputs = make_inputs("omnia", &[], &paths, &[], false, false);
    let out = filter(vec![entry], &inputs);
    assert!(out.is_empty());
}

/// Test 14: `paths` populated, caller supplies no paths, and
/// `include_unmatched` is off — rule filtered. The
/// `include_unmatched` branch is exercised by the language test.
#[test]
fn paths_caller_absent_excluded_by_default() {
    let entry = make_entry(
        "PATH-003",
        Some(applicability_with(None, None, None, Some(vec!["**/*.rs"]))),
        None,
    );
    let inputs = make_inputs("omnia", &[], &[], &[], false, false);
    let out = filter(vec![entry.clone()], &inputs);
    assert!(out.is_empty());

    let inputs = make_inputs("omnia", &[], &[], &[], false, true);
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
}

/// Test 15: a single `*` segment does not cross `/`. The same
/// pattern matches `src/lib.rs` but not `src/nested/lib.rs`.
#[test]
fn single_star_no_cross_separator() {
    let entry = make_entry(
        "PATH-004",
        Some(applicability_with(None, None, None, Some(vec!["src/*.rs"]))),
        None,
    );

    let matching = vec![PathBuf::from("src/lib.rs")];
    let inputs = make_inputs("omnia", &[], &matching, &[], false, false);
    assert_eq!(filter(vec![entry.clone()], &inputs).len(), 1);

    let nested = vec![PathBuf::from("src/nested/lib.rs")];
    let inputs = make_inputs("omnia", &[], &nested, &[], false, false);
    assert!(filter(vec![entry], &inputs).is_empty());
}

/// Test 16: AND across dimensions — both `adapters` and
/// `languages` must match. Adapter-only match still filters the
/// rule when languages disagree.
#[test]
fn and_across_dimensions() {
    let entry = make_entry(
        "MULTI-001",
        Some(applicability_with(Some(vec!["omnia"]), Some(vec!["rust"]), None, None)),
        None,
    );

    let rust = vec!["rust".to_string()];
    let inputs = make_inputs("omnia", &[], &[], &rust, false, false);
    assert_eq!(filter(vec![entry.clone()], &inputs).len(), 1);

    let ts = vec!["typescript".to_string()];
    let inputs = make_inputs("omnia", &[], &[], &ts, false, false);
    assert!(filter(vec![entry], &inputs).is_empty());
}

/// Test 17: a deprecated rule is filtered when
/// `include_deprecated` is off.
#[test]
fn deprecated_filtered_by_default() {
    let entry = make_entry("DEP-001", None, Some(deprecation_meta()));
    let inputs = make_inputs("omnia", &[], &[], &[], false, false);
    assert!(filter(vec![entry], &inputs).is_empty());
}

/// Test 18: a deprecated rule survives when `include_deprecated`
/// is on AND its applicability (here `None`) passes.
#[test]
fn deprecated_passes_when_flag_set() {
    let entry = make_entry("DEP-002", None, Some(deprecation_meta()));
    let inputs = make_inputs("omnia", &[], &[], &[], true, false);
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
    assert!(out[0].rule.deprecated.is_some());
}

/// Test 19: deprecation runs before applicability. A deprecated
/// rule whose applicability also wouldn't match is filtered out
/// silently — not via a partial-evaluation bypass.
#[test]
fn deprecation_runs_before_applicability() {
    let entry = make_entry(
        "DEP-003",
        Some(applicability_with(Some(vec!["vectis"]), None, None, None)),
        Some(deprecation_meta()),
    );
    let inputs = make_inputs("omnia", &[], &[], &[], false, false);
    assert!(filter(vec![entry.clone()], &inputs).is_empty());

    // With include_deprecated on, applicability still rejects the
    // rule because the adapter list does not match.
    let inputs = make_inputs("omnia", &[], &[], &[], true, false);
    assert!(filter(vec![entry], &inputs).is_empty());
}

/// Test 20: a malformed glob pattern in a rule must not panic;
/// the rule is excluded because the pattern cannot match anything.
#[test]
fn malformed_glob_pattern_is_safe() {
    let entry = make_entry(
        "PATH-BAD",
        Some(applicability_with(None, None, None, Some(vec!["[broken"]))),
        None,
    );
    let paths = vec![PathBuf::from("src/lib.rs")];
    let inputs = make_inputs("omnia", &[], &paths, &[], false, false);
    let out = filter(vec![entry], &inputs);
    assert!(out.is_empty());
}

/// A [`Origin::Core`] entry is dropped on a default consumer
/// export — `--include-core` is off.
#[test]
fn core_origin_excluded_by_default() {
    let entry = core_entry("CORE-001");
    let inputs = make_inputs("omnia", &[], &[], &[], false, false);
    let out = filter(vec![entry], &inputs);
    assert!(out.is_empty(), "core rules must not appear without --include-core");
}

/// With `--include-core` set, the core entry passes
/// the origin filter and rides through the remaining filters
/// unchanged. Origin metadata is preserved on the surviving entry.
#[test]
fn core_origin_passes_when_flag_set() {
    let entry = core_entry("CORE-001");
    let mut inputs = make_inputs("omnia", &[], &[], &[], false, false);
    inputs.include_core = true;
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].origin, Origin::Core);
    assert_eq!(out[0].rule.id, "CORE-001");
}

/// `--include-core` is orthogonal to other origins: shared / source
/// / target entries flow through whether the flag is on or off.
#[test]
fn core_filter_orthogonal() {
    let shared = make_entry("UNI-001", None, None);
    let core = core_entry("CORE-001");
    let inputs = make_inputs("omnia", &[], &[], &[], false, false);
    let out = filter(vec![shared.clone(), core.clone()], &inputs);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].rule.id, "UNI-001");

    let mut inputs = make_inputs("omnia", &[], &[], &[], false, false);
    inputs.include_core = true;
    let out = filter(vec![shared, core], &inputs);
    assert_eq!(out.len(), 2);
    assert!(out.iter().any(|e| e.rule.id == "UNI-001"));
    assert!(out.iter().any(|e| e.rule.id == "CORE-001"));
}

/// Origin runs before deprecation: a deprecated core rule with
/// `--include-deprecated` set still falls out of the export when
/// `--include-core` is off.
#[test]
fn core_filter_runs_before_deprecation() {
    let entry = ResolvedRuleEntry {
        rule: make_rule("CORE-DEP", None, Some(deprecation_meta())),
        origin: Origin::Core,
        path_root: PathRoot::RulesRoot,
        path: "adapters/shared/rules/core/CORE-DEP.md".to_string(),
    };
    let inputs = make_inputs("omnia", &[], &[], &[], true, false);
    assert!(filter(vec![entry.clone()], &inputs).is_empty());

    let mut inputs = make_inputs("omnia", &[], &[], &[], true, false);
    inputs.include_core = true;
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].origin, Origin::Core);
}
