//! Enforce the repo-local Rust-quality predicates.
//!
//! Run with `cargo test --test rust_quality`. Hard gates: any
//! `rust.test-fn-name-too-long`, `rust.workflow-clock-read`, or
//! `rust.allow-without-reason` finding fails CI. The archaeology
//! predicate (`rust.archaeology-in-doc-comment`) is advisory only —
//! its markers over-fire on the canonical contract vocabulary the
//! codebase and AGENTS.md use, so it is not gated. The predicates
//! live in [`checks`], dev-only beside this gate.

mod checks {
    //! Repo-local Rust-quality predicates, dev-only.
    //!
    //! These scan the in-tree `engine/` workspace tree (`crates/` + `src/`,
    //! skipping `target/`) and back the
    //! `cargo test --test rust_quality` gate. They are deliberately not a
    //! lint producer: `specify lint framework` runs entirely through
    //! declarative hints and WASI tools, so this code lives with its only
    //! consumer instead of in `specify-standards`.

    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    /// Longest acceptable `#[test]` fn name (see docs/standards/testing.md).
    const MAX_TEST_FN_LEN: usize = 40;

    /// Rule id for sentence-length test fn names.
    pub const RULE_TEST_FN_NAME: &str = "rust.test-fn-name-too-long";
    /// Rule id for archaeology markers in doc comments (advisory only,
    /// not gated).
    pub const RULE_ARCHAEOLOGY: &str = "rust.archaeology-in-doc-comment";
    /// Rule id for `#[allow]` without a `reason`.
    pub const RULE_ALLOW_NO_REASON: &str = "rust.allow-without-reason";
    /// Rule id for wall-clock reads in specify-workflow library code.
    pub const RULE_WORKFLOW_CLOCK: &str = "rust.workflow-clock-read";
    /// Rule id for first-party adapter name literals in runtime dispatch code.
    pub const RULE_ADAPTER_NAME_LITERAL: &str = "rust.adapter-name-literal-in-runtime";

    const BANNED_ADAPTER_NAMES: &[&str] = &["vectis", "omnia", "contracts"];
    const RUNTIME_SCAN_PREFIXES: &[&str] = &["src/", "crates/workflow/src/"];

    /// Forward-slash prefix marking `specify-workflow` library sources. Time
    /// injection (architecture §Time injection) forbids `Timestamp::now()`
    /// here; the clock is read once in `src/runtime/commands/**` handlers and
    /// threaded down.
    const WORKFLOW_SRC_PREFIX: &str = "crates/workflow/src/";

    const ARCHAEOLOGY_MARKERS: &[&str] = &[
        "RFC-",
        "Phase ",
        "formerly ",
        "previously lived",
        "old contract",
        "pre-cutover",
        "folded pair",
    ];

    /// One predicate hit: the rule id plus a human-readable message that
    /// names the offending path and line.
    pub struct Finding {
        pub rule: &'static str,
        pub message: String,
    }

    /// Run every Rust-quality predicate over the workspace rooted at `root`.
    ///
    /// The test-fn-name check covers every `.rs` test file in the tree;
    /// the source-quality checks (archaeology, bare `#[allow]`, workflow
    /// clock reads) are scoped to `crates/` and `src/`.
    pub fn run(root: &Path) -> Vec<Finding> {
        let mut findings = Vec::new();
        walk(root, root, &mut findings);
        findings.sort_by(|a, b| (a.rule, &a.message).cmp(&(b.rule, &b.message)));
        findings
    }

