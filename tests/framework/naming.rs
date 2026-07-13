//! Identifier-brevity predicates over the Rust test surface.

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::support::{Finding, rel, walk_files};

/// A `#[test]` fn identifier exceeds the house-style length cap.
pub const CHECK_TEST_FN_LENGTH: &str = "naming.test-fn-length";

/// Maximum length of a bare `#[test]` fn identifier (must match
/// `docs/standards/testing.md` §"Test naming").
const TEST_FN_CAP: usize = 30;

/// Trees scanned for test fn identifiers.
const SCOPE_PREFIXES: &[&str] = &["src/", "crates/", "harness/", "tests/"];
/// Subtrees excluded from the scan: build output, and this suite's
/// own sources (its fixture strings embed offending identifiers).
const EXCLUDED_PREFIXES: &[&str] = &["tests/framework/"];
const EXCLUDED_SEGMENT: &str = "/target/";

/// A `#[test]` / `#[tokio::test]` attribute, any further attributes,
/// then the fn identifier.
static TEST_FN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#\[(?:tokio::)?test[^\]]*\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)")
        .expect("test fn pattern")
});

/// Run every naming predicate rooted at `root`.
pub fn run(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    check_test_fn_length(root, &mut findings);
    findings
}

/// Every `#[test]` fn identifier must fit the cap; the module path is
/// context and does not count.
fn check_test_fn_length(root: &Path, findings: &mut Vec<Finding>) {
    let mut paths = Vec::new();
    for prefix in SCOPE_PREFIXES {
        let dir = root.join(prefix.trim_end_matches('/'));
        if dir.is_dir() {
            walk_files(&dir, &mut paths);
        }
    }
    paths.retain(|path| path.extension().and_then(|e| e.to_str()) == Some("rs"));

    for path in paths {
        let relative = rel(root, &path);
        if EXCLUDED_PREFIXES.iter().any(|prefix| relative.starts_with(prefix))
            || format!("/{relative}").contains(EXCLUDED_SEGMENT)
        {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for captures in TEST_FN_RE.captures_iter(&content) {
            let name = &captures[1];
            if name.len() <= TEST_FN_CAP {
                continue;
            }
            let offset = captures.get(1).expect("fn ident capture").start();
            let line = content[..offset].matches('\n').count() + 1;
            findings.push(Finding::new(
                CHECK_TEST_FN_LENGTH,
                format!(
                    "{relative}:{line} — test fn '{name}' is {} chars (cap {TEST_FN_CAP})",
                    name.len()
                ),
            ));
        }
    }
}
