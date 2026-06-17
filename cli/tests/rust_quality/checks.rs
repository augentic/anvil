//! Repo-local Rust-quality predicates, dev-only.
//!
//! These scan the specify-cli workspace tree (`crates/` + `src/`,
//! skipping `target/`) and back the
//! `cargo test --test rust_quality` gate. They are deliberately not a
//! lint producer: `specify lint framework` runs entirely through
//! declarative hints and WASI tools, so this code lives with its only
//! consumer instead of in `specify-standards`.

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
    rel.starts_with(WORKFLOW_SRC_PREFIX) && !rel.ends_with("/tests.rs") && !rel.contains("/tests/")
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