    /// Count `#[test]` / `#[tokio::test]` declarations in crate `src/`
    /// trees, keyed by crate directory name (the root binary's `src/`
    /// keys to `specify`). Integration tests under any `tests/` tree are
    /// excluded by construction — only `src/` files are scoped. Backs the
    /// unit-test ratchet (docs/standards/testing.md).
    #[must_use]
    pub fn count_src_unit_tests(root: &Path) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        count_walk(root, root, &mut counts);
        counts
    }

    fn count_walk(root: &Path, dir: &Path, counts: &mut BTreeMap<String, usize>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                count_walk(root, &path, counts);
                continue;
            }
            if path.extension().is_some_and(|e| e == "rs") {
                count_src_file(root, &path, counts);
            }
        }
    }

    fn count_src_file(root: &Path, path: &Path, counts: &mut BTreeMap<String, usize>) {
        let Some(scope) = src_scope(&relative_display(root, path)) else {
            return;
        };
        let Ok(content) = fs::read_to_string(path) else {
            return;
        };
        let n = content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("#[test]") || trimmed.starts_with("#[tokio::test")
            })
            .count();
        if n > 0 {
            *counts.entry(scope).or_default() += n;
        }
    }

    /// Scope key for a crate `src/` Rust file, or `None` when the file is
    /// not crate `src/` source (integration tests under `tests/`, build
    /// scripts, fixtures). `crates/<dir>/src/**` keys to `<dir>`; the root
    /// binary's `src/**` keys to `specify`.
    fn src_scope(rel: &str) -> Option<String> {
        if let Some(rest) = rel.strip_prefix("crates/") {
            let mut parts = rest.split('/');
            let dir = parts.next()?;
            if parts.next() == Some("src") {
                return Some(dir.to_owned());
            }
            return None;
        }
        if rel.starts_with("src/") {
            return Some("specify".to_owned());
        }
        None
    }

    fn walk(root: &Path, dir: &Path, findings: &mut Vec<Finding>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                walk(root, &path, findings);
                continue;
            }
            if path.extension().is_some_and(|e| e == "rs") {
                check_rust_file(root, &path, findings);
            }
        }
    }

    fn relative_display(root: &Path, path: &Path) -> String {
        path.strip_prefix(root).unwrap_or(path).display().to_string().replace('\\', "/")
    }

    /// True for `specify-workflow` library sources subject to the
    /// time-injection rule. Test modules (`tests.rs` files or anything under
    /// a `tests/` directory) are exempt — they pin the clock with fixtures.
    fn is_workflow_runtime_source(rel: &str) -> bool {
        rel.starts_with(WORKFLOW_SRC_PREFIX)
            && !rel.ends_with("/tests.rs")
            && !rel.contains("/tests/")
    }

    fn is_test_rust_file(rel: &str) -> bool {
        rel.ends_with("tests.rs") || rel.split('/').any(|part| part == "tests")
    }

    fn check_rust_file(root: &Path, path: &Path, findings: &mut Vec<Finding>) {
        let Ok(content) = fs::read_to_string(path) else {
            return;
        };
        let rel = relative_display(root, path);
        let source_quality_scope = rel.starts_with("crates/") || rel.starts_with("src/");
        let workflow_clock_scope = is_workflow_runtime_source(&rel);
        let test_file = is_test_rust_file(&rel);
        let lines: Vec<&str> = content.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let line_no = line_idx + 1;

            if test_file {
                check_test_fn_name(&lines, line_idx, &rel, findings);
            }
            if !source_quality_scope {
                continue;
            }

            // Time injection: library code never reads the wall clock. Skip
            // comment lines so doc comments may still name the API.
            if workflow_clock_scope
                && !trimmed.starts_with("//")
                && trimmed.contains("Timestamp::now()")
            {
                findings.push(Finding {
                rule: RULE_WORKFLOW_CLOCK,
                message: format!(
                    "`Timestamp::now()` at {rel}:{line_no} — specify-workflow must accept an injected `now`; read the clock once in a `src/runtime/commands/**` handler and thread it down (architecture §Time injection)"
                ),
            });
            }

            if trimmed.starts_with("//!") || trimmed.starts_with("///") {
                for marker in ARCHAEOLOGY_MARKERS {
                    if trimmed.contains(marker) {
                        findings.push(Finding {
                        rule: RULE_ARCHAEOLOGY,
                        message: format!(
                            "archaeology marker `{marker}` in doc comment at {rel}:{line_no} — keep ≤3 lines of what-it-does-today; history belongs in DECISIONS.md"
                        ),
                    });
                        break;
                    }
                }
            }

            if trimmed.contains("#[allow(") && !trimmed.contains("reason") {
                findings.push(Finding {
                rule: RULE_ALLOW_NO_REASON,
                message: format!(
                    "#[allow] without reason at {rel}:{line_no} — use #[expect] with reason or promote a module #![allow]"
                ),
            });
            }

            check_adapter_name_literals(&rel, line, line_no, findings);
        }
    }

    fn is_runtime_dispatch_scope(rel: &str) -> bool {
        RUNTIME_SCAN_PREFIXES.iter().any(|prefix| rel.starts_with(prefix))
            && !rel.contains("init/adapter_uri")
    }

    fn is_artifact_class_contracts_reference(line: &str) -> bool {
        line.contains("class_name")
            || line.contains("staged_dir")
            || line.contains("baseline_dir")
            || line.contains("ArtifactClass")
            || line.contains("artefact")
            || line.contains("artifact")
            || line.contains("rel:")
            || line.contains("dest_contracts")
            || (line.contains("name:") && line.contains("\"contracts\""))
            || line.contains("join(\"contracts\")")
    }

    fn check_adapter_name_literals(
        rel: &str, line: &str, line_no: usize, findings: &mut Vec<Finding>,
    ) {
        if !is_runtime_dispatch_scope(rel) {
            return;
        }
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
            return;
        }
        for name in BANNED_ADAPTER_NAMES {
            if *name == "contracts" && is_artifact_class_contracts_reference(line) {
                continue;
            }
            let literal = format!("\"{name}\"");
            if line.contains(&literal) {
                findings.push(Finding {
                    rule: RULE_ADAPTER_NAME_LITERAL,
                    message: format!(
                        "adapter name literal `{name}` at {rel}:{line_no} — runtime dispatch must be manifest-driven; init shorthand belongs only in `init/adapter_uri/`"
                    ),
                });
            }
        }
    }

    fn check_test_fn_name(lines: &[&str], line_idx: usize, rel: &str, findings: &mut Vec<Finding>) {
        let trimmed = lines[line_idx].trim();
        let Some(rest) = trimmed.strip_prefix("fn ").or_else(|| trimmed.strip_prefix("async fn "))
        else {
            return;
        };
        let Some((name, _)) = rest.split_once('(') else {
            return;
        };
        if name.len() <= MAX_TEST_FN_LEN || !preceded_by_test_attr(lines, line_idx) {
            return;
        }
        findings.push(Finding {
        rule: RULE_TEST_FN_NAME,
        message: format!(
            "test fn `{name}` is {} chars; shorten per docs/standards/testing.md (got {rel}:{})",
            name.len(),
            line_idx + 1
        ),
    });
    }

    /// Walk upward over the attribute window above a `fn`, skipping blank lines and
    /// other attributes (`#[ignore]`, `#[case(..)]`, …), and report whether a
    /// `#[test]` / `#[tokio::test]` attribute introduces it.
    fn preceded_by_test_attr(lines: &[&str], fn_idx: usize) -> bool {
        for prev in lines[..fn_idx].iter().rev() {
            let trimmed = prev.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !trimmed.starts_with("#[") {
                return false;
            }
            if trimmed.starts_with("#[test]") || trimmed.starts_with("#[tokio::test") {
                return true;
            }
        }
        false
    }
}

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use checks::{
    RULE_ADAPTER_NAME_LITERAL, RULE_ALLOW_NO_REASON, RULE_TEST_FN_NAME, RULE_WORKFLOW_CLOCK,
};

