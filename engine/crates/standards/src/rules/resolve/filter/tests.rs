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

/// One single-entry `filter` case: the entry survives (`pass: true` →
/// one entry out) or is dropped. The three `*_dimension`/`*_matrix`
/// tests below group these by applicability dimension; the
/// AND-across-dimensions, deprecation-ordering, and `--include-core`
/// interaction cases keep their own named tests further down.
struct Case {
    name: &'static str,
    applicability: Option<Applicability>,
    sources: &'static [&'static str],
    paths: &'static [&'static str],
    languages: &'static [&'static str],
    include_unmatched: bool,
    pass: bool,
}

fn check_filter_cases(cases: Vec<Case>) {
    for case in cases {
        let entry = make_entry(case.name, case.applicability, None);
        let sources: Vec<String> = case.sources.iter().map(|s| (*s).to_string()).collect();
        let paths: Vec<PathBuf> = case.paths.iter().map(PathBuf::from).collect();
        let languages: Vec<String> = case.languages.iter().map(|s| (*s).to_string()).collect();
        let inputs =
            make_inputs("omnia", &sources, &paths, &languages, false, case.include_unmatched);
        let out = filter(vec![entry], &inputs);
        assert_eq!(out.len(), usize::from(case.pass), "case {}", case.name);
    }
}

/// `adapters` matches the caller's target or a bound source adapter; a
/// populated list with no match is filtered; `@v<major>` is stripped.
#[test]
fn adapter_dimension_matrix() {
    let app = applicability_with;
    check_filter_cases(vec![
        // No applicability block survives any inputs.
        Case {
            name: "UNI-001",
            applicability: None,
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: false,
            pass: true,
        },
        // `adapters` matches the caller's target adapter.
        Case {
            name: "OMNIA-001",
            applicability: Some(app(Some(vec!["omnia"]), None, None, None)),
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: false,
            pass: true,
        },
        // `adapters` matches a bound source adapter.
        Case {
            name: "SRC-001",
            applicability: Some(app(Some(vec!["typescript"]), None, None, None)),
            sources: &["typescript"],
            paths: &[],
            languages: &[],
            include_unmatched: false,
            pass: true,
        },
        // `adapters` populated but neither target nor source matches.
        Case {
            name: "VEC-001",
            applicability: Some(app(Some(vec!["vectis"]), None, None, None)),
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: false,
            pass: false,
        },
        // `omnia@1.0.0` on the rule matches bare `omnia` — v1 strips `@v<major>`.
        Case {
            name: "OMNIA-002",
            applicability: Some(app(Some(vec!["omnia@1.0.0"]), None, None, None)),
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: false,
            pass: true,
        },
    ]);
}

/// `languages` is matched against caller tokens; excluded when the
/// caller supplies none unless `include_unmatched` is set.
#[test]
fn language_dimension_matrix() {
    let app = applicability_with;
    check_filter_cases(vec![
        // `languages` matches a caller token.
        Case {
            name: "LANG-001",
            applicability: Some(app(None, Some(vec!["rust"]), None, None)),
            sources: &[],
            paths: &[],
            languages: &["rust"],
            include_unmatched: false,
            pass: true,
        },
        // `languages` mismatches a caller token.
        Case {
            name: "LANG-002",
            applicability: Some(app(None, Some(vec!["rust"]), None, None)),
            sources: &[],
            paths: &[],
            languages: &["typescript"],
            include_unmatched: false,
            pass: false,
        },
        // `languages` populated, caller supplies none, include off.
        Case {
            name: "LANG-003",
            applicability: Some(app(None, Some(vec!["rust"]), None, None)),
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: false,
            pass: false,
        },
        // `languages` populated, caller supplies none, include on.
        Case {
            name: "LANG-004",
            applicability: Some(app(None, Some(vec!["rust"]), None, None)),
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: true,
            pass: true,
        },
    ]);
}

/// `artifacts` and `paths` dimensions: `artifacts` are excluded by
/// default (no `--artifact-kind` input); path globs match by segment.
#[test]
fn artifact_and_path_matrix() {
    let app = applicability_with;
    check_filter_cases(vec![
        // `artifacts` populated — excluded by default (no `--artifact-kind`).
        Case {
            name: "ART-001",
            applicability: Some(app(None, None, Some(vec!["code"]), None)),
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: false,
            pass: false,
        },
        // `artifacts` populated + include — passes.
        Case {
            name: "ART-002",
            applicability: Some(app(None, None, Some(vec!["code"]), None)),
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: true,
            pass: true,
        },
        // `paths` matches via `**` across segments.
        Case {
            name: "PATH-001",
            applicability: Some(app(None, None, None, Some(vec!["crates/**/src/**/*.rs"]))),
            sources: &[],
            paths: &["crates/billing/src/lib.rs"],
            languages: &[],
            include_unmatched: false,
            pass: true,
        },
        // `paths` populated, no caller path matches.
        Case {
            name: "PATH-002",
            applicability: Some(app(None, None, None, Some(vec!["crates/**/src/**/*.rs"]))),
            sources: &[],
            paths: &["README.md"],
            languages: &[],
            include_unmatched: false,
            pass: false,
        },
    ]);
}

/// `paths` populated but the caller supplies no paths: filtered when
/// `include_unmatched` is off, passed when it is on.
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

/// A single `*` segment does not cross `/`. The same
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

/// AND across dimensions — both `adapters` and
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

/// A deprecated rule is filtered when `include_deprecated` is off.
#[test]
fn deprecated_filtered_by_default() {
    let entry = make_entry("DEP-001", None, Some(deprecation_meta()));
    let inputs = make_inputs("omnia", &[], &[], &[], false, false);
    assert!(filter(vec![entry], &inputs).is_empty());
}

/// A deprecated rule survives when `include_deprecated` is on AND its
/// applicability (here `None`) passes.
#[test]
fn deprecated_passes_when_flag_set() {
    let entry = make_entry("DEP-002", None, Some(deprecation_meta()));
    let inputs = make_inputs("omnia", &[], &[], &[], true, false);
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
    assert!(out[0].rule.deprecated.is_some());
}

/// Deprecation runs before applicability. A deprecated
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

/// A malformed glob pattern in a rule must not panic; the rule is
/// excluded because the pattern cannot match anything.
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
