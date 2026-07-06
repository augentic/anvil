use super::*;
use crate::rules::{HintKind, LintMode, Severity};

const HARDCODED_CONFIGURATION: &str =
    include_str!("../../../tests/fixtures/rules/hardcoded-configuration.md");
const DOCUMENTATION_VERBATIM_PRESERVATION: &str =
    include_str!("../../../tests/fixtures/rules/documentation-verbatim-preservation.md");

// `parse_rule` and the `parse_rule_file` I/O wrapper are crate-public, but the
// CLI e2e (`tests/rules.rs`) exercises export end-to-end, not the
// per-input parse matrix. The thirteen `parse_rule*` cases collapse into one
// success matrix and one error matrix, each preserving every former input; the
// private `snake_to_kebab_keys` helper test stays on its own.

#[expect(
    clippy::too_many_lines,
    reason = "collapsed parse-success matrix: one block per former test"
)]
#[test]
fn parse_success_matrix() {
    // Real shared UNI-014 fixture: typed frontmatter fields land and the body
    // preserves the blank line before '## Rule' plus the documented headings
    // (only the single newline after the delimiter is stripped).
    let rule = parse_rule(HARDCODED_CONFIGURATION).expect("parses");
    assert_eq!(rule.id, "UNI-014");
    assert_eq!(rule.title, "Hardcoded Configuration Values");
    assert_eq!(rule.severity, Severity::Important);
    assert!(
        rule.body.starts_with("\n## Rule\n"),
        "body must preserve the blank line before '## Rule', got: {:?}",
        &rule.body[..rule.body.len().min(40)]
    );
    assert!(rule.body.contains("\n## Look For\n"));
    assert!(rule.body.contains("\n## Spec Guidance\n"));

    // Real source-axis SRC-001 fixture: optional applicability / references
    // blocks parse and multi-paragraph body prose survives.
    let rule = parse_rule(DOCUMENTATION_VERBATIM_PRESERVATION).expect("parses");
    assert_eq!(rule.id, "SRC-001");
    assert_eq!(rule.severity, Severity::Important);
    let applicability = rule.applicability.expect("applicability present");
    assert_eq!(applicability.adapters.as_deref(), Some(&["documentation".to_string()][..]));
    let references = rule.references.expect("references present");
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].label, "documentation.extract determinism rules");
    assert_eq!(
        references[0].path.as_deref(),
        Some("adapters/sources/documentation/prose/prompts/extract.md"),
    );
    assert!(rule.body.contains("\n## Rule\n"));

    // snake_case authoring keys lift to the kebab-case wire shape carried by
    // `Rule`; no snake_case key leaks on re-serialize.
    let content = r"---
id: UNI-014
title: Sample
severity: important
trigger: A short trigger sentence covering the rule context.
lint_mode: hybrid
rule_hints:
  - kind: regex
    value: 'https?://'
    description: Literal URL.
deprecated:
  reason: superseded by SEC-001
  replaced_by: SEC-001
---
## Rule

Body.
";
    let rule = parse_rule(content).expect("parses");
    assert_eq!(rule.lint_mode, Some(LintMode::Hybrid));
    let hints = rule.rule_hints.as_ref().expect("hints present");
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].kind, HintKind::Regex);
    assert_eq!(hints[0].value, "https?://");
    let deprecated = rule.deprecated.as_ref().expect("deprecated present");
    assert_eq!(deprecated.reason, "superseded by SEC-001");
    assert_eq!(deprecated.replaced_by.as_deref(), Some("SEC-001"));
    let json = serde_json::to_string(&rule).expect("serialize");
    assert!(json.contains("\"lint-mode\""));
    assert!(json.contains("\"rule-hints\""));
    assert!(json.contains("\"replaced-by\""));
    assert!(!json.contains("\"lint_mode\""));
    assert!(!json.contains("\"rule_hints\""));
    assert!(!json.contains("\"replaced_by\""));

    // Body bytes are preserved verbatim, including fenced blocks with inner
    // '---' separators and inner blank lines.
    let content = "---\n\
id: UNI-014\n\
title: Body fidelity\n\
severity: optional\n\
trigger: Body preservation regression covering fenced ``` blocks with inner separators.\n\
---\n\
## Rule\n\
\n\
Some prose.\n\
\n\
```yaml\n\
key: value\n\
---\n\
other: doc\n\
```\n\
\n\
Trailing line.\n";
    let rule = parse_rule(content).expect("parses");
    let expected_body = "## Rule\n\
\n\
Some prose.\n\
\n\
```yaml\n\
key: value\n\
---\n\
other: doc\n\
```\n\
\n\
Trailing line.\n";
    assert_eq!(rule.body, expected_body);

    // CRLF line endings round-trip; the single newline after the closing
    // `---\r\n` is stripped, every other byte preserved.
    let content = "---\r\nid: UNI-014\r\ntitle: CRLF\r\nseverity: optional\r\ntrigger: CRLF body fidelity covering Windows-style line endings end-to-end.\r\n---\r\n## Rule\r\n\r\nLine one.\r\nLine two.\r\n";
    let rule = parse_rule(content).expect("parses");
    assert_eq!(rule.body, "## Rule\r\n\r\nLine one.\r\nLine two.\r\n");

    // A hint with an unparseable regex value still parses — regex compilation
    // belongs to the evaluator, not the parser.
    let content = r"---