/// The gated rules and the standards-doc pointer rendered when one fires.
const GATED_RULES: [(&str, &str); 4] = [
    (RULE_TEST_FN_NAME, "test fn names must be <= 40 chars (see docs/standards/testing.md)"),
    (
        // Time injection (architecture §Time injection): `specify-workflow`
        // must accept an injected `now`; the clock is read once in a
        // `src/runtime/commands/**` handler and threaded down.
        RULE_WORKFLOW_CLOCK,
        "specify-workflow library code must not call `Timestamp::now()` (see docs/standards/architecture.md §Time injection)",
    ),
    (
        // `#[allow]` without a `reason` is forbidden (style.md §Lint
        // suppression posture): use `#[expect(.., reason = "…")]` at the
        // smallest scope, or a contract-locked module `#![allow]`.
        RULE_ALLOW_NO_REASON,
        "`#[allow]` must carry a reason or be an `#[expect]` (see docs/standards/style.md)",
    ),
    (
        RULE_ADAPTER_NAME_LITERAL,
        "runtime dispatch must not hardcode first-party adapter names (see rfcs/rfc-51-typed-adapter-abi.md)",
    ),
];

#[test]
fn no_gated_rust_quality_findings() {
    // One repo scan; findings grouped per rule id so a failure stays
    // attributable to the standard it breaches.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let findings = checks::run(&root);

    let mut failures = String::new();
    for (rule, guidance) in GATED_RULES {
        let offenders: Vec<&str> =
            findings.iter().filter(|f| f.rule == rule).map(|f| f.message.as_str()).collect();
        if !offenders.is_empty() {
            writeln!(failures, "[{rule}] {guidance}; offenders: {offenders:#?}")
                .expect("write to String");
        }
    }
    assert!(failures.is_empty(), "rust-quality gates failed:\n{failures}");
}

#[test]
fn flags_long_test_fn_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("crates/workflow/src/foo/tests.rs");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, "#[test]\nfn this_test_function_name_is_way_too_long_for_policy() {}\n")
        .expect("write");

    let findings = checks::run(dir.path());
    assert!(
        findings.iter().any(|f| f.rule == RULE_TEST_FN_NAME),
        "expected long-name finding, got: {:?}",
        findings.iter().map(|f| &f.message).collect::<Vec<_>>()
    );
}

