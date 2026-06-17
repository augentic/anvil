use super::*;
use crate::rules::{HintKind, LintMode, Severity};

const HARDCODED_CONFIGURATION: &str =
    include_str!("../../../tests/fixtures/rules/hardcoded-configuration.md");
const DOCUMENTATION_VERBATIM_PRESERVATION: &str =
    include_str!("../../../tests/fixtures/rules/documentation-verbatim-preservation.md");

/// Real shared `UNI-014` rule (rule file shape worked
/// example). Frontmatter fields land in the typed shape and the
/// body carries the policy headings verbatim. The fixture has a
/// blank line between the closing `---` and `## Rule`, so the
/// body opens with that preserved blank — only the single
/// newline immediately after the delimiter is stripped, per
/// contract-fidelity rules.
#[test]
fn parses_hardcoded_configuration_fixture() {
    let rule = parse_rule(HARDCODED_CONFIGURATION).expect("parses");
    assert_eq!(rule.id, "UNI-014");
    assert_eq!(rule.title, "Hardcoded Configuration Values");
    assert_eq!(rule.severity, Severity::Important);
    assert!(
        rule.body.starts_with("\n## Rule\n"),
        "body must preserve the blank line before '## Rule', got: {:?}",
        &rule.body[..rule.body.len().min(40)]
    );
    // Rule file shape: body carries the documented
    // section headings verbatim.
    assert!(rule.body.contains("\n## Look For\n"));
    assert!(rule.body.contains("\n## Spec Guidance\n"));
}

/// Real source-axis `SRC-001` rule. Exercises optional
/// frontmatter blocks (`applicability.adapters`, `references`)
/// and confirms the body preserves multi-paragraph prose.
#[test]
fn parses_verbatim_fixture() {
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
        Some("adapters/sources/documentation/briefs/extract.md"),
    );
    assert!(rule.body.contains("\n## Rule\n"));
}

/// [`parse_rule_file`] reads a path and delegates to [`parse_rule`].
/// Synthetic tempfile coverage of the thin I/O wrapper (no sibling
/// checkout required).
#[test]
fn parse_rule_file_reads_and_delegates() {
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
}

/// `snake_case` authoring keys lift to the kebab-case wire shape
/// carried by [`Rule`]. Covers every documented rename:
/// `lint_mode`, `rule_hints`, and the nested
/// `deprecated.replaced_by`.
#[test]
fn snake_case_keys_lift_to_kebab_case() {
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

    // Re-serialize and confirm the kebab-case wire form is
    // intact (no snake_case key leaks).
    let json = serde_json::to_string(&rule).expect("serialize");
    assert!(json.contains("\"lint-mode\""));
    assert!(json.contains("\"rule-hints\""));
    assert!(json.contains("\"replaced-by\""));
    assert!(!json.contains("\"lint_mode\""));
    assert!(!json.contains("\"rule_hints\""));
    assert!(!json.contains("\"replaced_by\""));
}

/// Body bytes are preserved verbatim, including code fences
/// containing `---` separators and inner blank lines. Reviewers
/// consume the body as policy text, so any byte-level edit here
/// is a correctness break.
#[test]
fn body_preserved_with_fences() {
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
}

/// CRLF line endings on the closing delimiter and inside the
/// body round-trip into the typed rule. The leading newline
/// after the closing `---\r\n` is stripped, every other byte
/// is preserved.
#[test]
fn body_preserves_crlf_line_endings() {
    let content = "---\r\nid: UNI-014\r\ntitle: CRLF\r\nseverity: optional\r\ntrigger: CRLF body fidelity covering Windows-style line endings end-to-end.\r\n---\r\n## Rule\r\n\r\nLine one.\r\nLine two.\r\n";
    let rule = parse_rule(content).expect("parses");
    assert_eq!(rule.body, "## Rule\r\n\r\nLine one.\r\nLine two.\r\n");
}

/// Missing leading `---` line surfaces as
/// [`ParseError::MissingOpeningDelimiter`].
#[test]
fn missing_opening_delimiter_errors() {
    let content = "## Rule\nNo frontmatter at all.\n";
    let err = parse_rule(content).expect_err("must error");
    assert!(matches!(err, ParseError::MissingOpeningDelimiter), "got: {err:?}");
}

/// Opening delimiter without a matching close surfaces as
/// [`ParseError::MissingClosingDelimiter`].
#[test]
fn missing_closing_delimiter_errors() {
    let content = "---\nid: UNI-014\ntitle: dangling\nseverity: optional\ntrigger: t.\n";
    let err = parse_rule(content).expect_err("must error");
    assert!(matches!(err, ParseError::MissingClosingDelimiter), "got: {err:?}");
}

/// Unparseable YAML in the frontmatter surfaces as
/// [`ParseError::Yaml`].
#[test]
fn invalid_yaml_errors() {
    let content = "---\nid: UNI-014\n  bad: : indent\n---\n## Rule\n";
    let err = parse_rule(content).expect_err("must error");
    assert!(matches!(err, ParseError::Yaml(_)), "got: {err:?}");
}

/// Schema-mandated `id` field missing surfaces as
/// [`ParseError::Schema`].
#[test]
fn schema_violation_errors_when_id_missing() {
    let content = "---\ntitle: No Id\nseverity: important\ntrigger: t.\n---\n## Rule\n";
    let err = parse_rule(content).expect_err("must error");
    assert!(matches!(err, ParseError::Schema(_)), "got: {err:?}");
    if let ParseError::Schema(detail) = err {
        assert!(detail.contains("id"), "expected 'id' in schema detail, got: {detail}");
    }
}

/// Deterministic-hints extensibility: "the runtime
/// resolver MUST NOT compile a regex it never executes". A hint
/// with an unparseable regex pattern MUST still parse — regex
/// compilation belongs to the hint evaluator.
#[test]
fn invalid_regex_hint_value_still_parses() {
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
}

/// Reserved hint kind hint kinds (`set-coverage`, etc.) round-trip
/// successfully even though no execution semantics exist for
/// them yet.
#[test]
fn reserved_hint_kinds_round_trip() {
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
}

/// Framework-side `applicability.artifacts` tokens
/// (`skill`, `adapter`, `brief`, `reference`, `codex`, `rfc`,
/// `doc`) compose with the consumer-side tokens in the closed
/// schema enum. A rule whose applicability mixes both sides
/// must shape-validate cleanly.
#[test]
fn artifacts_accepts_framework_tokens() {
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

/// Helper: snake-to-kebab lift only rewrites keys, never
/// values. Adapter names like `typescript` must survive
/// untouched.
#[test]
fn snake_to_kebab_only_touches_keys() {
    let input = serde_json::json!({
        "lint_mode": "hybrid",
        "applicability": {
            "adapters": ["typescript", "documentation"],
        },
        "rule_hints": [
            {"kind": "regex", "value": "snake_in_value_stays"},
        ],
    });
    let lifted = snake_to_kebab_keys(input);
    assert_eq!(lifted["lint-mode"], "hybrid");
    assert_eq!(lifted["applicability"]["adapters"][0], "typescript");
    assert_eq!(lifted["rule-hints"][0]["value"], "snake_in_value_stays");
}
