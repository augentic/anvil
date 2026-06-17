//! Enforce the repo-local Rust-quality predicates.
//!
//! Run with `cargo test --test rust_quality`. Hard gates: any
//! `rust.test-fn-name-too-long`, `rust.workflow-clock-read`, or
//! `rust.allow-without-reason` finding fails CI. The archaeology
//! predicate (`rust.archaeology-in-doc-comment`) is advisory only —
//! its markers over-fire on the canonical contract vocabulary the
//! codebase and AGENTS.md use, so it is not gated. The predicates
//! live in [`checks`], dev-only beside this gate.

mod checks;

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use checks::{RULE_ALLOW_NO_REASON, RULE_TEST_FN_NAME, RULE_WORKFLOW_CLOCK};

/// The gated rules and the standards-doc pointer rendered when one fires.
const GATED_RULES: [(&str, &str); 3] = [
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