#[test]
fn flags_tokio_test_behind_attributes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("crates/workflow/src/foo/tests.rs");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(
        &path,
        "#[tokio::test]\n#[ignore]\nasync fn this_async_test_function_name_is_clearly_too_long() {}\n",
    )
    .expect("write");

    let findings = checks::run(dir.path());
    assert!(
        findings.iter().any(|f| f.rule == RULE_TEST_FN_NAME),
        "tokio::test behind an intervening attribute must still be flagged"
    );
}

#[test]
fn ignores_long_non_test_fn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("crates/workflow/src/foo/tests.rs");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, "fn this_helper_function_name_is_long_but_not_a_test_case() {}\n")
        .expect("write");

    let findings = checks::run(dir.path());
    assert!(
        !findings.iter().any(|f| f.rule == RULE_TEST_FN_NAME),
        "non-test fns must not be flagged"
    );
}

#[test]
fn flags_bare_allow_and_clock_read() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("crates/workflow/src/foo.rs");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(
        &path,
        "#[allow(dead_code)]\nfn now() -> jiff::Timestamp { jiff::Timestamp::now() }\n",
    )
    .expect("write");

    let findings = checks::run(dir.path());
    assert!(findings.iter().any(|f| f.rule == RULE_ALLOW_NO_REASON), "bare allow must flag");
    assert!(findings.iter().any(|f| f.rule == RULE_WORKFLOW_CLOCK), "clock read must flag");
}

/// Per-crate src unit-test budget file, relative to the crate root.
const BUDGET_FILE: &str = "tests/rust_quality_budget.toml";

/// Read the ratchet budget. Deliberately a minimal `key = <int>` reader
/// (skips blank lines, `#` comments, and the `[crate]` header) so this
/// dev-gate stays dependency-free rather than pulling in a TOML parser.
fn load_budget(root: &Path) -> BTreeMap<String, usize> {
    let path = root.join(BUDGET_FILE);
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut budget = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"').to_owned();
        let count = value
            .trim()
            .parse()
            .unwrap_or_else(|e| panic!("budget for `{key}` is not a usize: {e}"));
        budget.insert(key, count);
    }
    budget
}

/// Strict ratchet on src unit tests: the committed budget must equal the
/// live count per crate. Above budget means a new unit test was added —
/// exercise the behavior through the public surface in `tests/` instead
/// (docs/standards/testing.md); below budget means a reduction landed and
/// the number must be lowered to lock it in. Either way the budget edit is
/// the reviewable signal that catches a unit test being added.
#[test]
fn unit_test_budget_holds() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let counts = checks::count_src_unit_tests(&root);
    let budget = load_budget(&root);

    let scopes: BTreeSet<&String> = counts.keys().chain(budget.keys()).collect();
    let mut failures = String::new();
    for scope in scopes {
        let current = counts.get(scope).copied().unwrap_or(0);
        let allowed = budget.get(scope).copied().unwrap_or(0);
        if current > allowed {
            writeln!(
                failures,
                "[{scope}] {current} src unit tests > budget {allowed}: do not add src unit tests — exercise the behavior through the public surface in tests/, or justify and raise the budget in review (docs/standards/testing.md)"
            )
            .expect("write to String");
        } else if current < allowed {
            writeln!(
                failures,
                "[{scope}] {current} src unit tests < budget {allowed}: ratchet down — set `{scope} = {current}` in {BUDGET_FILE}"
            )
            .expect("write to String");
        }
    }
    assert!(failures.is_empty(), "unit-test ratchet failed:\n{failures}");
}

#[test]
fn counts_src_unit_tests_by_scope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let crate_src = root.join("crates/demo/src/foo.rs");
    fs::create_dir_all(crate_src.parent().expect("parent")).expect("mkdir");
    fs::write(&crate_src, "#[test]\nfn a() {}\n#[tokio::test]\nasync fn b() {}\n").expect("write");
    // Integration tests under tests/ must never be counted.
    let crate_it = root.join("crates/demo/tests/it.rs");
    fs::create_dir_all(crate_it.parent().expect("parent")).expect("mkdir");
    fs::write(&crate_it, "#[test]\nfn c() {}\n").expect("write");
    // Root-binary src/ keys to `specify`.
    let bin_src = root.join("src/main.rs");
    fs::create_dir_all(bin_src.parent().expect("parent")).expect("mkdir");
    fs::write(&bin_src, "#[test]\nfn d() {}\n").expect("write");

    let counts = checks::count_src_unit_tests(root);
    assert_eq!(counts.get("demo").copied(), Some(2));
    assert_eq!(counts.get("specify").copied(), Some(1));
    assert_eq!(counts.len(), 2, "integration tests under tests/ are excluded");
}
