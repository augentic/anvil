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
        path: format!("adapters/shared/prose/rules/universal/{id}.md"),
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
        path: format!("adapters/shared/prose/rules/core/{id}.md"),
    }
}

/// One single-entry `filter` case: the entry survives (`pass: true` →
/// one entry out) or is dropped. `applicability_and_path_matrix` drives
/// every single-entry case below; the deprecation-flag and
/// `--include-core` interactions keep their own matrices because they
/// need flags, multi-entry input, or `Origin::Core` entries this table
/// does not model.
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
            make_inputs("demo-target", &sources, &paths, &languages, false, case.include_unmatched);
        let out = filter(vec![entry], &inputs);
        assert_eq!(out.len(), usize::from(case.pass), "case {}", case.name);
    }
}

/// Every single-entry case across all four applicability dimensions —
/// `adapters` (target / source match, no-match, `@v<major>` strip),
/// `languages` (match / mismatch / absent ± include), `artifacts`
/// (excluded by default ± include), and `paths` (`**` cross-segment,
/// single `*` no-cross, caller-absent ± include, malformed glob) — plus
/// the AND-across-dimensions interaction. One row per former input.
#[expect(clippy::too_many_lines, reason = "collapsed filter matrix: one row per former case")]
#[test]
fn applicability_and_path_matrix() {
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
            name: "ORG-001",
            applicability: Some(app(Some(vec!["demo-target"]), None, None, None)),
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: false,
            pass: true,
        },
        // `adapters` matches a bound source adapter.
        Case {
            name: "SRC-001",
            applicability: Some(app(Some(vec!["demo-source"]), None, None, None)),
            sources: &["demo-source"],
            paths: &[],
            languages: &[],
            include_unmatched: false,
            pass: true,
        },
        // `adapters` populated but neither target nor source matches.
        Case {
            name: "OTHER-001",
            applicability: Some(app(Some(vec!["other-target"]), None, None, None)),
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: false,
            pass: false,
        },
        // `demo-target@1.0.0` on the rule matches bare `demo-target` — v1 strips `@v<major>`.
        Case {
            name: "ORG-002",
            applicability: Some(app(Some(vec!["demo-target@1.0.0"]), None, None, None)),
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: false,
            pass: true,
        },
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
        // `paths` populated, caller supplies none, include off → dropped.
        Case {
            name: "PATH-003a",
            applicability: Some(app(None, None, None, Some(vec!["**/*.rs"]))),
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: false,
            pass: false,
        },
        // ...same rule, include on → passes.
        Case {
            name: "PATH-003b",
            applicability: Some(app(None, None, None, Some(vec!["**/*.rs"]))),
            sources: &[],
            paths: &[],
            languages: &[],
            include_unmatched: true,
            pass: true,
        },
        // A single `*` segment does not cross `/`: matches `src/lib.rs`...
        Case {
            name: "PATH-004a",
            applicability: Some(app(None, None, None, Some(vec!["src/*.rs"]))),
            sources: &[],
            paths: &["src/lib.rs"],
            languages: &[],
            include_unmatched: false,
            pass: true,
        },
        // ...but not `src/nested/lib.rs`.
        Case {
            name: "PATH-004b",
            applicability: Some(app(None, None, None, Some(vec!["src/*.rs"]))),
            sources: &[],
            paths: &["src/nested/lib.rs"],
            languages: &[],
            include_unmatched: false,
            pass: false,
        },
        // AND across dimensions: adapter + language both match.
        Case {
            name: "MULTI-001a",
            applicability: Some(app(Some(vec!["demo-target"]), Some(vec!["rust"]), None, None)),
            sources: &[],
            paths: &[],
            languages: &["rust"],
            include_unmatched: false,
            pass: true,
        },
        // ...adapter matches but language disagrees → dropped.
        Case {
            name: "MULTI-001b",
            applicability: Some(app(Some(vec!["demo-target"]), Some(vec!["rust"]), None, None)),
            sources: &[],
            paths: &[],
            languages: &["typescript"],
            include_unmatched: false,
            pass: false,
        },
        // A malformed glob never panics and matches nothing → dropped.
        Case {
            name: "PATH-BAD",
            applicability: Some(app(None, None, None, Some(vec!["[broken"]))),
            sources: &[],
            paths: &["src/lib.rs"],
            languages: &[],
            include_unmatched: false,
            pass: false,
        },
    ]);
}