id: UNI-014
title: Broken regex tolerated
severity: optional
trigger: Parser must not compile regex hint values; that belongs to hint execution.
rule_hints:
  - kind: regex
    value: '[invalid regex)('
---
## Rule
";
    let rule = parse_rule(content).expect("parses despite broken regex value");
    let hints = rule.rule_hints.expect("hints present");
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].kind, HintKind::Regex);
    assert_eq!(hints[0].value, "[invalid regex)(");

    // Reserved hint kinds shape-validate without execution semantics.
    let content = r"---
id: UNI-014
title: Reserved hint kinds
severity: optional
trigger: Reserved hint kind hint kinds must shape-validate without execution semantics.
rule_hints:
  - kind: set-coverage
    value: 'rule.id'
  - kind: constant-eq
    value: 'UNI'
---
## Rule
";
    let rule = parse_rule(content).expect("parses");
    let hints = rule.rule_hints.expect("hints present");
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0].kind, HintKind::SetCoverage);
    assert_eq!(hints[1].kind, HintKind::ConstantEq);

    // Framework-side applicability.artifacts tokens compose with consumer-side
    // tokens in the closed schema enum.
    let content = r"---
id: UNI-014
title: Framework artifact tokens
severity: optional
trigger: Framework-side artifact tokens must compose with consumer-side tokens.
applicability:
  artifacts:
    - skill
    - adapter
---
## Rule
";
    let rule = parse_rule(content).expect("parses with framework artifact tokens");
    let applicability = rule.applicability.expect("applicability present");
    assert_eq!(
        applicability.artifacts.as_deref(),
        Some(&["skill".to_string(), "adapter".to_string()][..]),
    );
}

#[test]
fn parse_error_matrix() {
    // `parse_rule_file` reads a path and delegates to `parse_rule`; a missing
    // file maps to `ParseError::Io` (thin I/O wrapper, no sibling checkout).
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("UNI-001.md");
    fs::write(
        &path,
        "---\nid: UNI-001\ntitle: File wrapper\nseverity: important\ntrigger: Synthetic parse_rule_file fixture trigger sentence long enough for schema.\n---\n\n## Rule\n\nBody.\n",
    )
    .expect("write rule file");
    let rule = parse_rule_file(&path).expect("parse_rule_file parses a real file");
    assert_eq!(rule.id, "UNI-001");
    assert!(rule.body.contains("## Rule"));
    let missing = parse_rule_file(&dir.path().join("absent.md"));
    assert!(matches!(missing, Err(ParseError::Io(_))), "missing file maps to Io error");

    // Missing leading `---` line.
    let err = parse_rule("## Rule\nNo frontmatter at all.\n").expect_err("must error");
    assert!(matches!(err, ParseError::MissingOpeningDelimiter), "got: {err:?}");

    // Opening delimiter without a matching close.
    let err = parse_rule("---\nid: UNI-014\ntitle: dangling\nseverity: optional\ntrigger: t.\n")
        .expect_err("must error");
    assert!(matches!(err, ParseError::MissingClosingDelimiter), "got: {err:?}");

    // Unparseable YAML in the frontmatter.
    let err =
        parse_rule("---\nid: UNI-014\n  bad: : indent\n---\n## Rule\n").expect_err("must error");
    assert!(matches!(err, ParseError::Yaml(_)), "got: {err:?}");

    // Schema-mandated `id` field missing.
    let err = parse_rule("---\ntitle: No Id\nseverity: important\ntrigger: t.\n---\n## Rule\n")
        .expect_err("must error");
    assert!(matches!(err, ParseError::Schema(_)), "got: {err:?}");
    if let ParseError::Schema(detail) = err {
        assert!(detail.contains("id"), "expected 'id' in schema detail, got: {detail}");
    }
}

/// Helper: snake-to-kebab lift only rewrites keys, never
/// values. Adapter names like `demo-source` must survive
/// untouched.
#[test]
fn snake_to_kebab_only_touches_keys() {
    let input = serde_json::json!({
        "lint_mode": "hybrid",
        "applicability": {
            "adapters": ["demo-source", "demo-docs"],
        },
        "rule_hints": [
            {"kind": "regex", "value": "snake_in_value_stays"},
        ],
    });
    let lifted = snake_to_kebab_keys(input);
    assert_eq!(lifted["lint-mode"], "hybrid");
    assert_eq!(lifted["applicability"]["adapters"][0], "demo-source");
    assert_eq!(lifted["rule-hints"][0]["value"], "snake_in_value_stays");
}