/// Deprecation-flag interactions (flag absent / present, and the
/// deprecation-before-applicability ordering) — these need the
/// `include_deprecated` flag the single-entry `Case` table omits.
#[test]
fn deprecation_matrix() {
    // A deprecated rule is filtered when `include_deprecated` is off.
    let entry = make_entry("DEP-001", None, Some(deprecation_meta()));
    let inputs = make_inputs("demo-target", &[], &[], &[], false, false);
    assert!(filter(vec![entry], &inputs).is_empty());

    // It survives when the flag is on AND its (here `None`) applicability
    // passes; the deprecation metadata rides through.
    let entry = make_entry("DEP-002", None, Some(deprecation_meta()));
    let inputs = make_inputs("demo-target", &[], &[], &[], true, false);
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
    assert!(out[0].rule.deprecated.is_some());

    // Deprecation runs before applicability: a deprecated rule whose
    // applicability also wouldn't match is filtered out either way (no
    // partial-evaluation bypass).
    let entry = make_entry(
        "DEP-003",
        Some(applicability_with(Some(vec!["other-target"]), None, None, None)),
        Some(deprecation_meta()),
    );
    let inputs = make_inputs("demo-target", &[], &[], &[], false, false);
    assert!(filter(vec![entry.clone()], &inputs).is_empty());
    let inputs = make_inputs("demo-target", &[], &[], &[], true, false);
    assert!(filter(vec![entry], &inputs).is_empty());
}

/// `--include-core` interactions — these need `Origin::Core` entries,
/// the `include_core` flag, and multi-entry input the single-entry
/// `Case` table does not model.
#[test]
fn core_origin_matrix() {
    // A core entry is dropped on a default consumer export (flag off).
    let entry = core_entry("CORE-001");
    let inputs = make_inputs("demo-target", &[], &[], &[], false, false);
    assert!(
        filter(vec![entry], &inputs).is_empty(),
        "core rules must not appear without --include-core"
    );

    // With the flag set, the core entry passes and keeps its origin.
    let entry = core_entry("CORE-001");
    let mut inputs = make_inputs("demo-target", &[], &[], &[], false, false);
    inputs.include_core = true;
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].origin, Origin::Core);
    assert_eq!(out[0].rule.id, "CORE-001");

    // The flag is orthogonal to other origins: a shared entry flows through
    // regardless; toggling the flag only adds the core entry.
    let shared = make_entry("UNI-001", None, None);
    let core = core_entry("CORE-001");
    let inputs = make_inputs("demo-target", &[], &[], &[], false, false);
    let out = filter(vec![shared.clone(), core.clone()], &inputs);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].rule.id, "UNI-001");
    let mut inputs = make_inputs("demo-target", &[], &[], &[], false, false);
    inputs.include_core = true;
    let out = filter(vec![shared, core], &inputs);
    assert_eq!(out.len(), 2);
    assert!(out.iter().any(|e| e.rule.id == "UNI-001"));
    assert!(out.iter().any(|e| e.rule.id == "CORE-001"));

    // Origin runs before deprecation: a deprecated core rule with
    // `--include-deprecated` set still falls out when `--include-core` is off.
    let entry = ResolvedRuleEntry {
        rule: make_rule("CORE-DEP", None, Some(deprecation_meta())),
        origin: Origin::Core,
        path_root: PathRoot::RulesRoot,
        path: "adapters/shared/prose/rules/core/CORE-DEP.md".to_string(),
    };
    let inputs = make_inputs("demo-target", &[], &[], &[], true, false);
    assert!(filter(vec![entry.clone()], &inputs).is_empty());
    let mut inputs = make_inputs("demo-target", &[], &[], &[], true, false);
    inputs.include_core = true;
    let out = filter(vec![entry], &inputs);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].origin, Origin::Core);
}
